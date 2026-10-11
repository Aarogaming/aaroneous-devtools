//! Contract tests for layered settings and the permission engine (lane C120). They use only the public API.

use hud_core::{Actor, Layer, LayerKind, LoadError, Loaded, Manager, Mode, Reason, Verdict};

fn user(text: &str) -> Layer {
    Layer::new(LayerKind::User, Some(text))
}
fn project(text: &str) -> Layer {
    Layer::new(LayerKind::Project, Some(text))
}
fn local(text: &str) -> Layer {
    Layer::new(LayerKind::Local, Some(text))
}
fn seat(name: &str) -> Actor {
    Actor::Seat(name.to_owned())
}
fn load(layers: &[Layer]) -> Loaded {
    Loaded::load(layers).expect("layers should load")
}
fn error(layers: &[Layer]) -> LoadError {
    Loaded::load(layers).expect_err("layers should be refused")
}

// ---- defaults -------------------------------------------------------------------------------------------------

#[test]
fn the_embedded_defaults_load_and_start_read_only_with_no_warnings() {
    let loaded = Loaded::defaults().expect("defaults load");
    assert_eq!(loaded.config.permissions.mode, Mode::ReadOnly);
    assert_eq!(loaded.effective_mode, Mode::ReadOnly);
    assert!(loaded.warnings.is_empty());
    assert!(!loaded.config.routing.cloud_enabled);
    assert_eq!(
        loaded.config.models.roles.get("glue").map(String::as_str),
        Some("local")
    );
}

#[test]
fn a_missing_settings_file_is_the_same_as_an_empty_one() {
    let none = [Layer::new(LayerKind::User, None)];
    assert_eq!(load(&none).effective(), load(&[]).effective());
}

#[test]
fn by_default_a_seat_reads_freely_but_cannot_change_state_unasked() {
    let loaded = load(&[]);
    for read in ["get_card", "list_work", "get_availability", "read_events"] {
        let decision = loaded.decide(&seat("claude"), read, None);
        assert_eq!(decision.verdict, Verdict::Allow, "{read}");
        assert_eq!(decision.reason, Reason::ReadDefault, "{read}");
    }
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "report_result", None)
            .verdict,
        Verdict::Ask
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "submit_inquiry", None)
            .verdict,
        Verdict::Ask
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "run_operation", None)
            .verdict,
        Verdict::Ask
    );
}

#[test]
fn a_seat_may_set_its_own_availability_but_not_another_seats() {
    let loaded = load(&[]);
    assert_eq!(
        loaded
            .decide(&seat("claude"), "set_availability", Some("claude"))
            .verdict,
        Verdict::Allow
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "set_availability", Some("gemini"))
            .verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "set_availability", None)
            .verdict,
        Verdict::Deny
    );
}

#[test]
fn the_owner_acting_as_a_human_is_allowed_everything_except_the_floor() {
    let loaded = load(&[]);
    assert_eq!(
        loaded.decide(&Actor::Human, "edit_settings", None).verdict,
        Verdict::Allow
    );
    assert_eq!(
        loaded.decide(&Actor::Human, "run_operation", None).verdict,
        Verdict::Allow
    );
}

#[test]
fn an_action_that_is_not_registered_is_refused() {
    let decision = load(&[]).decide(&seat("claude"), "launch_missiles", None);
    assert_eq!(decision.verdict, Verdict::Deny);
    assert_eq!(decision.reason, Reason::UnknownAction);
}

// ---- the floor ------------------------------------------------------------------------------------------------

#[test]
fn merge_push_review_and_attest_are_refused_for_everyone_whatever_the_rules_say() {
    let loaded = load(&[user(
        "[permissions]\nrules = [\"allow * *\", \"allow * merge\", \"allow human push\"]\n",
    )]);
    for action in hud_core::FLOOR_DENIED {
        for actor in [Actor::Human, seat("claude")] {
            let decision = loaded.decide(&actor, action, None);
            assert_eq!(decision.verdict, Verdict::Deny, "{action}");
            assert_eq!(decision.reason, Reason::Floor, "{action}");
        }
    }
}

#[test]
fn a_floor_action_cannot_be_registered_as_an_ordinary_action() {
    let refused = error(&[user("[actions.merge]\nkind = \"write\"\nrisk = \"low\"\n")]);
    assert!(
        matches!(refused, LoadError::Invalid { ref message } if message.contains("permission floor")),
        "{refused}"
    );
}

// ---- rules ----------------------------------------------------------------------------------------------------

#[test]
fn deny_beats_ask_beats_allow_whichever_layer_wrote_them() {
    let loaded = load(&[
        user("[permissions]\nrules = [\"deny seat:gemini report_result\"]\n"),
        project("[permissions]\nrules = [\"allow seat:* report_result\"]\n"),
    ]);
    let denied = loaded.decide(&seat("gemini"), "report_result", None);
    assert_eq!(denied.verdict, Verdict::Deny);
    assert!(matches!(
        denied.reason,
        Reason::Rule {
            layer: LayerKind::User,
            ..
        }
    ));
    // The project's allow beats the default `ask` rule for everyone else.
    assert_eq!(
        loaded
            .decide(&seat("claude"), "report_result", None)
            .verdict,
        Verdict::Ask
    );
}

#[test]
fn a_higher_layer_can_add_rules_but_not_remove_a_lower_layers_deny() {
    let loaded = load(&[
        user(
            "[permissions]\ntrust_shared_layers = true\nrules = [\"deny seat:* run_operation\"]\n",
        ),
        local("[permissions]\nrules = [\"allow seat:* run_operation\"]\n"),
    ]);
    assert_eq!(
        loaded
            .decide(&seat("claude"), "run_operation", None)
            .verdict,
        Verdict::Deny
    );
    assert_eq!(loaded.rules().len(), load(&[]).rules().len() + 2);
}

#[test]
fn prefix_and_literal_scope_rules_match_as_written() {
    let text = "allow seat:claude edit_* lane:C120";
    let loaded = load(&[user(&format!(
        "[permissions]
rules = [\"{text}\", \"deny seat:claude report_*\"]
"
    ))]);
    let allowed = loaded.decide(&seat("claude"), "edit_settings", Some("lane:C120"));
    assert_eq!(
        (allowed.verdict, allowed.reason),
        (
            Verdict::Allow,
            Reason::Rule {
                layer: LayerKind::User,
                text: text.to_owned()
            }
        )
    );
    // Wrong scope, wrong seat or no scope: the allow does not apply and the read-only default refuses.
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", Some("lane:C1"))
            .verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded
            .decide(&seat("gemini"), "edit_settings", Some("lane:C120"))
            .verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Deny
    );
    // A prefix deny also beats the default `ask` rule for the same action.
    assert_eq!(
        loaded
            .decide(&seat("claude"), "report_result", None)
            .verdict,
        Verdict::Deny
    );
    assert_eq!(
        loaded
            .decide(&seat("gemini"), "report_result", None)
            .verdict,
        Verdict::Ask
    );
}

#[test]
fn malformed_rules_name_the_layer_and_the_rule() {
    for (rule, fragment) in [
        ("permit seat:* get_card", "unknown effect"),
        ("allow robot get_card", "unknown actor"),
        ("allow seat:* Get-Card", "bad action name"),
        ("allow seat:*", "expected"),
    ] {
        let refused = error(&[project(&format!("[permissions]\nrules = [\"{rule}\"]\n"))]);
        match refused {
            LoadError::BadRule {
                layer,
                rule: shown,
                message,
            } => {
                assert_eq!(layer, "project");
                assert_eq!(shown, rule);
                assert!(message.contains(fragment), "{message}");
            }
            other => panic!("{other}"),
        }
    }
}

#[test]
fn a_rule_about_an_action_nobody_registered_is_refused() {
    let refused = error(&[user(
        "[permissions]\nrules = [\"allow seat:* get_cardd\"]\n",
    )]);
    assert!(
        matches!(refused, LoadError::BadRule { ref message, .. } if message.contains("not a registered action")),
        "{refused}"
    );
}

// ---- modes ----------------------------------------------------------------------------------------------------

#[test]
fn ask_mode_asks_for_unmatched_writes() {
    let loaded = load(&[user("[permissions]\nmode = \"ask\"\n")]);
    assert_eq!(
        loaded
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Ask
    );
    assert_eq!(
        loaded.decide(&seat("claude"), "get_card", None).verdict,
        Verdict::Allow
    );
}

#[test]
fn auto_mode_is_ignored_unless_the_user_layer_enabled_it() {
    let alone = load(&[user("[permissions]\nmode = \"auto\"\n")]);
    assert_eq!(alone.effective_mode, Mode::ReadOnly);
    assert!(
        alone.warnings.iter().any(|w| w.contains("auto_enabled")),
        "{:?}",
        alone.warnings
    );

    let enabled_by_project = load(&[
        user("[permissions]\nmode = \"auto\"\n"),
        project("[permissions]\nauto_enabled = true\n"),
    ]);
    assert_eq!(enabled_by_project.effective_mode, Mode::ReadOnly);

    let enabled = load(&[user(
        "[permissions]\nmode = \"auto\"\nauto_enabled = true\n",
    )]);
    assert_eq!(enabled.effective_mode, Mode::Auto);
    assert!(enabled.warnings.is_empty());
    // Low-risk unmatched write runs; high-risk unmatched write asks.
    let low = enabled.decide(&seat("claude"), "set_availability", Some("gemini"));
    assert_eq!(
        (low.verdict, low.reason),
        (Verdict::Allow, Reason::AutoLowRisk)
    );
    assert_eq!(
        enabled
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Ask
    );
}

// ---- layers and locks -----------------------------------------------------------------------------------------

#[test]
fn a_higher_layer_overrides_a_lower_one_and_the_effective_view_names_the_layer() {
    let loaded = load(&[
        user("[models.backends.local]\nmodel = \"other:7b\"\n"),
        local("[ui]\ntheme = \"dark\"\n"),
    ]);
    let view = loaded.effective();
    let find = |path: &str| {
        view.iter()
            .find(|v| v.path == path)
            .unwrap_or_else(|| panic!("{path}"))
    };
    assert_eq!(find("models.backends.local.model").layer, LayerKind::User);
    assert_eq!(find("models.backends.local.model").value, "\"other:7b\"");
    assert_eq!(find("ui.theme").layer, LayerKind::Local);
    assert_eq!(find("ui.notification_level").layer, LayerKind::Defaults);
    // Untouched sibling keys survive the merge.
    assert_eq!(
        loaded.config.models.backends["local"].endpoint,
        "http://127.0.0.1:11434"
    );
}

#[test]
fn a_locked_key_cannot_be_changed_by_a_higher_layer() {
    let refused = error(&[
        user("locked = [\"permissions.mode\"]\n[permissions]\nmode = \"read-only\"\n"),
        project("[permissions]\nmode = \"ask\"\n"),
    ]);
    assert_eq!(
        refused,
        LoadError::Locked {
            path: "permissions.mode".into(),
            locked_by: "user".into(),
            attempted_by: "project".into()
        }
    );
    assert!(
        refused
            .to_string()
            .contains("project settings set `permissions.mode`")
    );
}

#[test]
fn locking_a_table_locks_everything_under_it_and_a_layer_can_set_its_own_locked_keys() {
    // The locking layer itself may set the key it locks.
    load(&[user("locked = [\"cache\"]\n[cache]\nttl_secs = 5\n")]);
    let refused = error(&[
        user("locked = [\"cache\"]\n"),
        local("[cache]\nttl_secs = 5\n"),
    ]);
    assert!(
        matches!(refused, LoadError::Locked { ref path, .. } if path == "cache"),
        "{refused}"
    );
    // Replacing a locked leaf by a whole table is also caught.
    let refused = error(&[
        user("locked = [\"ui.visualizer.enabled\"]\n"),
        local(
            "ui = { theme = \"dark\", notification_level = \"quiet\", visualizer = { enabled = false, max_fps = 30, fallback_2d_when_model_busy = true } }\n",
        ),
    ]);
    assert!(matches!(refused, LoadError::Locked { .. }), "{refused}");
}

#[test]
fn rules_can_be_locked_so_a_higher_layer_cannot_add_any() {
    let refused = error(&[
        user("locked = [\"permissions.rules\"]\n"),
        project("[permissions]\nrules = [\"allow seat:* run_operation\"]\n"),
    ]);
    assert!(
        matches!(refused, LoadError::Locked { ref path, .. } if path == "permissions.rules"),
        "{refused}"
    );
}

#[test]
fn layers_must_come_in_order_once_each() {
    let wrong = error(&[project(""), user("")]);
    assert!(matches!(wrong, LoadError::LayerOrder { .. }), "{wrong}");
    let twice = error(&[user(""), user("")]);
    assert!(matches!(twice, LoadError::LayerOrder { .. }), "{twice}");
    let defaults = error(&[Layer::new(LayerKind::Defaults, Some(""))]);
    assert!(
        matches!(defaults, LoadError::LayerOrder { .. }),
        "{defaults}"
    );
}

#[test]
fn a_malformed_lock_list_is_refused() {
    for text in [
        "locked = \"permissions.mode\"\n",
        "locked = [1]\n",
        "locked = [\"\"]\n",
    ] {
        assert!(
            matches!(error(&[user(text)]), LoadError::BadLockList { .. }),
            "{text}"
        );
    }
}

// ---- validation -----------------------------------------------------------------------------------------------

#[test]
fn invalid_toml_names_the_layer() {
    let refused = error(&[local("this is = = not toml")]);
    assert!(
        matches!(refused, LoadError::Parse { ref layer, .. } if layer == "local"),
        "{refused}"
    );
}

#[test]
fn unknown_keys_and_wrong_types_are_errors_not_silent_defaults() {
    for text in [
        "[agent_door]\nrate_per_minut = 5\n",
        "[permissions]\nmode = \"yolo\"\n",
        "[cache]\nttl_secs = \"soon\"\n",
        "[surprise]\nx = 1\n",
    ] {
        assert!(
            matches!(error(&[user(text)]), LoadError::Invalid { .. }),
            "{text}"
        );
    }
}

#[test]
fn out_of_range_and_inconsistent_values_are_refused_with_the_setting_name() {
    for (text, fragment) in [
        (
            "[agent_door]\nrate_per_minute = 0\n",
            "agent_door.rate_per_minute",
        ),
        (
            "[agent_door]\nmax_body_bytes = 99999999\n",
            "agent_door.max_body_bytes",
        ),
        (
            "[routing]\nmin_confidence = 1.5\n",
            "routing.min_confidence",
        ),
        (
            "[routing]\ntiers = [\"software\", \"cloud\"]\n",
            "cloud_enabled",
        ),
        ("[routing]\ntiers = []\n", "at least one tier"),
        ("[routing]\ntiers = [\"local\", \"local\"]\n", "twice"),
        ("[models.roles]\nglue = \"missing\"\n", "not defined"),
        (
            "[models.backends.local]\nendpoint = \"ftp://x\"\n",
            "http://",
        ),
        ("[ui.visualizer]\nmax_fps = 0\n", "max_fps"),
        (
            "[operations]\nrun_without_approval = [\"guild_tick\"]\n",
            "operations.visible",
        ),
        (
            "[events]\nlong_poll_timeout_secs = 0\n",
            "long_poll_timeout_secs",
        ),
    ] {
        let refused = error(&[user(text)]);
        assert!(refused.to_string().contains(fragment), "{text}: {refused}");
    }
}

#[test]
fn the_cloud_tier_can_be_turned_on_in_settings_without_a_code_change() {
    let loaded = load(&[user(
        "[routing]\ntiers = [\"software\", \"local\", \"cloud\"]\ncloud_enabled = true\n",
    )]);
    assert!(loaded.config.routing.cloud_enabled);
    assert_eq!(loaded.config.routing.tiers.len(), 3);
}

// ---- reload keeps the last good configuration -----------------------------------------------------------------

#[test]
fn a_bad_reload_keeps_the_previous_good_configuration() {
    let mut manager =
        Manager::start(&[user("[agent_door]\nrate_per_minute = 10\n")]).expect("start");
    assert_eq!(manager.current().config.agent_door.rate_per_minute, 10);
    assert_eq!(manager.generation(), 1);

    let outcome = manager.reload(&[user("[agent_door]\nrate_per_minute = 0\n")]);
    assert!(!outcome.applied);
    assert!(outcome.error.is_some());
    assert_eq!(manager.current().config.agent_door.rate_per_minute, 10);
    assert_eq!(manager.generation(), 1);
    assert!(manager.last_error().is_some());

    let outcome = manager.reload(&[user("[agent_door]\nrate_per_minute = 20\n")]);
    assert!(outcome.applied);
    assert_eq!(manager.current().config.agent_door.rate_per_minute, 20);
    assert_eq!(manager.generation(), 2);
    assert!(manager.last_error().is_none());
}

#[test]
fn a_bad_reload_cannot_loosen_permissions() {
    let mut manager = Manager::start(&[]).expect("start");
    let before = manager
        .current()
        .decide(&seat("claude"), "edit_settings", None)
        .verdict;
    assert_eq!(before, Verdict::Deny);
    // The file tries to loosen permissions but also contains a typo; nothing of it may take effect.
    let outcome = manager.reload(&[user("[permissions]\nmode = \"ask\"\nauto_enable = true\n")]);
    assert!(!outcome.applied);
    assert_eq!(
        manager
            .current()
            .decide(&seat("claude"), "edit_settings", None)
            .verdict,
        Verdict::Deny
    );
}

#[test]
fn starting_with_a_bad_file_falls_back_to_the_defaults_and_keeps_the_error() {
    let manager =
        Manager::start(&[user("not = = valid")]).expect("start never fails on a bad user file");
    assert_eq!(manager.current().effective_mode, Mode::ReadOnly);
    assert!(matches!(
        manager.last_error(),
        Some(LoadError::Parse { .. })
    ));
}
