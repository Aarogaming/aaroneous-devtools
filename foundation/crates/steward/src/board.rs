//! Pure-Rust round table discussion transport and snapshot verification (lane C145).
//!
//! Replaces the Python `board.py` service. Pure functions and data structures representing:
//! - Entry parsing, bound validation (actor, kind, length, evidence).
//! - Deterministic snapshot creation, reverse cursor pagination, and storage budget calculation.
//! - Atomic append ledger modeling and SHA-256 contribution verification.

use std::fmt::Write;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const LIMIT: usize = 16384;
pub const PAGE_MAX: usize = 128;
pub const MAX_STORAGE_BYTES_CEILING: u64 = 1024 * 1024 * 1024; // 1 GB
pub const MIN_STORAGE_BYTES_FLOOR: u64 = 32768; // 32 KB

pub const VALID_KINDS: &[&str] = &["idea", "question", "concern", "evidence", "synthesis"];

pub const VALID_ACTORS: &[&str] = &[
    "Owner",
    "Codex",
    "Codex host",
    "Codex compliance",
    "Codex coordination",
    "Codex reuse research",
    "Codex architecture research",
    "Claude",
    "MiMo (OpenCode)",
    "Gemini",
    "Local Qwen",
    "Guest",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardConfig {
    pub schema_version: u32,
    pub max_storage_bytes: u64,
    pub port: u16,
    pub restart_delay_seconds: Option<u64>,
    pub restart_window_seconds: Option<u64>,
    pub max_restarts: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionLifecycle {
    pub state: String,
    pub closes_at: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionDocument {
    pub session_id: String,
    pub question: String,
    pub deadline_epoch: Option<f64>,
    pub lifecycle: Option<SessionLifecycle>,
    pub participants: Vec<Value>,
    pub model_dispatch: bool,
    pub acceptance: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoardEntry {
    pub id: u64,
    pub actor: String,
    pub kind: String,
    pub text: String,
    pub evidence: String,
    pub received_at_epoch: f64,
    pub attribution: String,
    pub accepted: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ContributionPayload {
    pub actor: String,
    pub kind: String,
    pub text: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendReceipt {
    pub id: u64,
    pub sha256: String,
    pub accepted: bool,
    pub contract_version: u32,
}

impl BoardEntry {
    /// Serializes the entry with deterministic insertion key ordering matching `board.py`:
    /// `actor, kind, text, evidence, id, received_at_epoch, attribution, accepted, status`.
    #[must_use]
    pub fn to_ordered_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("actor".to_string(), Value::String(self.actor.clone()));
        map.insert("kind".to_string(), Value::String(self.kind.clone()));
        map.insert("text".to_string(), Value::String(self.text.clone()));
        map.insert("evidence".to_string(), Value::String(self.evidence.clone()));
        map.insert("id".to_string(), Value::Number(self.id.into()));
        if let Some(n) = serde_json::Number::from_f64(self.received_at_epoch) {
            map.insert("received_at_epoch".to_string(), Value::Number(n));
        } else {
            map.insert("received_at_epoch".to_string(), Value::Null);
        }
        map.insert(
            "attribution".to_string(),
            Value::String(self.attribution.clone()),
        );
        map.insert("accepted".to_string(), Value::Bool(self.accepted));
        map.insert("status".to_string(), Value::String(self.status.clone()));
        Value::Object(map)
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        self.to_ordered_value()
    }

    /// # Errors
    /// Returns problem string if JSON shape or fields are invalid.
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let obj = v.as_object().ok_or("entry must be an object")?;
        Ok(Self {
            id: obj
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("missing or invalid id")?,
            actor: obj
                .get("actor")
                .and_then(Value::as_str)
                .ok_or("missing or invalid actor")?
                .to_string(),
            kind: obj
                .get("kind")
                .and_then(Value::as_str)
                .ok_or("missing or invalid kind")?
                .to_string(),
            text: obj
                .get("text")
                .and_then(Value::as_str)
                .ok_or("missing or invalid text")?
                .to_string(),
            evidence: obj
                .get("evidence")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            received_at_epoch: obj
                .get("received_at_epoch")
                .and_then(Value::as_f64)
                .ok_or("missing or invalid received_at_epoch")?,
            attribution: obj
                .get("attribution")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            accepted: obj
                .get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            status: obj
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }
}

pub fn validate_config(config: &BoardConfig) -> Result<(), &'static str> {
    if config.schema_version != 2 {
        return Err("unsupported server configuration schema_version");
    }
    if config.max_storage_bytes < MIN_STORAGE_BYTES_FLOOR
        || config.max_storage_bytes > MAX_STORAGE_BYTES_CEILING
    {
        return Err("storage budget out of bounds");
    }
    if config.port < 1024 {
        return Err("explicit local unprivileged port required");
    }
    Ok(())
}

pub fn evaluate_session_state(
    session: &SessionDocument,
    clock_now: f64,
    legacy_deadline: Option<f64>,
) -> Result<(String, Option<f64>, bool), &'static str> {
    let (state, mut closes_at) = if let Some(ref lc) = session.lifecycle {
        (lc.state.clone(), lc.closes_at)
    } else {
        ("open".to_string(), session.deadline_epoch)
    };

    if state != "open" && state != "paused" && state != "closed" {
        return Err("unsupported session state");
    }

    if let Some(ld) = legacy_deadline {
        closes_at = Some(ld);
    }

    if let Some(deadline) = closes_at {
        if !deadline.is_finite() {
            return Err("invalid session deadline");
        }
    }

    let closed = state != "open" || closes_at.is_some_and(|d| clock_now >= d);
    Ok((state, closes_at, closed))
}

pub fn compute_intake_flags(closed: bool, publication_failed: bool) -> (bool, bool, bool) {
    let lifecycle_open = !closed;
    let publication_hold = publication_failed;
    let intake_open = !closed && !publication_failed;
    (lifecycle_open, publication_hold, intake_open)
}

pub fn validate_contribution(payload: &ContributionPayload) -> Result<(), &'static str> {
    if !VALID_ACTORS.contains(&payload.actor.as_str()) {
        return Err("unknown participant");
    }
    if !VALID_KINDS.contains(&payload.kind.as_str()) {
        return Err("unknown contribution kind");
    }
    let trimmed_len = payload.text.trim().chars().count();
    if trimmed_len == 0 || trimmed_len > 10000 {
        return Err("contribution requires 1-10000 characters");
    }
    if payload.evidence.chars().count() > 3000 {
        return Err("evidence references exceed bound");
    }
    Ok(())
}

/// Builds, validates bounds, serializes, and hashes an appended entry.
///
/// Matches `board.py` append rules:
/// - Fails if `validate_contribution(&payload)` fails (actor, kind, text length, evidence bound).
/// - Fails if `now` is non-finite (`NaN` / `Inf`).
/// - Checks serialized payload length against `LIMIT * 2 = 32768` bytes ("entry byte bound exceeded").
/// - Checks `used_bytes + len > max_storage_bytes` ("storage budget reached; history retained, no automatic eviction").
/// - Produces identical SHA-256 contribution hashes to `board.py`.
///
/// # Errors
/// Returns `Err` if contribution is invalid, time is non-finite, serialized bytes exceed bounds, or storage budget is exceeded.
pub fn build_appended_entry(
    payload: ContributionPayload,
    id: u64,
    now: f64,
    used_bytes: u64,
    max_storage_bytes: u64,
) -> Result<(BoardEntry, Vec<u8>, AppendReceipt), &'static str> {
    validate_contribution(&payload)?;

    if !now.is_finite() {
        return Err("invalid received_at_epoch: must be finite");
    }

    let entry = BoardEntry {
        id,
        actor: payload.actor,
        kind: payload.kind,
        text: payload.text,
        evidence: payload.evidence,
        received_at_epoch: now,
        attribution: "self-reported discussion label, not authenticated review identity"
            .to_string(),
        accepted: false,
        status: "advisory-unverified".to_string(),
    };

    let val = entry.to_ordered_value();
    let serialized = serde_json::to_vec_pretty(&val).map_err(|_| "serialization failed")?;

    if serialized.len() > LIMIT * 2 {
        return Err("entry byte bound exceeded");
    }

    if used_bytes.saturating_add(serialized.len() as u64) > max_storage_bytes {
        return Err("storage budget reached; history retained, no automatic eviction");
    }

    let mut hasher = Sha256::new();
    hasher.update(&serialized);
    let mut hash_hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        let _ = write!(hash_hex, "{byte:02x}");
    }

    let receipt = AppendReceipt {
        id,
        sha256: hash_hex,
        accepted: false,
        contract_version: 3,
    };

    Ok((entry, serialized, receipt))
}

/// Paginates entries in reverse chronological order (newest first) and returns the page in ascending order.
///
/// Matches `board.py` validation rules:
/// - `before` must be `>= 1` if provided.
/// - `limit` must be `1..=PAGE_MAX` (`1..=128`).
///
/// # Errors
/// Returns `Err("pagination out of bounds")` if `limit` is 0 or exceeds `PAGE_MAX`, or if `before < 1`.
pub fn paginate_entries(
    all_entries: &[BoardEntry],
    before: Option<u64>,
    limit: usize,
) -> Result<(Vec<BoardEntry>, bool, Option<u64>), &'static str> {
    if limit == 0 || limit > PAGE_MAX {
        return Err("pagination out of bounds");
    }
    if let Some(b) = before {
        if b < 1 {
            return Err("pagination out of bounds");
        }
    }

    let mut filtered: Vec<&BoardEntry> = all_entries
        .iter()
        .filter(|e| before.is_none_or(|b| e.id < b))
        .collect();

    // Sort descending by ID to grab the newest matching
    filtered.sort_by_key(|a| std::cmp::Reverse(a.id));

    let has_older = filtered.len() > limit;
    let selected_refs = &filtered[..filtered.len().min(limit)];

    let older_cursor = if has_older {
        selected_refs.last().map(|e| e.id)
    } else {
        None
    };

    // Return in chronological order (ascending)
    let mut page: Vec<BoardEntry> = selected_refs.iter().map(|&e| (*e).clone()).collect();
    page.reverse();

    Ok((page, has_older, older_cursor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid = BoardConfig {
            schema_version: 2,
            max_storage_bytes: 536870912,
            port: 31492,
            restart_delay_seconds: Some(5),
            restart_window_seconds: Some(300),
            max_restarts: Some(5),
        };
        assert!(validate_config(&valid).is_ok());

        let bad_schema = BoardConfig {
            schema_version: 1,
            ..valid.clone()
        };
        assert_eq!(
            validate_config(&bad_schema).unwrap_err(),
            "unsupported server configuration schema_version"
        );

        let bad_port = BoardConfig {
            port: 80,
            ..valid.clone()
        };
        assert_eq!(
            validate_config(&bad_port).unwrap_err(),
            "explicit local unprivileged port required"
        );
    }

    #[test]
    fn test_contribution_bounds() {
        let good = ContributionPayload {
            actor: "Gemini".into(),
            kind: "evidence".into(),
            text: "Passing gate results verified".into(),
            evidence: "PR #226; exit 0".into(),
        };
        assert!(validate_contribution(&good).is_ok());

        let unknown_actor = ContributionPayload {
            actor: "MaliciousActor".into(),
            ..good.clone()
        };
        assert_eq!(
            validate_contribution(&unknown_actor).unwrap_err(),
            "unknown participant"
        );

        let empty_text = ContributionPayload {
            text: "   ".into(),
            ..good.clone()
        };
        assert_eq!(
            validate_contribution(&empty_text).unwrap_err(),
            "contribution requires 1-10000 characters"
        );

        // Probe 3 test: 10000 'x' followed by 50 spaces should be accepted (trimmed len <= 10000)
        let mut text_with_trailing = "x".repeat(10000);
        text_with_trailing.push_str(&" ".repeat(50));
        let probe3 = ContributionPayload {
            text: text_with_trailing,
            ..good.clone()
        };
        assert!(validate_contribution(&probe3).is_ok());

        // Text exceeding 10000 trimmed chars should be rejected
        let text_too_long = "x".repeat(10001);
        let too_long = ContributionPayload {
            text: text_too_long,
            ..good.clone()
        };
        assert_eq!(
            validate_contribution(&too_long).unwrap_err(),
            "contribution requires 1-10000 characters"
        );
    }

    #[test]
    fn test_deterministic_append_receipt_and_hash_parity() {
        let payload = ContributionPayload {
            actor: "Guest".into(),
            kind: "idea".into(),
            text: "Hello world".into(),
            evidence: "test".into(),
        };
        let res = build_appended_entry(payload, 1, 1000.0, 0, 32768);
        assert!(res.is_ok());
        let (entry, bytes, receipt) = res.unwrap();
        assert_eq!(entry.id, 1);
        assert_eq!(receipt.id, 1);
        assert_eq!(receipt.contract_version, 3);
        assert!(!receipt.accepted);
        assert_eq!(receipt.sha256.len(), 64);
        assert_eq!(bytes.len(), 271);
        // Exact SHA-256 parity with board.py Python output:
        assert_eq!(
            receipt.sha256,
            "3d085ab5b1c4491abbac496dba98ecdc888e8461d4c070481227c19171e9f10c"
        );
    }

    #[test]
    fn test_probe_1_entry_byte_bound_and_storage_budget() {
        // Probe 1: 10000 4-byte characters (🦀 = 4 bytes) plus 3000 evidence characters
        // validate_contribution checks chars (10000 and 3000) so it passes character count check
        let four_byte_char = "\u{1F980}"; // 🦀
        let large_payload = ContributionPayload {
            actor: "Gemini".into(),
            kind: "evidence".into(),
            text: four_byte_char.repeat(10000),
            evidence: "e".repeat(3000),
        };
        assert!(validate_contribution(&large_payload).is_ok());

        // Serialized size is > 40000 bytes, which exceeds LIMIT * 2 = 32768 bytes
        let res = build_appended_entry(large_payload, 10, 1000.0, 0, 1000000);
        assert_eq!(res.unwrap_err(), "entry byte bound exceeded");

        // Storage budget test
        let normal_payload = ContributionPayload {
            actor: "Gemini".into(),
            kind: "evidence".into(),
            text: "normal text".into(),
            evidence: "evidence".into(),
        };
        // Budget only allows 100 bytes, entry is > 200 bytes
        let budget_res = build_appended_entry(normal_payload, 11, 1000.0, 0, 100);
        assert_eq!(
            budget_res.unwrap_err(),
            "storage budget reached; history retained, no automatic eviction"
        );
    }

    #[test]
    fn test_probe_2_non_finite_time_rejected() {
        let payload = ContributionPayload {
            actor: "Gemini".into(),
            kind: "idea".into(),
            text: "Testing nan".into(),
            evidence: "".into(),
        };
        let res_nan = build_appended_entry(payload.clone(), 1, f64::NAN, 0, 32768);
        assert_eq!(
            res_nan.unwrap_err(),
            "invalid received_at_epoch: must be finite"
        );

        let res_inf = build_appended_entry(payload, 1, f64::INFINITY, 0, 32768);
        assert_eq!(
            res_inf.unwrap_err(),
            "invalid received_at_epoch: must be finite"
        );
    }

    #[test]
    fn test_pagination_limits_and_cursors() {
        let entries: Vec<BoardEntry> = (1..=20)
            .map(|i| BoardEntry {
                id: i,
                actor: "Gemini".into(),
                kind: "evidence".into(),
                text: format!("Entry {}", i),
                evidence: "".into(),
                received_at_epoch: 1000.0 + (i as f64),
                attribution: "test".into(),
                accepted: false,
                status: "advisory-unverified".into(),
            })
            .collect();

        // Page 5 newest (16..=20)
        let (page, has_older, cursor) = paginate_entries(&entries, None, 5).unwrap();
        assert_eq!(page.len(), 5);
        assert_eq!(page.first().unwrap().id, 16);
        assert_eq!(page.last().unwrap().id, 20);
        assert!(has_older);
        assert_eq!(cursor, Some(16));

        // Fetch older before cursor 16 (11..=15)
        let (older_page, has_more_older, next_cursor) =
            paginate_entries(&entries, Some(16), 5).unwrap();
        assert_eq!(older_page.len(), 5);
        assert_eq!(older_page.first().unwrap().id, 11);
        assert_eq!(older_page.last().unwrap().id, 15);
        assert!(has_more_older);
        assert_eq!(next_cursor, Some(11));

        // Bound validations:
        // limit = 0 rejected
        assert_eq!(
            paginate_entries(&entries, None, 0).unwrap_err(),
            "pagination out of bounds"
        );
        // limit > PAGE_MAX (128) rejected
        assert_eq!(
            paginate_entries(&entries, None, 129).unwrap_err(),
            "pagination out of bounds"
        );
        // before < 1 rejected
        assert_eq!(
            paginate_entries(&entries, Some(0), 10).unwrap_err(),
            "pagination out of bounds"
        );
    }

    #[test]
    fn test_build_appended_entry_validates_contribution_payload() {
        let bad_actor = ContributionPayload {
            actor: "UnregisteredAttacker".into(),
            kind: "idea".into(),
            text: "Hello".into(),
            evidence: "".into(),
        };
        let res = build_appended_entry(bad_actor, 1, 1000.0, 0, 32768);
        assert_eq!(res.unwrap_err(), "unknown participant");

        let empty_text = ContributionPayload {
            actor: "Gemini".into(),
            kind: "idea".into(),
            text: "   ".into(),
            evidence: "".into(),
        };
        let res_empty = build_appended_entry(empty_text, 1, 1000.0, 0, 32768);
        assert_eq!(
            res_empty.unwrap_err(),
            "contribution requires 1-10000 characters"
        );
    }
}
