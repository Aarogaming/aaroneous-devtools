//! A fully merged, validated configuration, and the manager that keeps the last good one.

use crate::config::{ActionDef, Config, Mode};
use crate::error::LoadError;
use crate::layers::{Layer, LayerKind, merge};
use crate::permissions::{Actor, Decision, FLOOR_DENIED, Rule, Verdict, decide};
use std::collections::BTreeMap;
use std::sync::Arc;
use toml::Value;

/// The embedded built-in defaults, the only place a default is written down.
pub const DEFAULTS_TOML: &str = include_str!("defaults.toml");

/// One setting's final value and the layer that set it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveValue {
    pub path: String,
    pub value: String,
    pub layer: LayerKind,
}

#[derive(Debug, Clone)]
pub struct Loaded {
    pub config: Config,
    /// `config.permissions.mode` unless `auto` was requested without being enabled in the user layer.
    pub effective_mode: Mode,
    /// Things worth showing a person that did not stop the load (for example `auto` was not enabled).
    pub warnings: Vec<String>,
    rules: Vec<Rule>,
    provenance: BTreeMap<String, LayerKind>,
    table: toml::Table,
}

fn mode_of(table: &toml::Table) -> Option<Mode> {
    table
        .get("permissions")?
        .get("mode")?
        .clone()
        .try_into()
        .ok()
}

fn from_shared_layer(provenance: &BTreeMap<String, LayerKind>, path: &str) -> bool {
    matches!(
        provenance.get(path),
        Some(LayerKind::Project | LayerKind::Local)
    )
}

fn flatten(value: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Table(table) => {
            for (key, inner) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(inner, &path, out);
            }
        }
        other => out.push((prefix.to_owned(), other.to_string())),
    }
}

impl Loaded {
    /// The built-in defaults alone. They are tested to load, so this cannot fail in a released build.
    pub fn defaults() -> Result<Self, LoadError> {
        Self::load(&[])
    }

    /// Merge the embedded defaults with the caller's layers (user, project, local; strictly in that order).
    pub fn load(layers: &[Layer]) -> Result<Self, LoadError> {
        let mut previous = LayerKind::Defaults;
        let mut sources: Vec<(LayerKind, &str)> = vec![(LayerKind::Defaults, DEFAULTS_TOML)];
        for layer in layers {
            if layer.kind <= previous {
                return Err(LoadError::LayerOrder {
                    message: format!(
                        "`{}` came after `{}`; give user, project, local in that order, each at most once, and never defaults",
                        layer.kind.name(),
                        previous.name()
                    ),
                });
            }
            previous = layer.kind;
            if let Some(text) = &layer.text {
                sources.push((layer.kind, text));
            }
        }
        let merged = merge(&sources)?;
        let config: Config = toml::Value::Table(merged.table.clone())
            .try_into()
            .map_err(|error: toml::de::Error| LoadError::Invalid {
                message: error.message().to_owned(),
            })?;
        config.validate()?;

        let mut rules = Vec::with_capacity(merged.rules.len());
        for (layer, text) in &merged.rules {
            let rule = Rule::parse(text, *layer).map_err(|message| LoadError::BadRule {
                layer: layer.name().to_owned(),
                rule: text.clone(),
                message,
            })?;
            if let Some(name) = rule.exact_action()
                && !config.actions.contains_key(name)
                && !FLOOR_DENIED.contains(&name)
            {
                return Err(LoadError::BadRule {
                    layer: layer.name().to_owned(),
                    rule: text.clone(),
                    message: format!(
                        "`{name}` is not a registered action; add it under [actions.{name}]"
                    ),
                });
            }
            rules.push(rule);
        }

        let mut warnings = Vec::new();
        let trust_shared = config.permissions.accept_project_allows
            && merged.provenance.get("permissions.accept_project_allows") == Some(&LayerKind::User);
        if config.permissions.accept_project_allows && !trust_shared {
            warnings.push(
                "`permissions.accept_project_allows` is ignored: it must be set in the user settings".to_owned(),
            );
        }
        let mut effective_mode = config.permissions.mode;
        if !trust_shared {
            // A cloned repository's shared settings must not loosen the owner's machine.
            let defaults_only = merge(&sources[..1])?;
            let defaults_config: Config = Value::Table(defaults_only.table).try_into().map_err(
                |error: toml::de::Error| LoadError::Invalid {
                    message: error.message().to_owned(),
                },
            )?;
            for (name, built_in) in &defaults_config.actions {
                let changed: Option<&ActionDef> =
                    config.actions.get(name).filter(|now| *now != built_in);
                if changed.is_some()
                    && (from_shared_layer(&merged.provenance, &format!("actions.{name}.kind"))
                        || from_shared_layer(&merged.provenance, &format!("actions.{name}.risk")))
                {
                    return Err(LoadError::Invalid {
                        message: format!(
                            "project or local settings may not reclassify the built-in action `{name}`; set it in the user settings or turn on `permissions.accept_project_allows` there"
                        ),
                    });
                }
            }
            if from_shared_layer(&merged.provenance, "permissions.mode") {
                let trusted: Vec<(LayerKind, &str)> = sources
                    .iter()
                    .copied()
                    .filter(|(kind, _)| *kind <= LayerKind::User)
                    .collect();
                if let Some(base_mode) = mode_of(&merge(&trusted)?.table)
                    && base_mode.looseness() < effective_mode.looseness()
                {
                    effective_mode = base_mode;
                    warnings.push(
                        "project or local settings tried to loosen `permissions.mode`; ignored (set it in the user settings, or turn on `permissions.accept_project_allows` there)".to_owned(),
                    );
                }
            }
            let before = rules.len();
            rules.retain(|rule| !(rule.verdict == Verdict::Allow && rule.layer > LayerKind::User));
            if rules.len() != before {
                warnings.push(format!(
                    "{} allow rule(s) from project or local settings ignored; turn on `permissions.accept_project_allows` in the user settings to honour them",
                    before - rules.len()
                ));
            }
        }
        if effective_mode == Mode::Auto {
            let from_user =
                merged.provenance.get("permissions.auto_enabled") == Some(&LayerKind::User);
            if !(config.permissions.auto_enabled && from_user) {
                effective_mode = Mode::ReadOnly;
                warnings.push(
                    "`permissions.mode = \"auto\"` is ignored: `permissions.auto_enabled = true` must be set in the user settings; running read-only"
                        .to_owned(),
                );
            }
        }
        Ok(Self {
            config,
            effective_mode,
            warnings,
            rules,
            provenance: merged.provenance,
            table: merged.table,
        })
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn decide(&self, actor: &Actor, action: &str, scope: Option<&str>) -> Decision {
        decide(
            &self.config,
            self.effective_mode,
            &self.rules,
            actor,
            action,
            scope,
        )
    }

    /// Every setting with its value and the layer that set it, sorted by path. Rules are listed by `rules()`.
    pub fn effective(&self) -> Vec<EffectiveValue> {
        let mut flat = Vec::new();
        flatten(&Value::Table(self.table.clone()), "", &mut flat);
        flat.into_iter()
            .map(|(path, value)| {
                let layer = self
                    .provenance
                    .get(&path)
                    .copied()
                    .unwrap_or(LayerKind::Defaults);
                EffectiveValue { path, value, layer }
            })
            .collect()
    }
}

/// What a reload did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reload {
    /// True when the new configuration is now active.
    pub applied: bool,
    /// Why it was not applied. The previous good configuration stays active.
    pub error: Option<LoadError>,
}

/// Holds the active configuration. A bad reload never replaces it, so a typo cannot loosen anything.
#[derive(Debug)]
pub struct Manager {
    current: Arc<Loaded>,
    generation: u64,
    last_error: Option<LoadError>,
}

impl Manager {
    /// Start from the caller's layers. If they do not load, start from the defaults and keep the error to show.
    pub fn start(layers: &[Layer]) -> Result<Self, LoadError> {
        match Loaded::load(layers) {
            Ok(loaded) => Ok(Self {
                current: Arc::new(loaded),
                generation: 1,
                last_error: None,
            }),
            Err(error) => Ok(Self {
                current: Arc::new(Loaded::defaults()?),
                generation: 1,
                last_error: Some(error),
            }),
        }
    }

    pub fn current(&self) -> Arc<Loaded> {
        Arc::clone(&self.current)
    }

    /// Increases by one each time a new configuration is applied.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The error from the most recent failed start or reload; cleared by the next successful one.
    pub fn last_error(&self) -> Option<&LoadError> {
        self.last_error.as_ref()
    }

    pub fn reload(&mut self, layers: &[Layer]) -> Reload {
        match Loaded::load(layers) {
            Ok(loaded) => {
                self.current = Arc::new(loaded);
                self.generation += 1;
                self.last_error = None;
                Reload {
                    applied: true,
                    error: None,
                }
            }
            Err(error) => {
                self.last_error = Some(error.clone());
                Reload {
                    applied: false,
                    error: Some(error),
                }
            }
        }
    }
}
