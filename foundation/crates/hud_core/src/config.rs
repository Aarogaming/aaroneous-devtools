//! Typed settings. Unknown keys are errors, so a typo cannot silently fall back to a default.

use crate::error::LoadError;
use crate::permissions::FLOOR_DENIED;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub permissions: PermissionsSection,
    pub actions: BTreeMap<String, ActionDef>,
    pub agent_door: AgentDoor,
    pub routing: Routing,
    pub models: Models,
    pub budgets: Budgets,
    pub cache: Cache,
    pub events: Events,
    pub operations: Operations,
    pub ui: Ui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    ReadOnly,
    Ask,
    Auto,
}

impl Mode {
    /// How much a mode lets through without a person: read-only is the strictest.
    pub fn looseness(self) -> u8 {
        match self {
            Self::ReadOnly => 0,
            Self::Ask => 1,
            Self::Auto => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionsSection {
    pub mode: Mode,
    pub auto_enabled: bool,
    /// Honour `allow` rules, a looser mode, routing, model destinations, budgets, exposure and reclassified actions
    /// that come from the shared project or local layers. Takes effect only when set in the user layer, so a cloned
    /// repository cannot loosen the owner's machine.
    pub trust_shared_layers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    Low,
    #[default]
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDef {
    pub kind: ActionKind,
    #[serde(default)]
    pub risk: Risk,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDoor {
    pub enabled: bool,
    pub token_lifetime_secs: u64,
    pub rate_per_minute: u32,
    pub max_body_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Software,
    Local,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routing {
    pub tiers: Vec<Tier>,
    pub cloud_enabled: bool,
    pub min_confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Ollama,
    OpenaiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backend {
    pub kind: BackendKind,
    pub endpoint: String,
    pub model: String,
    pub context_tokens: u32,
    pub timeout_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    pub backends: BTreeMap<String, Backend>,
    /// Role name (for example `glue`) to the backend that serves it.
    pub roles: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Exhaustion {
    Queue,
    Ask,
    Refuse,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub accounting: bool,
    /// 0 means no cap.
    pub per_seat_daily_tokens: u64,
    pub on_exhaustion: Exhaustion,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cache {
    pub enabled: bool,
    pub max_entries: usize,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Events {
    pub long_poll_timeout_secs: u32,
    pub retention: usize,
    pub heartbeat_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operations {
    pub visible: Vec<String>,
    pub run_without_approval: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationLevel {
    Quiet,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Visualizer {
    pub enabled: bool,
    pub max_fps: u32,
    pub fallback_2d_when_model_busy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    pub theme: Theme,
    pub notification_level: NotificationLevel,
    pub visualizer: Visualizer,
}

fn invalid(message: impl Into<String>) -> LoadError {
    LoadError::Invalid {
        message: message.into(),
    }
}

fn in_range<T: PartialOrd + std::fmt::Display>(
    name: &str,
    value: T,
    low: T,
    high: T,
) -> Result<(), LoadError> {
    if value < low || value > high {
        return Err(invalid(format!(
            "`{name}` is {value}; it must be between {low} and {high}"
        )));
    }
    Ok(())
}

impl Config {
    /// Range and consistency checks that the schema alone cannot express.
    pub fn validate(&self) -> Result<(), LoadError> {
        for name in self.actions.keys() {
            if FLOOR_DENIED.contains(&name.as_str()) {
                return Err(invalid(format!(
                    "`actions.{name}` cannot be registered: `{name}` is on the permission floor and is never allowed"
                )));
            }
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                return Err(invalid(format!(
                    "action name {name:?} must use lower-case letters, digits and `_`"
                )));
            }
        }

        in_range(
            "agent_door.token_lifetime_secs",
            self.agent_door.token_lifetime_secs,
            60,
            30 * 86_400,
        )?;
        in_range(
            "agent_door.rate_per_minute",
            self.agent_door.rate_per_minute,
            1,
            100_000,
        )?;
        in_range(
            "agent_door.max_body_bytes",
            self.agent_door.max_body_bytes,
            1024,
            1024 * 1024,
        )?;

        let routing = &self.routing;
        if routing.tiers.is_empty() {
            return Err(invalid("`routing.tiers` must name at least one tier"));
        }
        for (index, tier) in routing.tiers.iter().enumerate() {
            if routing.tiers[..index].contains(tier) {
                return Err(invalid(format!("`routing.tiers` lists {tier:?} twice")));
            }
        }
        if routing.tiers.contains(&Tier::Cloud) && !routing.cloud_enabled {
            return Err(invalid(
                "`routing.tiers` includes `cloud` but `routing.cloud_enabled` is false",
            ));
        }
        if !(0.0..=1.0).contains(&routing.min_confidence) {
            return Err(invalid("`routing.min_confidence` must be between 0 and 1"));
        }

        for (name, backend) in &self.models.backends {
            if !(backend.endpoint.starts_with("http://")
                || backend.endpoint.starts_with("https://"))
            {
                return Err(invalid(format!(
                    "`models.backends.{name}.endpoint` must start with http:// or https://"
                )));
            }
            if backend.model.trim().is_empty() {
                return Err(invalid(format!("`models.backends.{name}.model` is empty")));
            }
            in_range(
                &format!("models.backends.{name}.context_tokens"),
                backend.context_tokens,
                256,
                4_000_000,
            )?;
            in_range(
                &format!("models.backends.{name}.timeout_secs"),
                backend.timeout_secs,
                1,
                3600,
            )?;
        }
        for (role, backend) in &self.models.roles {
            if !self.models.backends.contains_key(backend) {
                return Err(invalid(format!(
                    "`models.roles.{role}` names the backend `{backend}`, which is not defined"
                )));
            }
        }
        if routing.tiers.contains(&Tier::Local) && !self.models.roles.contains_key("glue") {
            return Err(invalid(
                "`routing.tiers` includes `local` but `models.roles.glue` is not set",
            ));
        }

        if self.cache.enabled {
            in_range("cache.max_entries", self.cache.max_entries, 1, 100_000_000)?;
        }
        in_range(
            "events.long_poll_timeout_secs",
            self.events.long_poll_timeout_secs,
            1,
            120,
        )?;
        in_range("events.heartbeat_secs", self.events.heartbeat_secs, 1, 3600)?;
        in_range("events.retention", self.events.retention, 1, 100_000_000)?;
        in_range("ui.visualizer.max_fps", self.ui.visualizer.max_fps, 1, 240)?;

        for name in &self.operations.run_without_approval {
            if !self.operations.visible.contains(name) {
                return Err(invalid(format!(
                    "`operations.run_without_approval` lists `{name}`, which is not in `operations.visible`"
                )));
            }
        }
        Ok(())
    }
}
