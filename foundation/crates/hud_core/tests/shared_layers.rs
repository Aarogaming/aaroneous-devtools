//! A cloned repository's shared settings (project and local layers) may tighten permissions but not loosen them
//! unless the owner opts in from the user settings (lane C120).

use hud_core::{Actor, Layer, LayerKind, LoadError, Loaded, Mode, Verdict};

fn layer(kind: LayerKind, text: &str) -> Layer {
    Layer::new(kind, Some(text))
}
fn seat(name: &str) -> Actor {
    Actor::Seat(name.to_owned())
}

#[test]
fn a_project_allow_rule_is_ignored_with_a_warning_unless_the_owner_opts_in() {
    let project = layer(
        LayerKind::Project,
        "[permissions]\nrules = [\"allow seat:* edit_settings\"]\n",
    );
    let loaded = Loaded::load(std::slice::from_ref(&project)).expect("loads");
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Deny
    );
    assert!(
        loaded.warnings.iter().any(|w| w.contains("allow rule")),
        "{:?}",
        loaded.warnings
    );

    let trusted = layer(
        LayerKind::User,
        "[permissions]\naccept_project_allows = true\n",
    );
    let loaded = Loaded::load(&[trusted, project]).expect("loads");
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Allow
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
}

#[test]
fn a_user_allow_rule_is_honoured_without_any_opt_in() {
    let user = layer(
        LayerKind::User,
        "[permissions]\nrules = [\"allow seat:* edit_settings\"]\n",
    );
    let loaded = Loaded::load(&[user]).expect("loads");
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Allow
    );
}

#[test]
fn project_and_local_deny_and_ask_rules_still_apply() {
    let loaded = Loaded::load(&[
        layer(
            LayerKind::Project,
            "[permissions]\nrules = [\"deny seat:gemini get_card\"]\n",
        ),
        layer(
            LayerKind::Local,
            "[permissions]\nrules = [\"ask seat:claude list_work\"]\n",
        ),
    ])
    .expect("loads");
    assert_eq!(
        loaded.decide(&seat("gemini"), "get_card", None).verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded.decide(&seat("claude"), "list_work", None).verdict,
        Verdict::Ask
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
}

#[test]
fn a_project_cannot_turn_on_its_own_opt_in() {
    let loaded = Loaded::load(&[layer(
        LayerKind::Project,
        "[permissions]\naccept_project_allows = true\nrules = [\"allow seat:* edit_settings\"]\n",
    )])
    .expect("loads");
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Deny
    );
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("accept_project_allows")),
        "{:?}",
        loaded.warnings
    );
}

#[test]
fn a_project_cannot_loosen_the_mode_but_may_tighten_it() {
    let loosened = Loaded::load(&[layer(LayerKind::Project, "[permissions]\nmode = \"ask\"\n")])
        .expect("loads");
    assert_eq!(loosened.effective_mode, Mode::ReadOnly);
    assert!(
        loosened
            .warnings
            .iter()
            .any(|w| w.contains("loosen `permissions.mode`")),
        "{:?}",
        loosened.warnings
    );

    let tightened = Loaded::load(&[
        layer(LayerKind::User, "[permissions]\nmode = \"ask\"\n"),
        layer(LayerKind::Project, "[permissions]\nmode = \"read-only\"\n"),
    ])
    .expect("loads");
    assert_eq!(tightened.effective_mode, Mode::ReadOnly);
    assert!(tightened.warnings.is_empty(), "{:?}", tightened.warnings);

    let owner_choice =
        Loaded::load(&[layer(LayerKind::User, "[permissions]\nmode = \"ask\"\n")]).expect("loads");
    assert_eq!(owner_choice.effective_mode, Mode::Ask);
}

#[test]
fn the_owner_can_let_a_project_loosen_the_mode() {
    let loaded = Loaded::load(&[
        layer(
            LayerKind::User,
            "[permissions]\naccept_project_allows = true\n",
        ),
        layer(LayerKind::Project, "[permissions]\nmode = \"ask\"\n"),
    ])
    .expect("loads");
    assert_eq!(loaded.effective_mode, Mode::Ask);
}

#[test]
fn a_project_cannot_reclassify_a_built_in_action_but_may_add_a_new_one() {
    let reclassified = Loaded::load(&[layer(
        LayerKind::Project,
        "[actions.edit_settings]\nkind = \"read\"\nrisk = \"low\"\n",
    )]);
    assert!(
        matches!(reclassified, Err(LoadError::Invalid { ref message }) if message.contains("reclassify")),
        "{reclassified:?}"
    );

    let added = Loaded::load(&[layer(
        LayerKind::Project,
        "[actions.summarise_day]\nkind = \"write\"\nrisk = \"low\"\n",
    )])
    .expect("a new action is fine");
    assert!(added.config.actions.contains_key("summarise_day"));
}

#[test]
fn the_owner_may_reclassify_an_action_in_the_user_settings() {
    let loaded = Loaded::load(&[layer(
        LayerKind::User,
        "[actions.report_result]\nkind = \"write\"\nrisk = \"low\"\n",
    )])
    .expect("loads");
    assert_eq!(
        loaded.config.actions["report_result"].risk,
        hud_core::config::Risk::Low
    );
}
