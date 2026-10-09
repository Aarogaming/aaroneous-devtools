//! The task-card dispatcher: one card per agent per tick. Port of `next_task.build_cards` and the helpers it calls
//! (`card_for`, `enrich_card`, `recipe_for`, `implementers_field`, `audit_item`, the healthy-idle and standby cards),
//! checked by the 18 scenario vectors, the `audit` vector and the differential test.
//!
//! Snapshot, items and cards are dynamic JSON (`serde_json::Value`) read with Python's rules. Item identity (`id()` in
//! Python) is the index into the ladder.

use crate::checklist::py_repr;
use crate::gate::truthy;
use crate::work::{item_key, py_str, work_items};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

const AUDIT_EVERY: i64 = 6;
const LEASE_TTL: i64 = 6;
const DEFAULT_ROLES: [&str; 4] = ["implementer", "reviewer", "auditor", "skeptic"];
const LENSES: [&str; 4] = ["direct", "skeptic", "minimal", "handoff"];
const OBSERVABLE_KINDS: [&str; 4] = [
    "resolve_threads",
    "address_review",
    "identify_implementers",
    "not_mergeable",
];

fn lens_text(lens: &str) -> &'static str {
    match lens {
        "direct" => "Do the task as stated.",
        "skeptic" => {
            "Earlier handouts of this item did not finish. Assume the obvious path hides a problem: find the two weakest claims in the diff or thread and verify each by running something, not reading."
        }
        "minimal" => {
            "Produce the smallest artifact that changes the state (one comment, one command output, one attestation) and stop. Do not widen scope."
        }
        _ => {
            "Write for a seat with no memory of this PR: starting state, what you ran with exit codes, what remains. Leave it continuable."
        }
    }
}

type Item = Map<String, Value>;

fn s(item: &Item, key: &str) -> String {
    item.get(key).map(py_str).unwrap_or_default()
}

fn text<'a>(item: &'a Item, key: &str, default: &'a str) -> &'a str {
    item.get(key).and_then(Value::as_str).unwrap_or(default)
}

/// `shlex.quote`.
fn shlex_quote(text: &str) -> String {
    if text.is_empty() {
        return "''".to_owned();
    }
    let safe = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if text.chars().all(safe) {
        return text.to_owned();
    }
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn pr_of(item: &Item) -> Option<&Value> {
    item.get("pr").filter(|v| truthy(v))
}

fn sha_or(sha: &str) -> String {
    if sha.is_empty() {
        "<merge-base>".to_owned()
    } else {
        sha.to_owned()
    }
}

fn implementers_field(item: &Item, base_sha: &str, head: &str) -> String {
    let names: Vec<Value> = item
        .get("implementers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !names.is_empty() {
        return py_repr(&Value::Array(names));
    }
    format!(
        "<NOT KNOWN, so find them: git log --format='%(trailers:key=Co-Authored-By,key=Contribution-ID,only=true,unfold=true)' {}..{head}, plus a seat-named branch prefix; the list must not be empty, so if git names no seat, post a comment saying the implementers are unknown instead of an accept block>",
        sha_or(base_sha)
    )
}

fn recipe_for(item: &Item) -> Vec<Value> {
    let Some(pr) = pr_of(item).map(py_str) else {
        return Vec::new();
    };
    let head = s(item, "head");
    let base_ref = item
        .get("base_ref")
        .map_or_else(|| "main".to_owned(), py_str);
    let base_sha = s(item, "base_sha");
    let lines: Vec<String> = match text(item, "kind", "") {
        "needs_review" => vec![
            format!("git fetch --prune origin && git fetch origin pull/{pr}/head   # head must be {head}"),
            format!("git diff --check {}...{head}   # expect exit 0", sha_or(&base_sha)),
            format!(
                "git merge-tree --write-tree {} {head}   # expect exit 0 (clean trial merge)",
                shlex_quote(&format!("origin/{base_ref}"))
            ),
            "run the PR's own tests if it changes code (python -m unittest discover -s scripts -p 'test_*.py')".to_owned(),
            format!(
                "post ONE attestation block with fields: repo, pr={pr}, head={head}, base_sha={}, scope=whole, reviewer=<your seat>, implementers={}, verdict",
                sha_or(&base_sha),
                implementers_field(item, &base_sha, &head)
            ),
        ],
        "repair_verdict" => vec![
            "read why each of your comments was rejected (REJECTED lines above)".to_owned(),
            format!(
                "post a new comment, built from a file so backslash sequences are not mangled, holding ONE fenced block tagged attestation whose body is a JSON object (not YAML) with exactly these fields: repo (this repository), pr={pr}, head={head}, base_sha={}, scope=whole, reviewer=<your seat>, implementers={}, verdict",
                sha_or(&base_sha),
                implementers_field(item, &base_sha, &head)
            ),
            "keep your analysis as prose above the block".to_owned(),
        ],
        "identify_implementers" => vec![
            format!(
                "git log --format='%(trailers:key=Co-Authored-By,key=Contribution-ID,only=true,unfold=true)' {}..{head}   # trailers naming a seat",
                sha_or(&base_sha)
            ),
            "read the head branch name: a seat-named prefix such as codex/ names its owner".to_owned(),
            "post a comment naming each implementing seat with that evidence, or saying none can be established; the author may add a Co-Authored-By trailer in a new commit".to_owned(),
        ],
        "address_review" => vec![
            "read the reviewer's latest comment on the PR and list each finding it names".to_owned(),
            "fix each finding in a commit, or reply with the evidence that answers it".to_owned(),
            "push the new head and say so on the PR; the reviewer re-reviews the new head".to_owned(),
        ],
        "resolve_threads" => vec![
            "list the unresolved threads on the PR with their replies".to_owned(),
            "for each thread, read the reply and open the commit it names".to_owned(),
            "resolve the thread only if it is actually addressed; otherwise reply with what is still missing".to_owned(),
        ],
        "fix_threads" => vec![
            "list the unresolved threads on the PR".to_owned(),
            "fix each in a commit".to_owned(),
            "reply on each thread naming the commit; resolve only what is addressed".to_owned(),
        ],
        _ => Vec::new(),
    };
    lines.into_iter().map(Value::String).collect()
}

fn task_and_acceptance(item: &Item, reference: &str) -> Result<(String, String), String> {
    let kind = text(item, "kind", "");
    let blocker = item
        .get("blocker")
        .filter(|v| truthy(v))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let verdict = blocker
        .get("verdict")
        .map_or_else(|| "rejecting verdict".to_owned(), py_str);
    let by = blocker
        .get("by")
        .map_or_else(|| "a reviewer".to_owned(), py_str);
    let detail = text(item, "detail", "");
    let pair = |a: String, b: &str| Ok((a, b.to_owned()));
    match kind {
        "fix_threads" => pair(
            format!("Fix and answer every unresolved review thread on {reference}"),
            "every thread has a reply naming the commit that addresses it, and is resolved only if addressed",
        ),
        "address_review" => pair(
            format!("Answer the standing {verdict} by {by} on {reference}"),
            "a new head that fixes what the reviewer found, or a reply naming each finding and the commit or evidence that answers it; the card clears when the new head is reviewed",
        ),
        "identify_implementers" => pair(
            format!(
                "Identify who implemented {reference} from git, or say it cannot be established"
            ),
            "a comment naming each implementing seat with the git evidence (trailer or branch), or stating that none can be established; until the implementers are known no review, fix or resolve card is issued for this PR",
        ),
        "resolve_threads" => pair(
            format!(
                "Check the replies on every unresolved review thread of {reference} and resolve each one that is addressed"
            ),
            "every thread is resolved after you checked the commit its reply names, or carries your reply saying what is still missing",
        ),
        "needs_review" => pair(
            format!(
                "Post an independent whole-diff review of {reference} at its current head, with an attestation block"
            ),
            "a packet with commands run and exit codes, plus one `attestation` JSON block; review_coverage.py then reads covered=True or a stated reason",
        ),
        "not_mergeable" => pair(
            format!("Make {reference} mergeable: {detail}"),
            "mergeable_state is CLEAN at the new head; new head stated, earlier reviews marked as not carrying",
        ),
        "ready_to_merge" => pair(
            format!("Prepare the owner merge brief for {reference}"),
            "a one-command merge line with the full head SHA, the evidence for each merge condition, and the order relative to other ready PRs",
        ),
        "draft" => pair(
            format!(
                "Ask the author, or prepare the one-line command, to mark {reference} ready for review"
            ),
            "the PR is no longer a draft, or a comment names who must flip it",
        ),
        "repair_verdict" => pair(
            format!(
                "Repair your rejected verdict on {reference}: the review you posted did not count"
            ),
            "review_coverage.py reads a structured verdict from you at the current head, or states a different reason",
        ),
        "stale" => pair(
            format!("Nudge or triage {reference}: {detail}"),
            "a comment with the specific missing step, the owner of that step and its acceptance condition",
        ),
        _ => {
            let task = item
                .get("task")
                .map(py_str)
                .ok_or("backlog item without task")?;
            Ok((
                task,
                text(
                    item,
                    "acceptance",
                    "a retained artifact (commit, comment or report) with evidence",
                )
                .to_owned(),
            ))
        }
    }
}

fn card_for(name: &str, role: &str, tick: i64, item: &Item) -> Result<Map<String, Value>, String> {
    let kind = text(item, "kind", "");
    let pr = pr_of(item);
    let reference = match pr {
        Some(p) => format!("PR #{}", py_str(p)),
        None => item.get("id").map_or_else(|| "backlog".to_owned(), py_str),
    };
    let (task, acceptance) = task_and_acceptance(item, &reference)?;
    let head_lower = item
        .get("head")
        .map_or_else(String::new, py_str)
        .to_lowercase();
    let marker = pr.map_or_else(String::new, |p| {
        format!("[done:{kind}:{}:{head_lower}]", py_str(p))
    });
    let suffix = match pr {
        Some(p) if !OBSERVABLE_KINDS.contains(&kind) => format!(
            " Start your comment with the receipt [done:{kind}:{}:{head_lower}] (the full 40-character head) and name your seat on the first line, so this task is not re-issued; if you could not complete it, do not use that receipt and say what blocked you.",
            py_str(p)
        ),
        _ => String::new(),
    };
    let mut card = Map::new();
    card.insert("agent".into(), json!(name));
    card.insert("role".into(), json!(role));
    card.insert("tick".into(), json!(tick));
    card.insert(
        "priority".into(),
        item.get("rank").cloned().unwrap_or(json!(9)),
    );
    card.insert("task".into(), json!(task));
    card.insert(
        "why_now".into(),
        item.get("detail")
            .cloned()
            .unwrap_or_else(|| json!("ranked next on the backlog")),
    );
    card.insert(
        "inputs".into(),
        json!({"pr": item.get("pr").cloned().unwrap_or(Value::Null), "kind": kind}),
    );
    card.insert("marker".into(), json!(marker));
    card.insert("acceptance".into(), json!(format!("{acceptance}{suffix}")));
    card.insert(
        "if_blocked".into(),
        json!("say exactly which step is blocked and by whom, then take the next item on the ladder; do not report 'pending' as the outcome"),
    );
    Ok(card)
}

fn healthy_idle_card(name: &str, role: &str, tick: i64, state: &Value) -> Map<String, Value> {
    let prs: Vec<&Value> = state
        .get("prs")
        .and_then(Value::as_array)
        .map(|l| l.iter().collect())
        .unwrap_or_default();
    let threads: i64 = prs
        .iter()
        .map(|p| {
            p.get("threads_unresolved")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        })
        .sum();
    let mut card = Map::new();
    card.insert("agent".into(), json!(name));
    card.insert("role".into(), json!(role));
    card.insert("tick".into(), json!(tick));
    card.insert("priority".into(), json!(9));
    card.insert(
        "task".into(),
        json!("Healthy idle: no action required this tick"),
    );
    card.insert(
        "why_now".into(),
        json!("verified healthy: nothing urgent and no backlog"),
    );
    card.insert("inputs".into(), json!({"pr": null, "kind": "healthy_idle"}));
    card.insert(
        "evidence".into(),
        json!({
            "open_prs": prs.len(),
            "covered": prs.iter().filter(|p| truthy(p.get("covered").unwrap_or(&Value::Null))).count(),
            "unresolved_threads": threads,
            "not_clean": prs.iter().filter(|p| p.get("mergeable_state").and_then(Value::as_str) != Some("CLEAN")).count(),
            "backlog": state.get("backlog").and_then(Value::as_array).map_or(0, Vec::len),
        }),
    );
    card.insert(
        "acceptance".into(),
        json!("no artifact required; do not invent work or notify"),
    );
    card.insert(
        "if_blocked".into(),
        json!("not applicable; if state changes the next card will say so; do not report 'pending' as the outcome"),
    );
    card.insert("owner_blocked".into(), json!(false));
    card
}

fn standby_card(name: &str, role: &str, tick: i64) -> Map<String, Value> {
    let mut card = Map::new();
    card.insert("agent".into(), json!(name));
    card.insert("role".into(), json!(role));
    card.insert("tick".into(), json!(tick));
    card.insert("priority".into(), json!(9));
    card.insert(
        "task".into(),
        json!("Standby: every open item is currently leased to another seat"),
    );
    card.insert(
        "why_now".into(),
        json!("no free item; duplicating work or inventing a task would be noise"),
    );
    card.insert("inputs".into(), json!({"pr": null, "kind": "standby"}));
    card.insert("marker".into(), json!(""));
    card.insert(
        "acceptance".into(),
        json!("no artifact required; take the next free card when a lease completes or expires"),
    );
    card.insert(
        "if_blocked".into(),
        json!("not applicable; do not report 'pending' as the outcome"),
    );
    card
}

fn audit_item(state: &Value) -> Option<Item> {
    let merged = state.get("last_merged").filter(|v| truthy(v))?;
    let repo = state.get("repo").filter(|v| truthy(v))?;
    let first12 = |v: Option<&Value>| {
        py_str(v.unwrap_or(&json!("")))
            .chars()
            .take(12)
            .collect::<String>()
    };
    let subject = format!(
        "{}#{} (merge commit {}, head {})",
        py_str(repo),
        merged.get("number").map_or_else(String::new, py_str),
        first12(merged.get("merge_commit")),
        first12(merged.get("head"))
    );
    let mut item = Item::new();
    item.insert("kind".into(), json!("audit"));
    item.insert("id".into(), json!("audit-newest-merged"));
    item.insert("rank".into(), json!(8));
    item.insert(
        "task".into(),
        json!(format!(
            "Re-run review_coverage.py against {subject} and compare its structured verdict with the prose verdicts in its comments; report any disagreement"
        )),
    );
    item.insert("acceptance".into(), json!("a comment or report listing agreements and disagreements, each with the comment id and head SHA"));
    item.insert(
        "detail".into(),
        json!(format!(
            "periodic audit (every {AUDIT_EVERY} ticks): re-verify healthy state"
        )),
    );
    Some(item)
}

fn enrich_card(
    card: &mut Map<String, Value>,
    item: &Item,
    name: &str,
    state: &Value,
    renewed: bool,
    upcoming: &str,
) {
    let empty = json!({});
    let history = state.get("history").unwrap_or(&empty);
    let pr = pr_of(item);
    let key = pr.map(|p| item_key(text(item, "kind", ""), p, &s(item, "head")));
    let handed = key
        .as_ref()
        .and_then(|k| {
            history
                .get("item_attempts")
                .and_then(|a| a.get(k.as_str()))
                .and_then(Value::as_i64)
        })
        .unwrap_or(0);
    let index = if renewed { (handed - 1).max(0) } else { handed };
    let lens = LENSES[usize::try_from(index.rem_euclid(4)).unwrap_or(0)];
    card.insert("lens".into(), json!(lens));
    card.insert("lens_text".into(), json!(lens_text(lens)));
    card.insert("attempt".into(), json!(index + 1));
    if let Some(p) = pr {
        let head = s(item, "head");
        let head8: String = head.chars().take(8).collect();
        let head12: String = head.chars().take(12).collect();
        let base_ref = item
            .get("base_ref")
            .map_or_else(|| "main".to_owned(), py_str);
        let size = item
            .get("size")
            .filter(|v| truthy(v))
            .map_or_else(|| "size unknown".to_owned(), py_str);
        let mergeable = item
            .get("mergeable_state")
            .filter(|v| truthy(v))
            .map_or_else(|| "?".to_owned(), py_str);
        let implementers: Vec<String> = item
            .get("implementers")
            .and_then(Value::as_array)
            .map(|l| l.iter().map(py_str).collect())
            .unwrap_or_default();
        let joined = if implementers.is_empty() {
            "unknown".to_owned()
        } else {
            implementers.join(",")
        };
        card.insert(
            "state".into(),
            json!(format!("PR #{} head {head12} on {base_ref}; {size}; mergeable={mergeable}; implementers={joined}", py_str(p))),
        );
        let last = history
            .get("last_head")
            .and_then(|m| m.get(format!("{name}|{}", py_str(p))));
        let new_since = match last {
            None => "first time this seat is handed this PR".to_owned(),
            Some(l) if py_str(l) != head8 => {
                format!(
                    "head moved {} -> {head8} since your last card for this PR; earlier accepts do not carry",
                    py_str(l)
                )
            }
            Some(_) => format!(
                "same head {head8} as your last card; do not repeat the same output, use the lens below"
            ),
        };
        card.insert("new_since_last".into(), json!(new_since));
    } else {
        card.insert(
            "state".into(),
            json!(item.get("detail").map_or_else(String::new, py_str)),
        );
        card.insert("new_since_last".into(), json!(""));
    }
    card.insert("recipe".into(), Value::Array(recipe_for(item)));
    let reject_view = |list: &Value| -> Value {
        Value::Array(
            list.as_array()
                .map(|l| {
                    l.iter()
                        .map(|r| {
                            json!({
                                "comment_id": r.get("comment_id").cloned().unwrap_or(Value::Null),
                                "reason": r.get("reason").cloned().unwrap_or(Value::Null),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        )
    };
    if let Some(rejected) = item.get("rejected").filter(|v| truthy(v)) {
        card.insert("rejected".into(), reject_view(rejected));
    }
    if let Some(unattributed) = item.get("rejected_unattributed").filter(|v| truthy(v)) {
        if text(item, "kind", "") == "needs_review" {
            card.insert("rejected_note".into(), reject_view(unattributed));
        }
    }
    card.insert("then".into(), json!(upcoming));
}

fn key_of(item: &Item) -> Option<String> {
    pr_of(item).map(|p| item_key(text(item, "kind", ""), p, &s(item, "head")))
}

/// One card per agent for this tick.
///
/// # Errors
/// Returns a message where the Python version would raise.
pub fn build_cards(state: &Value, tick: i64) -> Result<Vec<Value>, String> {
    let agents: Vec<&Value> = state
        .get("agents")
        .and_then(Value::as_array)
        .ok_or("missing agents")?
        .iter()
        .collect();
    let names: Vec<String> = agents
        .iter()
        .map(|a| {
            a.get("name")
                .map(py_str)
                .ok_or_else(|| "agent without name".to_owned())
        })
        .collect::<Result<_, _>>()?;
    let roles_of = |a: &Value| -> Vec<String> {
        match a
            .get("roles")
            .filter(|v| truthy(v))
            .and_then(Value::as_array)
        {
            Some(list) => list.iter().map(py_str).collect(),
            None => DEFAULT_ROLES.iter().map(|r| (*r).to_owned()).collect(),
        }
    };
    let urgent: Vec<Item> = work_items(state)?
        .into_iter()
        .filter_map(|v| {
            if let Value::Object(m) = v {
                Some(m)
            } else {
                None
            }
        })
        .filter(|i| text(i, "kind", "") != "stacked")
        .collect();
    let substantive: Vec<&Item> = urgent
        .iter()
        .filter(|i| text(i, "kind", "") != "stale")
        .collect();
    let only_owner_left = !substantive.is_empty()
        && substantive
            .iter()
            .all(|i| matches!(text(i, "kind", ""), "ready_to_merge" | "draft"));
    let mut ladder: Vec<Item> = urgent.clone();
    for backlog in state
        .get("backlog")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let mut item = backlog
            .as_object()
            .cloned()
            .ok_or("backlog item is not an object")?;
        item.insert("kind".into(), json!("backlog"));
        item.insert("rank".into(), json!(7));
        ladder.push(item);
    }
    if ladder.is_empty() {
        match audit_item(state) {
            Some(item) if tick.rem_euclid(AUDIT_EVERY) == 0 => ladder = vec![item],
            _ => {
                let mut cards = Vec::new();
                for (i, agent) in agents.iter().enumerate() {
                    let roles = roles_of(agent);
                    let role =
                        &roles[usize::try_from((tick + i as i64).rem_euclid(roles.len() as i64))
                            .unwrap_or(0)];
                    cards.push(Value::Object(healthy_idle_card(
                        &names[i], role, tick, state,
                    )));
                }
                return Ok(cards);
            }
        }
    }
    let empty = json!({});
    let history = state.get("history").unwrap_or(&empty);
    let lease_ttl = state
        .get("lease_ttl")
        .and_then(Value::as_i64)
        .unwrap_or(LEASE_TTL);
    let raw_leases: BTreeMap<String, Value> = state
        .get("leases")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let allowed = |item: &Item, name: &str| -> bool {
        let kind = text(item, "kind", "");
        if matches!(kind, "needs_review" | "resolve_threads") {
            let named = |v: &Value| v.as_str() == Some(name);
            let implementer = item
                .get("implementers")
                .and_then(Value::as_array)
                .is_some_and(|l| l.iter().any(named));
            if implementer || item.get("seat").is_some_and(named) {
                return false;
            }
        }
        if matches!(kind, "fix_threads" | "repair_verdict" | "address_review") {
            let owner = item.get("owner_seat").cloned().unwrap_or(Value::Null);
            let blank = owner.is_null() || owner.as_str() == Some("");
            if !blank && owner.as_str() != Some(name) {
                return false;
            }
        }
        true
    };
    let burned = |item: &Item, name: &str| -> bool {
        key_of(item).is_some_and(|k| {
            history
                .get("expired")
                .and_then(|e| e.get(format!("{name}|{k}")))
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        })
    };
    let someone_else_can = |item: &Item, name: &str| -> bool {
        names
            .iter()
            .any(|other| other != name && allowed(item, other) && !burned(item, other))
    };
    let score = |name: &str| -> i64 {
        let rec = history.get("agents").and_then(|a| a.get(name));
        let part = |k: &str| {
            rec.and_then(|r| r.get(k))
                .and_then(Value::as_i64)
                .unwrap_or(0)
        };
        part("done") - part("expired")
    };
    // leases whose holder is no longer allowed to hold the item are ignored
    let leases: BTreeMap<String, Value> = raw_leases
        .into_iter()
        .filter(|(k, v)| {
            let holder = v.get("agent").map(py_str).unwrap_or_default();
            !ladder
                .iter()
                .any(|i| key_of(i).as_deref() == Some(k.as_str()) && !allowed(i, &holder))
        })
        .collect();
    let mut order: Vec<usize> = (0..agents.len()).collect();
    order.sort_by_key(|&i| (-score(&names[i]), i));
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    let mut cards: Vec<(usize, Value)> = Vec::new();
    for index in order {
        let roles = roles_of(agents[index]);
        let role = roles
            [usize::try_from((tick + index as i64).rem_euclid(roles.len() as i64)).unwrap_or(0)]
        .clone();
        let name = names[index].clone();
        let holder_of = |item: &Item| -> Option<String> {
            key_of(item)
                .and_then(|k| leases.get(&k))
                .map(|l| l.get("agent").map(py_str).unwrap_or_default())
        };
        let leased_to_others: BTreeSet<usize> = ladder
            .iter()
            .enumerate()
            .filter(|(_, i)| holder_of(i).is_some_and(|h| h != name))
            .map(|(n, _)| n)
            .collect();
        let eligible: Vec<usize> = ladder
            .iter()
            .enumerate()
            .filter(|(n, i)| {
                !leased_to_others.contains(n)
                    && allowed(i, &name)
                    && !(burned(i, &name) && someone_else_can(i, &name))
            })
            .map(|(n, _)| n)
            .collect();
        let held: Vec<usize> = eligible
            .iter()
            .copied()
            .filter(|&n| holder_of(&ladder[n]).is_some_and(|h| h == name))
            .collect();
        let rank_of = |n: usize| ladder[n].get("rank").and_then(Value::as_i64).unwrap_or(9);
        let chosen: usize = if let Some(&first) = held.first() {
            first
        } else {
            let free: Vec<usize> = eligible
                .iter()
                .copied()
                .filter(|n| !taken.contains(n))
                .collect();
            let Some(top) = free.iter().map(|&n| rank_of(n)).min() else {
                let mut card = standby_card(&name, &role, tick);
                card.insert("owner_blocked".into(), json!(only_owner_left));
                cards.push((index, Value::Object(card)));
                continue;
            };
            let tier: Vec<usize> = free
                .iter()
                .copied()
                .filter(|&n| rank_of(n) == top)
                .collect();
            tier[usize::try_from((tick + index as i64).rem_euclid(tier.len() as i64)).unwrap_or(0)]
        };
        taken.insert(chosen);
        let item = &ladder[chosen];
        let mut card = card_for(&name, &role, tick, item)?;
        card.insert("owner_blocked".into(), json!(only_owner_left));
        let key = key_of(item);
        let later: Vec<usize> = (0..ladder.len())
            .filter(|n| {
                !taken.contains(n) && !leased_to_others.contains(n) && allowed(&ladder[*n], &name)
            })
            .collect();
        let upcoming = match later.first().map(|&n| &ladder[n]) {
            Some(next) if pr_of(next).is_some() => {
                format!(
                    "{} on PR #{}",
                    text(next, "kind", ""),
                    next.get("pr").map(py_str).unwrap_or_default()
                )
            }
            _ => "no further item is free for you; stand by".to_owned(),
        };
        let lease = key.as_ref().and_then(|k| leases.get(k));
        let renewed =
            lease.is_some_and(|l| l.get("agent").map(py_str).as_deref() == Some(name.as_str()));
        enrich_card(&mut card, item, &name, state, renewed, &upcoming);
        if let Some(k) = key {
            let lease = leases.get(&k);
            card.insert(
                "lease".into(),
                json!({
                    "key": k,
                    "expires_tick": lease.map_or_else(|| json!(tick + lease_ttl), |l| l.get("expires").cloned().unwrap_or(Value::Null)),
                    "renewed": lease.is_some(),
                }),
            );
        }
        cards.push((index, Value::Object(card)));
    }
    cards.sort_by_key(|(i, _)| *i);
    Ok(cards.into_iter().map(|(_, c)| c).collect())
}
