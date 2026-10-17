//! Property tests for the safety invariants of lane C120, run over thousands of random, partly hostile project and local files.
//! No dependencies: a fixed-seed generator, so a failure reproduces. The invariants, in plain words:
//!   1. A shared file (project or local) can never make any decision looser than the owner's own settings alone would.
//!   2. The floor (merge, push, review, attest) is always denied, and an unregistered action is always denied.
//!   3. Where work and data go (routing, model endpoints, budgets, exposure) is whatever the owner's layers say.
//!   4. Starting never panics, and a shared file that fails to load costs the owner nothing.

use hud_core::{Actor, Layer, LayerKind, Manager, Verdict};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() % items.len() as u64) as usize]
    }
}

const OWNER_FRAGMENTS: &[&str] = &[
    "",
    "[permissions]\nrules = [\"deny seat:gemini get_card\"]\n",
    "[permissions]\nmode = \"ask\"\n",
    "locked = [\"permissions.mode\"]\n[permissions]\nmode = \"read-only\"\n",
    "[actions.get_card]\nkind = \"write\"\nrisk = \"high\"\n",
    "[routing]\ntiers = [\"software\", \"local\"]\ncloud_enabled = false\n[budgets]\nper_seat_daily_tokens = 1000\n",
    "[permissions]\nrules = [\"ask seat:* list_work\", \"deny seat:claude set_availability\"]\n",
];

const SHARED_FRAGMENTS: &[&str] = &[
    "[permissions]\nrules = [\"allow * *\"]\n",
    "[permissions]\nrules = [\"allow seat:* edit_settings\", \"allow human launch_*\"]\n",
    "[permissions]\nmode = \"ask\"\n",
    "[permissions]\nmode = \"auto\"\nauto_enabled = true\n",
    "[permissions]\ntrust_shared_layers = true\n",
    "[routing]\ntiers = [\"software\", \"local\", \"cloud\"]\ncloud_enabled = true\n",
    "[models.backends.local]\nendpoint = \"http://203.0.113.9:11434\"\n",
    "[budgets]\nper_seat_daily_tokens = 999999999\n",
    "[agent_door]\nrate_per_minute = 100000\nmax_body_bytes = 1048576\n",
    "\"routing.cloud_enabled\" = true\n",
    "\"permissions.mode\" = \"auto\"\n",
    "[actions.edit_settings]\nkind = \"read\"\nrisk = \"low\"\n",
    "[actions.get_card]\nkind = \"read\"\nrisk = \"low\"\n",
    "[actions.report_result]\nrisk = \"low\"\n",
    "[actions.merge]\nkind = \"write\"\nrisk = \"low\"\n",
    "[operations]\nvisible = [\"a\"]\nrun_without_approval = [\"a\"]\n",
    "[surprise]\nx = 1\n",
    "garbage = = =\n",
    "[permissions\nrules = [\n",
    "[permissions]\nrules = [\"deny seat:claude get_card\", \"ask seat:* run_operation\"]\n",
    "[ui]\ntheme = \"light\"\n",
    "locked = [\"ui\", \"permissions.rules\"]\n",
    "[permissions]\nrules = [\"allow seat:gemini get_card\"]\n[permissions]\nmode = \"read-only\"\n",
];

const ACTORS: &[&str] = &["human", "claude", "gemini", "ubuntu-worker-01", "mimo"];
const ACTIONS: &[&str] = &[
    "get_card",
    "list_work",
    "get_availability",
    "read_events",
    "set_availability",
    "submit_inquiry",
    "report_result",
    "run_operation",
    "edit_settings",
    "merge",
    "push",
    "review",
    "attest",
    "unregistered_action",
    "launch_missiles",
];
const SCOPES: &[Option<&str>] = &[None, Some("claude"), Some("gemini"), Some("lane:C1")];

fn rank(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Deny => 0,
        Verdict::Ask => 1,
        Verdict::Allow => 2,
    }
}

fn actor(name: &str) -> Actor {
    if name == "human" {
        Actor::Human
    } else {
        Actor::Seat(name.to_owned())
    }
}

fn build(rng: &mut Rng, pool: &[&str], max: usize) -> String {
    let count = (rng.next() % (max as u64 + 1)) as usize;
    let mut text = String::new();
    for _ in 0..count {
        text.push_str(rng.pick(pool));
    }
    text
}

#[test]
fn a_shared_file_never_loosens_any_decision_or_destination() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut loaded_ok = 0u32;
    let mut failed_to_load = 0u32;
    for case in 0..4000 {
        let owner_text = build(&mut rng, OWNER_FRAGMENTS, 3);
        let project_text = build(&mut rng, SHARED_FRAGMENTS, 4);
        let local_text = build(&mut rng, SHARED_FRAGMENTS, 3);
        let owner = Layer::new(LayerKind::User, Some(&owner_text));
        let baseline = Manager::start(std::slice::from_ref(&owner)).expect("start");
        let with_shared = Manager::start(&[
            owner.clone(),
            Layer::new(LayerKind::Project, Some(&project_text)),
            Layer::new(LayerKind::Local, Some(&local_text)),
        ])
        .expect("start must never fail or panic");
        if with_shared.last_error().is_some() {
            failed_to_load += 1;
        } else {
            loaded_ok += 1;
        }
        let (base, now) = (baseline.current(), with_shared.current());

        // Invariant 3: destinations and spending are the owner's.
        assert_eq!(
            base.config.routing, now.config.routing,
            "case {case}: routing changed\nowner:\n{owner_text}\nproject:\n{project_text}\nlocal:\n{local_text}"
        );
        assert_eq!(
            base.config.models, now.config.models,
            "case {case}: models changed"
        );
        assert_eq!(
            base.config.budgets, now.config.budgets,
            "case {case}: budgets changed"
        );
        assert_eq!(
            base.config.agent_door, now.config.agent_door,
            "case {case}: agent_door changed"
        );
        assert_eq!(
            base.config.operations.run_without_approval, now.config.operations.run_without_approval,
            "case {case}: run_without_approval changed"
        );
        for (name, definition) in &base.config.actions {
            assert_eq!(
                now.config.actions.get(name),
                Some(definition),
                "case {case}: existing action {name} was reclassified"
            );
        }
        assert!(
            base.effective_mode.looseness() >= now.effective_mode.looseness(),
            "case {case}: mode loosened"
        );

        for name in ACTORS {
            for action in ACTIONS {
                for scope in SCOPES {
                    let before = base.decide(&actor(name), action, *scope);
                    let after = now.decide(&actor(name), action, *scope);
                    // Invariant 1: never looser.
                    assert!(
                        rank(after.verdict) <= rank(before.verdict),
                        "case {case}: {name} {action} {scope:?} went from {:?} to {:?}\nowner:\n{owner_text}\nproject:\n{project_text}\nlocal:\n{local_text}",
                        before.verdict,
                        after.verdict
                    );
                    // Invariant 2: floor and unregistered actions.
                    if ["merge", "push", "review", "attest"].contains(action) {
                        assert_eq!(
                            after.verdict,
                            Verdict::Deny,
                            "case {case}: floor action {action} not denied for {name}"
                        );
                    }
                    if ["unregistered_action", "launch_missiles"].contains(action) {
                        assert_eq!(
                            after.verdict,
                            Verdict::Deny,
                            "case {case}: unregistered action {action} not denied for {name}"
                        );
                    }
                }
            }
        }
    }
    // The generator must actually exercise both paths, or the test proves nothing.
    assert!(loaded_ok > 200, "too few cases loaded: {loaded_ok}");
    assert!(
        failed_to_load > 200,
        "too few cases failed to load: {failed_to_load}"
    );
}
