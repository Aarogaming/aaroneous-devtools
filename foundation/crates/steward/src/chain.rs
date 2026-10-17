//! The steward's tamper-evident event log lines. Port of `steward_run.digest` and the line `append_event` writes
//! (vector family `chain`): each event carries the SHA-256 of the previous raw line, serialised the way Python's
//! `json.dumps(event, sort_keys=True, ensure_ascii=False)` does.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

/// The `prev` of the first event.
pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// SHA-256 of a raw log line, as lower-case hex.
#[must_use]
pub fn digest(line: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(line.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn escape(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `json.dumps(value, sort_keys=True, ensure_ascii=False)`: keys sorted by code point, `", "` and `": "` separators,
/// non-ASCII text left as is. Integers only for numbers: Python's float formatting is not reproduced.
#[must_use]
pub fn dumps(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => escape(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                escape(key, out);
                out.push_str(": ");
                write_value(&map[key], out);
            }
            out.push('}');
        }
    }
}

/// The raw line for the event with sequence number `seq` after a line whose digest is `prev`.
#[must_use]
pub fn event_line(seq: u64, ts: &str, event_type: &str, data: &Value, prev: &str) -> String {
    let mut event = serde_json::Map::new();
    event.insert("seq".to_owned(), Value::from(seq));
    event.insert("ts".to_owned(), Value::from(ts));
    event.insert("type".to_owned(), Value::from(event_type));
    event.insert("data".to_owned(), data.clone());
    event.insert("prev".to_owned(), Value::from(prev));
    dumps(&Value::Object(event))
}

/// Append each `(type, data, now)` to an empty log: the lines, and the digest of the last one (the tip), or
/// [`GENESIS`] for an empty log.
#[must_use]
pub fn build_log(events: &[(String, Value, String)]) -> (Vec<String>, String) {
    let mut lines: Vec<String> = Vec::new();
    let mut prev = GENESIS.to_owned();
    for (index, (event_type, data, now)) in events.iter().enumerate() {
        let line = event_line(index as u64 + 1, now, event_type, data, &prev);
        prev = digest(&line);
        lines.push(line);
    }
    (lines, prev)
}

#[cfg(test)]
mod tests {
    use super::{GENESIS, build_log, digest, dumps};
    use serde_json::json;

    #[test]
    fn digest_is_sha256_hex() {
        assert_eq!(
            digest(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn dumps_matches_pythons_default_separators_and_sorted_keys() {
        assert_eq!(
            dumps(&json!({"b": [1, true, null], "a": "x"})),
            "{\"a\": \"x\", \"b\": [1, true, null]}"
        );
        assert_eq!(
            dumps(&json!("d\u{e9}j\u{e0}\n\"\\\u{1}\u{7f}")),
            "\"d\u{e9}j\u{e0}\\n\\\"\\\\\\u0001\u{7f}\""
        );
        assert_eq!(dumps(&json!({})), "{}");
        assert_eq!(dumps(&json!([])), "[]");
    }

    #[test]
    fn the_first_event_chains_from_genesis_and_an_empty_log_has_the_genesis_tip() {
        let (lines, tip) =
            build_log(&[("x".to_owned(), json!({}), "2026-10-05T00:00:00Z".to_owned())]);
        assert!(lines[0].contains(GENESIS));
        assert_eq!(tip, digest(&lines[0]));
        assert_eq!(build_log(&[]), (Vec::new(), GENESIS.to_owned()));
    }
}
