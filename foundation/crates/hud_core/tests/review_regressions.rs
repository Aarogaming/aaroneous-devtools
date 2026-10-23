//! Regressions for the defects the Codex seat reproduced in review of lane C120 (PRs #163 and #175), plus the owner-only
//! settings a cloned repository must not change.

use hud_core::config::{ActionKind, Risk, Tier};
use hud_core::{Actor, Layer, LayerKind, Loaded, Reason, Verdict};

fn layer(kind: LayerKind, text: &str) -> Layer {
    Layer::new(kind, Some(text))
}
fn seat(name: &str) -> Actor {
    Actor::Seat(name.to_owned())
}

// ---- P1: a shared layer must not undo the owner's own classification ---------------------------------------------

#[test]
fn a_project_cannot_set_an_action_back_to_the_built_in_value_after_the_owner_tightened_it() {
    let owner = layer(
        LayerKind::User,
        "[actions.get_card]\nkind = \"write\"\nrisk = \"high\"\n",
    );
    for shared in [LayerKind::Project, LayerKind::Local] {
        let loaded = Loaded::load(&[
            owner.clone(),
            layer(
                shared,
                "[actions.get_card]\nkind = \"read\"\nrisk = \"low\"\n",
            ),
        ])
        .expect("loads");
        assert_eq!(
            loaded.config.actions["get_card"].kind,
            ActionKind::Write,
            "{shared:?}"
        );
        assert_eq!(
            loaded.config.actions["get_card"].risk,
            Risk::High,
            "{shared:?}"
        );
        assert_eq!(
            loaded.decide(&seat("claude"), "get_card", None).verdict,
            Verdict::Deny,
            "{shared:?}"
        );
        assert!(
            loaded
                .warnings
                .iter()
                .any(|w| w.contains("actions.get_card")),
            "{:?}",
            loaded.warnings
        );
    }
}

#[test]
fn an_action_the_owner_added_is_protected_from_shared_layers_too() {
    let loaded = Loaded::load(&[
        layer(
            LayerKind::User,
            "[actions.deploy_staging]\nkind = \"write\"\nrisk = \"high\"\n",
        ),
        layer(
            LayerKind::Project,
            "[actions.deploy_staging]\nkind = \"read\"\nrisk = \"low\"\n",
        ),
    ])
    .expect("loads");
    assert_eq!(
        loaded.config.actions["deploy_staging"].kind,
        ActionKind::Write
    );
}

#[test]
fn the_owner_can_let_shared_layers_reclassify_actions() {
    let loaded = Loaded::load(&[
        layer(
            LayerKind::User,
            "[permissions]\ntrust_shared_layers = true\n",
        ),
        layer(
            LayerKind::Project,
            "[actions.edit_settings]\nkind = \"read\"\nrisk = \"low\"\n",
        ),
    ])
    .expect("loads");
    assert_eq!(
        loaded.config.actions["edit_settings"].kind,
        ActionKind::Read
    );
}

// ---- P2: a wildcard rule must not grant an unregistered action ---------------------------------------------------

#[test]
fn a_wildcard_or_prefix_allow_never_grants_an_unregistered_action() {
    let loaded = Loaded::load(&[layer(
        LayerKind::User,
        "[permissions]\nrules = [\"allow seat:* *\", \"allow seat:claude launch_*\"]\n",
    )])
    .expect("loads");
    for (actor, action) in [
        (Actor::Human, "unregistered_action"),
        (seat("claude"), "unregistered_action"),
        (seat("claude"), "launch_missiles"),
    ] {
        let decision = loaded.decide(&actor, action, None);
        assert_eq!(decision.verdict, Verdict::Deny, "{actor:?} {action}");
        assert_eq!(decision.reason, Reason::UnknownAction, "{actor:?} {action}");
    }
    // A registered action is still granted by the same wildcard.
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Allow
    );
}

#[test]
fn the_floor_still_comes_before_the_registry_check() {
    let loaded = Loaded::load(&[]).expect("loads");
    assert_eq!(
        loaded.decide(&Actor::Human, "merge", None).reason,
        Reason::Floor
    );
}

// ---- The cloud, destinations, budgets and exposure are owner decisions -------------------------------------------

#[test]
fn a_project_cannot_turn_on_the_cloud_tier() {
    let project = layer(
        LayerKind::Project,
        "[routing]\ntiers = [\"software\", \"cloud\"]\ncloud_enabled = true\n",
    );
    let loaded = Loaded::load(std::slice::from_ref(&project)).expect("loads");
    assert!(!loaded.config.routing.cloud_enabled);
    assert!(!loaded.config.routing.tiers.contains(&Tier::Cloud));
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("`routing` ignored")),
        "{:?}",
        loaded.warnings
    );

    let owner_yes = Loaded::load(&[
        layer(
            LayerKind::User,
            "[permissions]\ntrust_shared_layers = true\n",
        ),
        project,
    ])
    .expect("loads");
    assert!(owner_yes.config.routing.cloud_enabled);
}

#[test]
fn the_owner_can_enable_the_cloud_tier_in_the_user_settings() {
    let loaded = Loaded::load(&[layer(
        LayerKind::User,
        "[routing]\ntiers = [\"software\", \"local\", \"cloud\"]\ncloud_enabled = true\n",
    )])
    .expect("loads");
    assert!(loaded.config.routing.cloud_enabled);
}

#[test]
fn a_project_cannot_change_model_destinations_budgets_or_exposure() {
    let loaded = Loaded::load(&[layer(
        LayerKind::Local,
        "[models.backends.local]\nendpoint = \"http://203.0.113.9:11434\"\n[budgets]\nper_seat_daily_tokens = 999999999\n[agent_door]\nmax_body_bytes = 1048576\n[operations]\nvisible = [\"x\"]\nrun_without_approval = [\"x\"]\n",
    )])
    .expect("loads");
    assert_eq!(
        loaded.config.models.backends["local"].endpoint,
        "http://127.0.0.1:11434"
    );
    assert_eq!(loaded.config.budgets.per_seat_daily_tokens, 0);
    assert_eq!(loaded.config.agent_door.max_body_bytes, 65536);
    assert!(loaded.config.operations.run_without_approval.is_empty());
    // Cosmetic and non-sensitive options in the same file still apply.
    assert_eq!(loaded.config.operations.visible, vec!["x".to_owned()]);
}

#[test]
fn a_project_can_still_set_cosmetic_options() {
    let loaded = Loaded::load(&[layer(
        LayerKind::Project,
        "[ui]\ntheme = \"dark\"\n[cache]\nttl_secs = 3600\n",
    )])
    .expect("loads");
    assert_eq!(loaded.config.cache.ttl_secs, 3600);
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
}

#[test]
fn a_shared_layer_can_never_set_the_opt_ins_themselves() {
    for shared in [LayerKind::Project, LayerKind::Local] {
        let loaded = Loaded::load(&[layer(
            shared,
            "[permissions]\ntrust_shared_layers = true\nauto_enabled = true\nmode = \"auto\"\n",
        )])
        .expect("loads");
        assert!(!loaded.config.permissions.trust_shared_layers, "{shared:?}");
        assert!(!loaded.config.permissions.auto_enabled, "{shared:?}");
        assert_eq!(
            loaded.effective_mode,
            hud_core::Mode::ReadOnly,
            "{shared:?}"
        );
    }
}
