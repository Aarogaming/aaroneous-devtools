//! Strict JSON codecs for the verification packets (lane C129, phase 2): the request the hall sends, the result a node sends
//! back, and the heartbeat a node sends every few ticks.
//!
//! Everything a node or the network hands us is untrusted. Parsing is strict: the version must match, every key must be known,
//! every type must be exact (a float, a negative number or a string where a number belongs is refused), and every label and
//! digest is validated again here. `signature_verified` is deliberately **not** a wire field: only the adapter that checked the
//! signature against the registry can set it, so a node cannot vouch for itself.
//!
//! Pure functions over `serde_json::Value`: no I/O, no clock, no environment.

use crate::verification::{Node, Outcome, PACKET_VERSION, Request, Verdict, is_online};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn label(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.chars().count() <= 48
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

fn object<'a>(
    value: &'a Value,
    keys: &[&str],
    what: &str,
) -> Result<&'a Map<String, Value>, String> {
    let map = value
        .as_object()
        .ok_or_else(|| format!("{what} must be a JSON object"))?;
    if let Some(extra) = map.keys().find(|k| !keys.contains(&k.as_str())) {
        return Err(format!("{what}: unknown field {extra:?}"));
    }
    if let Some(missing) = keys.iter().find(|k| !map.contains_key(**k)) {
        return Err(format!("{what}: missing field {missing:?}"));
    }
    match map.get("v").and_then(Value::as_u64) {
        Some(v) if v == u64::from(PACKET_VERSION) => Ok(map),
        _ => Err(format!("{what}: v must be {PACKET_VERSION}")),
    }
}

fn text<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    map.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn count(map: &Map<String, Value>, key: &str, max: u64) -> Result<u32, String> {
    let n = map
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a whole number"))?;
    if n > max {
        return Err(format!("{key} must be at most {max}"));
    }
    u32::try_from(n).map_err(|_| format!("{key} is out of range"))
}

const REQUEST_KEYS: [&str; 7] = [
    "v",
    "repo",
    "head",
    "base_sha",
    "platform",
    "suite",
    "suite_digest",
];

#[must_use]
pub fn request_to_value(request: &Request) -> Value {
    json!({
        "v": PACKET_VERSION, "repo": request.repo, "head": request.head, "base_sha": request.base_sha,
        "platform": request.platform, "suite": request.suite, "suite_digest": request.suite_digest,
    })
}

/// # Errors
/// A sentence naming the first thing wrong with the packet.
pub fn request_from_value(value: &Value) -> Result<Request, String> {
    let map = object(value, &REQUEST_KEYS, "request")?;
    let request = Request {
        repo: text(map, "repo")?.to_owned(),
        head: text(map, "head")?.to_owned(),
        base_sha: text(map, "base_sha")?.to_owned(),
        platform: text(map, "platform")?.to_owned(),
        suite: text(map, "suite")?.to_owned(),
        suite_digest: text(map, "suite_digest")?.to_owned(),
    };
    match request.problem() {
        Some(problem) => Err(format!("request: {problem}")),
        None => Ok(request),
    }
}

const OUTCOME_KEYS: [&str; 8] = [
    "v",
    "request_id",
    "head",
    "platform",
    "suite_digest",
    "node",
    "attempt",
    "verdict",
];

#[must_use]
pub fn outcome_to_value(outcome: &Outcome) -> Value {
    let verdict = match outcome.verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Error => "error",
    };
    json!({
        "v": PACKET_VERSION, "request_id": outcome.request_id, "head": outcome.head, "platform": outcome.platform,
        "suite_digest": outcome.suite_digest, "node": outcome.node, "attempt": outcome.attempt, "verdict": verdict,
    })
}

/// Parse a result. `signature_verified` is supplied by the caller that checked the signature; it is never read from the packet.
///
/// # Errors
/// A sentence naming the first thing wrong with the packet.
pub fn outcome_from_value(value: &Value, signature_verified: bool) -> Result<Outcome, String> {
    let map = object(value, &OUTCOME_KEYS, "result")?;
    let hex = |key: &str, len: usize| -> Result<String, String> {
        let s = text(map, key)?;
        if s.len() == len
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(s.to_owned())
        } else {
            Err(format!("{key} must be {len} lower-case hex characters"))
        }
    };
    let platform = text(map, "platform")?;
    let node = text(map, "node")?;
    if !label(platform) || !label(node) {
        return Err("platform and node must be short lower-case labels".to_owned());
    }
    let attempt = count(map, "attempt", 1000)?;
    if attempt == 0 {
        return Err("attempt starts at 1".to_owned());
    }
    let verdict = match text(map, "verdict")? {
        "pass" => Verdict::Pass,
        "fail" => Verdict::Fail,
        "error" => Verdict::Error,
        other => return Err(format!("verdict {other:?} must be pass, fail or error")),
    };
    Ok(Outcome {
        request_id: hex("request_id", 64)?,
        head: hex("head", 40)?,
        platform: platform.to_owned(),
        suite_digest: hex("suite_digest", 64)?,
        node: node.to_owned(),
        attempt,
        verdict,
        signature_verified,
    })
}

/// A node's heartbeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beat {
    pub node: String,
    pub platforms: BTreeSet<String>,
    pub os: String,
    pub arch: String,
    /// The devtools build the node runs (a git SHA), so a stale node is visible.
    pub build: String,
    pub toolchains: BTreeMap<String, String>,
    pub slots_total: u32,
    pub slots_free: u32,
    pub queue_depth: u32,
}

const BEAT_KEYS: [&str; 10] = [
    "v",
    "node",
    "platforms",
    "os",
    "arch",
    "build",
    "toolchains",
    "slots_total",
    "slots_free",
    "queue_depth",
];

fn short_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.chars().all(|c| (' '..='~').contains(&c))
}

#[must_use]
pub fn beat_to_value(beat: &Beat) -> Value {
    json!({
        "v": PACKET_VERSION, "node": beat.node, "platforms": beat.platforms, "os": beat.os, "arch": beat.arch,
        "build": beat.build, "toolchains": beat.toolchains, "slots_total": beat.slots_total,
        "slots_free": beat.slots_free, "queue_depth": beat.queue_depth,
    })
}

/// # Errors
/// A sentence naming the first thing wrong with the packet.
pub fn beat_from_value(value: &Value) -> Result<Beat, String> {
    let map = object(value, &BEAT_KEYS, "heartbeat")?;
    let node = text(map, "node")?;
    if !label(node) {
        return Err("node must be a short lower-case label".to_owned());
    }
    let mut platforms = BTreeSet::new();
    let listed = map
        .get("platforms")
        .and_then(Value::as_array)
        .ok_or("platforms must be a list")?;
    if listed.is_empty() || listed.len() > 16 {
        return Err("platforms must list 1 to 16 entries".to_owned());
    }
    for item in listed {
        let p = item
            .as_str()
            .filter(|s| label(s))
            .ok_or("platforms entries must be short lower-case labels")?;
        if !platforms.insert(p.to_owned()) {
            return Err(format!("platforms lists {p:?} twice"));
        }
    }
    let (os, arch, build) = (text(map, "os")?, text(map, "arch")?, text(map, "build")?);
    if ![os, arch, build].iter().all(|s| short_text(s)) {
        return Err("os, arch and build must be short printable text".to_owned());
    }
    let mut toolchains = BTreeMap::new();
    let raw = map
        .get("toolchains")
        .and_then(Value::as_object)
        .ok_or("toolchains must be an object")?;
    if raw.len() > 16 {
        return Err("toolchains lists too many entries".to_owned());
    }
    for (name, version) in raw {
        let version = version
            .as_str()
            .filter(|s| short_text(s))
            .ok_or("toolchain versions must be short printable text")?;
        if !label(name) {
            return Err(format!(
                "toolchain name {name:?} must be a short lower-case label"
            ));
        }
        toolchains.insert(name.clone(), version.to_owned());
    }
    let slots_total = count(map, "slots_total", 1024)?;
    let slots_free = count(map, "slots_free", 1024)?;
    if slots_free > slots_total {
        return Err("slots_free cannot exceed slots_total".to_owned());
    }
    let queue_depth = count(map, "queue_depth", 1_000_000)?;
    Ok(Beat {
        node: node.to_owned(),
        platforms,
        os: os.to_owned(),
        arch: arch.to_owned(),
        build: build.to_owned(),
        toolchains,
        slots_total,
        slots_free,
        queue_depth,
    })
}

impl Beat {
    /// The roster entry the router uses. `last_tick` is the hall's own tick when it received this beat.
    #[must_use]
    pub fn to_node(&self, now_tick: u64, last_tick: u64, offline_after_ticks: u64) -> Node {
        Node {
            name: self.node.clone(),
            platforms: self.platforms.clone(),
            online: is_online(now_tick, last_tick, offline_after_ticks),
            free_slots: self.slots_free,
            queue_depth: self.queue_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request {
            repo: "Aarogaming/aaroneous-devtools".into(),
            head: "a".repeat(40),
            base_sha: "b".repeat(40),
            platform: "linux-x86_64".into(),
            suite: "foundation-ci".into(),
            suite_digest: "c".repeat(64),
        }
    }

    fn outcome() -> Outcome {
        Outcome {
            request_id: "d".repeat(64),
            head: "a".repeat(40),
            platform: "linux-x86_64".into(),
            suite_digest: "c".repeat(64),
            node: "ubuntu-1".into(),
            attempt: 1,
            verdict: Verdict::Pass,
            signature_verified: true,
        }
    }

    fn beat() -> Beat {
        Beat {
            node: "ubuntu-1".into(),
            platforms: BTreeSet::from(["linux-x86_64".to_owned()]),
            os: "Ubuntu 24.04".into(),
            arch: "x86_64".into(),
            build: "e".repeat(40),
            toolchains: BTreeMap::from([("rustc".to_owned(), "1.90.0".to_owned())]),
            slots_total: 2,
            slots_free: 1,
            queue_depth: 3,
        }
    }

    fn without(mut value: Value, key: &str) -> Value {
        value.as_object_mut().unwrap().remove(key);
        value
    }

    fn with(mut value: Value, key: &str, new: Value) -> Value {
        value.as_object_mut().unwrap().insert(key.to_owned(), new);
        value
    }

    #[test]
    fn every_packet_round_trips() {
        assert_eq!(
            request_from_value(&request_to_value(&request())).unwrap(),
            request()
        );
        assert_eq!(
            outcome_from_value(&outcome_to_value(&outcome()), true).unwrap(),
            outcome()
        );
        assert_eq!(beat_from_value(&beat_to_value(&beat())).unwrap(), beat());
    }

    #[test]
    fn a_node_cannot_vouch_for_its_own_signature() {
        let smuggled = with(
            outcome_to_value(&outcome()),
            "signature_verified",
            json!(true),
        );
        assert!(
            outcome_from_value(&smuggled, false)
                .unwrap_err()
                .contains("unknown field")
        );
        let parsed = outcome_from_value(&outcome_to_value(&outcome()), false).unwrap();
        assert!(
            !parsed.signature_verified,
            "the flag comes from the caller, never the packet"
        );
    }

    #[test]
    fn unknown_missing_and_misversioned_fields_are_refused_for_every_packet() {
        let packets = [
            request_to_value(&request()),
            outcome_to_value(&outcome()),
            beat_to_value(&beat()),
        ];
        for packet in packets {
            let parse = |v: &Value| -> bool {
                request_from_value(v).is_ok()
                    || outcome_from_value(v, true).is_ok()
                    || beat_from_value(v).is_ok()
            };
            assert!(parse(&packet));
            assert!(!parse(&with(packet.clone(), "extra", json!(1))));
            assert!(!parse(&with(packet.clone(), "v", json!(2))));
            assert!(!parse(&with(packet.clone(), "v", json!("1"))));
            assert!(!parse(&without(packet.clone(), "v")));
            let keys: Vec<String> = packet.as_object().unwrap().keys().cloned().collect();
            for key in keys.iter().filter(|k| *k != "v") {
                assert!(!parse(&without(packet.clone(), key)), "missing {key}");
            }
        }
        assert!(request_from_value(&json!([])).is_err());
        assert!(request_from_value(&json!("x")).is_err());
        assert!(beat_from_value(&Value::Null).is_err());
    }

    #[test]
    fn a_result_with_the_wrong_shape_is_refused() {
        let good = outcome_to_value(&outcome());
        for (key, bad) in [
            ("request_id", json!("short")),
            ("request_id", json!("D".repeat(64))),
            ("head", json!("a".repeat(39))),
            ("suite_digest", json!(7)),
            ("platform", json!("Linux")),
            ("node", json!("")),
            ("attempt", json!(0)),
            ("attempt", json!(-1)),
            ("attempt", json!(1.5)),
            ("attempt", json!("1")),
            ("attempt", json!(1_000_000)),
            ("attempt", json!(u64::MAX)),
            ("verdict", json!("PASS")),
            ("verdict", json!("ok")),
            ("verdict", json!(true)),
        ] {
            assert!(
                outcome_from_value(&with(good.clone(), key, bad.clone()), true).is_err(),
                "{key}={bad}"
            );
        }
    }

    #[test]
    fn a_request_with_the_wrong_shape_is_refused() {
        let good = request_to_value(&request());
        for (key, bad) in [
            ("repo", json!("no-slash")),
            ("head", json!("A".repeat(40))),
            ("platform", json!("linux\nwindows")),
            ("suite", json!(null)),
            ("suite_digest", json!("z".repeat(64))),
        ] {
            assert!(
                request_from_value(&with(good.clone(), key, bad.clone())).is_err(),
                "{key}={bad}"
            );
        }
    }

    #[test]
    fn a_heartbeat_with_the_wrong_shape_is_refused() {
        let good = beat_to_value(&beat());
        for (key, bad) in [
            ("node", json!("Ubuntu")),
            ("platforms", json!([])),
            ("platforms", json!(["linux-x86_64", "linux-x86_64"])),
            ("platforms", json!(["Linux"])),
            ("platforms", json!("linux-x86_64")),
            ("os", json!("")),
            ("os", json!("a\u{7}b")),
            ("build", json!("x".repeat(81))),
            ("toolchains", json!({"Rustc": "1"})),
            ("toolchains", json!({"rustc": 1})),
            ("slots_free", json!(5)),
            ("slots_total", json!(100_000)),
            ("queue_depth", json!(-1)),
            ("queue_depth", json!(2.5)),
        ] {
            assert!(
                beat_from_value(&with(good.clone(), key, bad.clone())).is_err(),
                "{key}={bad}"
            );
        }
    }

    #[test]
    fn a_heartbeat_becomes_a_roster_entry_that_goes_offline_with_the_halls_ticks() {
        let live = beat().to_node(10, 9, 3);
        assert!(live.online);
        assert_eq!((live.free_slots, live.queue_depth), (1, 3));
        assert!(!beat().to_node(20, 9, 3).online);
        assert!(live.platforms.contains("linux-x86_64"));
    }
}
