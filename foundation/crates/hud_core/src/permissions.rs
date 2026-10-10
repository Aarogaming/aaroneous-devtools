//! The permission engine: a mode plus allow, ask and deny rules. Deny beats ask beats allow; a small floor of actions
//! is refused whatever the settings say.

use crate::config::{ActionKind, Config, Mode, Risk};
use crate::layers::LayerKind;

/// Actions the HUD never performs for anyone. They stay with the steward gate and signed attestations; this is a
/// safety invariant, not a setting, and `Config::validate` refuses to register an action with one of these names.
pub const FLOOR_DENIED: &[&str] = &["merge", "push", "review", "attest"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Ask,
    Deny,
}

impl Verdict {
    /// Precedence when several rules match: deny beats ask beats allow.
    fn rank(self) -> u8 {
        match self {
            Self::Deny => 2,
            Self::Ask => 1,
            Self::Allow => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    Human,
    Seat(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActorPattern {
    Any,
    Human,
    AnySeat,
    Seat(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActionPattern {
    Any,
    Exact(String),
    Prefix(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopePattern {
    Any,
    SelfOnly,
    Literal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub verdict: Verdict,
    actor: ActorPattern,
    action: ActionPattern,
    scope: ScopePattern,
    pub layer: LayerKind,
    pub text: String,
}

fn seat_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn action_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Rule {
    /// Parse `"<allow|ask|deny> <actor> <action> [scope]"`.
    pub fn parse(text: &str, layer: LayerKind) -> Result<Self, String> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err("expected `<allow|ask|deny> <actor> <action> [scope]`".into());
        }
        let verdict = match parts[0] {
            "allow" => Verdict::Allow,
            "ask" => Verdict::Ask,
            "deny" => Verdict::Deny,
            other => return Err(format!("unknown effect `{other}`; use allow, ask or deny")),
        };
        let actor = match parts[1] {
            "*" => ActorPattern::Any,
            "human" => ActorPattern::Human,
            "seat:*" => ActorPattern::AnySeat,
            other => match other.strip_prefix("seat:") {
                Some(name) if seat_name_ok(name) => ActorPattern::Seat(name.to_owned()),
                _ => {
                    return Err(format!(
                        "unknown actor `{other}`; use *, human, seat:* or seat:<name>"
                    ));
                }
            },
        };
        let action = match parts[2] {
            "*" => ActionPattern::Any,
            other => match other.strip_suffix('*') {
                Some(prefix) if action_name_ok(prefix) => ActionPattern::Prefix(prefix.to_owned()),
                Some(_) => return Err(format!("bad action prefix `{other}`")),
                None if action_name_ok(other) => ActionPattern::Exact(other.to_owned()),
                None => return Err(format!("bad action name `{other}`")),
            },
        };
        let scope = match parts.get(3).copied() {
            None | Some("*") => ScopePattern::Any,
            Some("self") => ScopePattern::SelfOnly,
            Some(literal) => ScopePattern::Literal(literal.to_owned()),
        };
        Ok(Self {
            verdict,
            actor,
            action,
            scope,
            layer,
            text: text.trim().to_owned(),
        })
    }

    /// An exact action name this rule mentions, so loading can reject rules about actions that do not exist.
    pub(crate) fn exact_action(&self) -> Option<&str> {
        match &self.action {
            ActionPattern::Exact(name) => Some(name),
            _ => None,
        }
    }

    fn matches(&self, actor: &Actor, action: &str, scope: Option<&str>) -> bool {
        let actor_ok = match (&self.actor, actor) {
            (ActorPattern::Any, _) => true,
            (ActorPattern::Human, Actor::Human) => true,
            (ActorPattern::AnySeat, Actor::Seat(_)) => true,
            (ActorPattern::Seat(wanted), Actor::Seat(name)) => wanted == name,
            _ => false,
        };
        let action_ok = match &self.action {
            ActionPattern::Any => true,
            ActionPattern::Exact(name) => name == action,
            ActionPattern::Prefix(prefix) => action.starts_with(prefix.as_str()),
        };
        let scope_ok = match (&self.scope, actor, scope) {
            (ScopePattern::Any, _, _) => true,
            (ScopePattern::SelfOnly, Actor::Seat(name), Some(target)) => name == target,
            (ScopePattern::Literal(wanted), _, Some(target)) => wanted == target,
            _ => false,
        };
        actor_ok && action_ok && scope_ok
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The action is on the permission floor.
    Floor,
    /// A rule decided it.
    Rule { layer: LayerKind, text: String },
    /// No rule matched and the action is a read.
    ReadDefault,
    /// No rule matched; the mode decided a write.
    ModeDefault(Mode),
    /// No rule matched and `auto` mode let a low-risk write through.
    AutoLowRisk,
    /// The action is not in the registry; unknown actions are refused.
    UnknownAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    pub reason: Reason,
}

impl Decision {
    fn new(verdict: Verdict, reason: Reason) -> Self {
        Self { verdict, reason }
    }
}

/// Decide one request. `mode` is the effective mode (already downgraded if `auto` was not properly enabled).
pub fn decide(
    config: &Config,
    mode: Mode,
    rules: &[Rule],
    actor: &Actor,
    action: &str,
    scope: Option<&str>,
) -> Decision {
    if FLOOR_DENIED.contains(&action) {
        return Decision::new(Verdict::Deny, Reason::Floor);
    }
    let strongest = rules
        .iter()
        .filter(|rule| rule.matches(actor, action, scope))
        .fold(None::<&Rule>, |best, rule| match best {
            Some(current) if current.verdict.rank() >= rule.verdict.rank() => Some(current),
            _ => Some(rule),
        });
    if let Some(rule) = strongest {
        return Decision::new(
            rule.verdict,
            Reason::Rule {
                layer: rule.layer,
                text: rule.text.clone(),
            },
        );
    }
    let Some(definition) = config.actions.get(action) else {
        return Decision::new(Verdict::Deny, Reason::UnknownAction);
    };
    match definition.kind {
        ActionKind::Read => Decision::new(Verdict::Allow, Reason::ReadDefault),
        ActionKind::Write => match mode {
            Mode::ReadOnly => Decision::new(Verdict::Deny, Reason::ModeDefault(mode)),
            Mode::Ask => Decision::new(Verdict::Ask, Reason::ModeDefault(mode)),
            Mode::Auto => match definition.risk {
                Risk::Low => Decision::new(Verdict::Allow, Reason::AutoLowRisk),
                Risk::High => Decision::new(Verdict::Ask, Reason::ModeDefault(mode)),
            },
        },
    }
}
