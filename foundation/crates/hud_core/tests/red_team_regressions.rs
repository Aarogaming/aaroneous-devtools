//! Regressions from a red-team pass over lane C120 (2026-10-11): a hostile or broken shared file must not cost the owner
//! their own restrictions, and a path-like key must not slip past the shared-layer filters.

use hud_core::{Actor, Layer, LayerKind, Loaded, Manager, Verdict};

fn layer(kind: LayerKind, text: &str) -> Layer {
    Layer::new(kind, Some(text))
}
fn seat(name: &str) -> Actor {
    Actor::Seat(name.to_owned())
}

const OWNER: &str =
    "[permissions]\nrules = [\"deny seat:gemini get_card\"]\n[ui]\ntheme = \"dark\"\n";

#[test]
fn a_malformed_project_file_at_start_does_not_drop_the_owners_deny_rule() {
    let owner = layer(LayerKind::User, OWNER);
    let broken = layer(LayerKind::Project, "this is = = not toml");
    let manager = Manager::start(&[owner, broken]).expect("start");
    let loaded = manager.current();
    assert_eq!(
        loaded.decide(&seat("gemini"), "get_card", None).verdict,
        Verdict::Deny
    );
    assert_eq!(loaded.config.ui.theme, hud_core::config::Theme::Dark);
    assert!(manager.last_error().is_some());
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("project and local settings were not applied")),
        "{:?}",
        loaded.warnings
    );
}

#[test]
fn an_unknown_key_in_a_project_file_at_start_also_keeps_the_owners_settings() {
    let manager = Manager::start(&[
        layer(LayerKind::User, OWNER),
        layer(LayerKind::Project, "[surprise]\nx = 1\n"),
    ])
    .expect("start");
    assert_eq!(
        manager
            .current()
            .decide(&seat("gemini"), "get_card", None)
            .verdict,
        Verdict::Deny
    );
}

#[test]
fn a_broken_owner_file_starts_in_safe_hold_and_refuses_everything() {
    // Codex review of #175: falling back to the defaults let a broken user file turn the owner's deny into an allow.
    // Now an unreadable trusted policy refuses every request, reads included, until a valid one is applied.
    let good = "[permissions]
rules = [\"deny seat:gemini get_card\"]
";
    let before = Manager::start(&[layer(LayerKind::User, good)]).expect("start");
    assert_eq!(
        before
            .current()
            .decide(&seat("gemini"), "get_card", None)
            .verdict,
        Verdict::Deny
    );
    let manager =
        Manager::start(&[layer(LayerKind::User, &format!("{good}not = = toml"))]).expect("start");
    assert!(manager.last_error().is_some());
    assert!(manager.current().is_safe_hold());
    for actor in ["claude", "gemini", "codex"] {
        for action in [
            "get_card",
            "list_work",
            "edit_settings",
            "run_operation",
            "merge",
            "unregistered",
        ] {
            let decision = manager.current().decide(&seat(actor), action, None);
            assert_eq!(decision.verdict, Verdict::Deny, "{actor} {action}");
            assert_eq!(
                decision.reason,
                hud_core::Reason::SafeHold,
                "{actor} {action}"
            );
        }
    }
    assert!(!manager.current().warnings.is_empty());
}

#[test]
fn safe_hold_ends_when_a_valid_policy_is_applied() {
    let mut manager = Manager::start(&[layer(LayerKind::User, "not = = toml")]).expect("start");
    assert!(manager.current().is_safe_hold());
    let reload = manager.reload(&[layer(LayerKind::User, "")]);
    assert!(reload.applied);
    assert!(!manager.current().is_safe_hold());
    assert_eq!(
        manager
            .current()
            .decide(&seat("claude"), "get_card", None)
            .verdict,
        Verdict::Allow
    );
}

#[test]
fn a_quoted_dotted_key_in_a_shared_file_is_ignored_and_the_file_still_loads() {
    let project = layer(
        LayerKind::Project,
        "\"routing.cloud_enabled\" = true\n\"permissions.mode\" = \"auto\"\n[ui]\ntheme = \"light\"\n",
    );
    let loaded = Loaded::load(&[layer(LayerKind::User, OWNER), project]).expect("loads");
    assert!(!loaded.config.routing.cloud_enabled);
    assert_eq!(loaded.effective_mode, hud_core::Mode::ReadOnly);
    assert_eq!(loaded.config.ui.theme, hud_core::config::Theme::Light);
    assert_eq!(
        loaded
            .warnings
            .iter()
            .filter(|w| w.contains("not a setting"))
            .count(),
        2,
        "{:?}",
        loaded.warnings
    );
}

#[test]
fn a_dotted_key_in_a_nested_shared_table_is_ignored_too() {
    let project = layer(
        LayerKind::Local,
        "[models.backends.local]\n\"endpoint\" = \"http://127.0.0.1:11434\"\n[routing]\n\"cloud_enabled.x\" = true\n",
    );
    let loaded = Loaded::load(&[project]).expect("loads");
    assert!(!loaded.config.routing.cloud_enabled);
}

#[test]
fn an_oversized_shared_file_is_refused_and_the_owner_keeps_their_settings_at_start() {
    let huge = format!("# {}\n", "x".repeat(hud_core::MAX_LAYER_BYTES + 1));
    let refused = Loaded::load(&[layer(LayerKind::Project, &huge)]);
    assert!(matches!(refused, Err(hud_core::LoadError::Invalid { .. })));
    let manager = Manager::start(&[
        layer(LayerKind::User, OWNER),
        layer(LayerKind::Project, &huge),
    ])
    .expect("start");
    assert_eq!(
        manager
            .current()
            .decide(&seat("gemini"), "get_card", None)
            .verdict,
        Verdict::Deny
    );
}

#[test]
fn a_flood_of_rules_is_refused() {
    let mut text = String::from("[permissions]\nrules = [\n");
    for _ in 0..=hud_core::MAX_RULES {
        text.push_str("  \"deny seat:x get_card\",\n");
    }
    text.push_str("]\n");
    let refused = Loaded::load(&[layer(LayerKind::User, &text)]);
    assert!(
        matches!(refused, Err(hud_core::LoadError::Invalid { ref message }) if message.contains("limit")),
        "{refused:?}"
    );
}
