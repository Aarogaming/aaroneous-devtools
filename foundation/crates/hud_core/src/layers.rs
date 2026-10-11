//! Layer merging with provenance and locks. Pure: layers arrive as text, nothing is read from disk.

use crate::error::LoadError;
use std::collections::BTreeMap;
use toml::{Table, Value};

/// Settings layers from lowest to highest precedence. `Defaults` is embedded and never supplied by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerKind {
    Defaults,
    User,
    Project,
    Local,
}

impl LayerKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Defaults => "defaults",
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
        }
    }
}

/// One caller-supplied layer. `text: None` means the file does not exist, which is normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub kind: LayerKind,
    pub text: Option<String>,
}

impl Layer {
    pub fn new(kind: LayerKind, text: Option<&str>) -> Self {
        Self {
            kind,
            text: text.map(str::to_owned),
        }
    }
}

pub(crate) const RULES_PATH: &str = "permissions.rules";

/// Settings that only the user layer may ever set: they decide who can loosen what.
const ALWAYS_OWNER_ONLY: &[&str] = &[
    "permissions.auto_enabled",
    "permissions.trust_shared_layers",
];

/// Settings that decide where work and data go, what may be spent, and what is exposed. A project or local layer
/// (which travels with a cloned repository) may set them only if the user layer set `permissions.trust_shared_layers`.
const TRUST_GATED: &[&str] = &[
    "routing",
    "models",
    "budgets",
    "agent_door",
    "operations.run_without_approval",
];

pub(crate) struct Merged {
    pub table: Table,
    /// Settings ignored because a shared layer may not set them, with the layer and path.
    pub warnings: Vec<String>,
    /// True when the user layer set `permissions.trust_shared_layers = true`.
    pub trust_shared: bool,
    /// Dotted path of every non-table value to the layer that last set it.
    pub provenance: BTreeMap<String, LayerKind>,
    /// Rule texts in layer order (lowest first), each with the layer that wrote it.
    pub rules: Vec<(LayerKind, String)>,
}

fn leaves(table: &Table, prefix: &str, out: &mut Vec<String>) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(inner) => leaves(inner, &path, out),
            _ => out.push(path),
        }
    }
}

fn overlaps(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}.")) || b.starts_with(&format!("{a}."))
}

fn merge_into(
    base: &mut Table,
    overlay: Table,
    prefix: &str,
    kind: LayerKind,
    provenance: &mut BTreeMap<String, LayerKind>,
) {
    for (key, value) in overlay {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(inner) => {
                let slot = base
                    .entry(key)
                    .or_insert_with(|| Value::Table(Table::new()));
                if let Value::Table(existing) = slot {
                    merge_into(existing, inner, &path, kind, provenance);
                } else {
                    // A scalar is being replaced by a table: forget the scalar's provenance.
                    provenance.remove(&path);
                    let mut fresh = Table::new();
                    merge_into(&mut fresh, inner, &path, kind, provenance);
                    *slot = Value::Table(fresh);
                }
            }
            other => {
                let prefix_dot = format!("{path}.");
                provenance.retain(|existing, _| !existing.starts_with(&prefix_dot));
                provenance.insert(path, kind);
                base.insert(key, other);
            }
        }
    }
}

fn trusted(merged: &Table, provenance: &BTreeMap<String, LayerKind>) -> bool {
    merged
        .get("permissions")
        .and_then(|permissions| permissions.get("trust_shared_layers"))
        .and_then(Value::as_bool)
        == Some(true)
        && provenance.get("permissions.trust_shared_layers") == Some(&LayerKind::User)
}

fn remove_path(table: &mut Table, path: &str) -> bool {
    match path.split_once('.') {
        None => table.remove(path).is_some(),
        Some((head, rest)) => match table.get_mut(head) {
            Some(Value::Table(inner)) => remove_path(inner, rest),
            _ => false,
        },
    }
}

/// A shared layer may add new actions but not change an action that already exists (built in or added by the user).
/// A key that contains a dot looks like a nested path in every path comparison here, so a shared file could use it to
/// slip past a filter that walks real tables. Such keys are never settings; drop them from shared layers.
fn strip_dotted_keys(table: &mut Table, prefix: &str, kind: LayerKind, warnings: &mut Vec<String>) {
    let dotted: Vec<String> = table
        .keys()
        .filter(|key| key.contains('.'))
        .cloned()
        .collect();
    for key in dotted {
        table.remove(&key);
        warnings.push(format!(
            "{} settings: key {prefix}{key:?} ignored; a key containing a dot is not a setting",
            kind.name()
        ));
    }
    for (key, value) in table.iter_mut() {
        if let Value::Table(inner) = value {
            strip_dotted_keys(inner, &format!("{prefix}{key}."), kind, warnings);
        }
    }
}

fn strip_reclassification(
    table: &mut Table,
    merged: &Table,
    kind: LayerKind,
    warnings: &mut Vec<String>,
) {
    let Some(Value::Table(layer_actions)) = table.get_mut("actions") else {
        return;
    };
    let Some(Value::Table(known)) = merged.get("actions") else {
        return;
    };
    for (name, definition) in layer_actions.iter_mut() {
        if !known.contains_key(name) {
            continue;
        }
        if let Value::Table(fields) = definition {
            for field in ["kind", "risk"] {
                if fields.remove(field).is_some() {
                    warnings.push(format!(
                        "{} settings: `actions.{name}.{field}` ignored; an existing action can only be reclassified in the user settings",
                        kind.name()
                    ));
                }
            }
        }
    }
}

fn take_lock_list(table: &mut Table, layer: &str) -> Result<Vec<String>, LoadError> {
    let Some(value) = table.remove("locked") else {
        return Ok(Vec::new());
    };
    let bad = || LoadError::BadLockList {
        layer: layer.to_owned(),
    };
    let Value::Array(items) = value else {
        return Err(bad());
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(path) if !path.is_empty() => Ok(path),
            _ => Err(bad()),
        })
        .collect()
}

fn take_rules(table: &mut Table, layer: &str) -> Result<Vec<String>, LoadError> {
    let Some(Value::Table(permissions)) = table.get_mut("permissions") else {
        return Ok(Vec::new());
    };
    let Some(value) = permissions.remove("rules") else {
        return Ok(Vec::new());
    };
    let invalid = || LoadError::Invalid {
        message: format!("{layer} settings: `{RULES_PATH}` must be an array of strings"),
    };
    let Value::Array(items) = value else {
        return Err(invalid());
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(rule) => Ok(rule),
            _ => Err(invalid()),
        })
        .collect()
}

/// Merge layers in order. Locks set by a layer bind only the layers above it.
pub(crate) fn merge(layers: &[(LayerKind, &str)]) -> Result<Merged, LoadError> {
    let mut merged = Table::new();
    let mut provenance = BTreeMap::new();
    let mut rules = Vec::new();
    let mut locks: Vec<(String, LayerKind)> = Vec::new();
    let mut warnings = Vec::new();
    for (kind, text) in layers {
        let mut table: Table = text
            .parse()
            .map_err(|error: toml::de::Error| LoadError::Parse {
                layer: kind.name().to_owned(),
                message: error.message().to_owned(),
            })?;
        let own_locks = take_lock_list(&mut table, kind.name())?;
        let own_rules = take_rules(&mut table, kind.name())?;

        if *kind >= LayerKind::Project {
            strip_dotted_keys(&mut table, "", *kind, &mut warnings);
            for path in ALWAYS_OWNER_ONLY {
                if remove_path(&mut table, path) {
                    warnings.push(format!(
                        "{} settings: `{path}` ignored; it can only be set in the user settings",
                        kind.name()
                    ));
                }
            }
            if !trusted(&merged, &provenance) {
                for path in TRUST_GATED {
                    if remove_path(&mut table, path) {
                        warnings.push(format!(
                            "{} settings: `{path}` ignored; set `permissions.trust_shared_layers = true` in the user settings to let shared files change it",
                            kind.name()
                        ));
                    }
                }
                strip_reclassification(&mut table, &merged, *kind, &mut warnings);
            }
        }

        let mut touched = Vec::new();
        leaves(&table, "", &mut touched);
        if !own_rules.is_empty() {
            touched.push(RULES_PATH.to_owned());
        }
        for path in &touched {
            if let Some((locked_path, locker)) =
                locks.iter().find(|(locked, _)| overlaps(path, locked))
            {
                return Err(LoadError::Locked {
                    path: locked_path.clone(),
                    locked_by: locker.name().to_owned(),
                    attempted_by: kind.name().to_owned(),
                });
            }
        }

        merge_into(&mut merged, table, "", *kind, &mut provenance);
        rules.extend(own_rules.into_iter().map(|rule| (*kind, rule)));
        locks.extend(own_locks.into_iter().map(|path| (path, *kind)));
    }
    let trust_shared = trusted(&merged, &provenance);
    Ok(Merged {
        table: merged,
        warnings,
        trust_shared,
        provenance,
        rules,
    })
}
