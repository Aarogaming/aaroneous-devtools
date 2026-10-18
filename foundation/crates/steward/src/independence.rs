//! Which model families the accepting verdicts came from (lane C125, vector family `independence`).
//!
//! A seat is a place and a key; the model behind it changes. The family of an accept is the family of the model the
//! signed record declares (through the registry's `model_families` table), else the seat's registered
//! `profile.family`, else the seat stands alone. The report says what it saw; it never refuses anyone. Box 3 of the
//! merge gate only counts the distinct families this returns.
//!
//! Pure functions over JSON values: no I/O, no clock, no environment. Mirrors `review_coverage.independence`, where the
//! logic was first written; the port is the authority from here (the Python copy is the oracle only until it is
//! retired).

use crate::coverage::canonical_reviewer_seat;
use crate::gate::py_repr_str;
use crate::seats::{canonical, named_seats};
use crate::text::strip;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Longest model id a record may declare.
pub const MODEL_MAX: usize = 80;

/// One accepting seat and the model it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub seat: String,
    pub model: Option<String>,
    pub family: String,
    /// `declared` (the record's model), `registered` (the seat's profile) or `seat` (it stands alone).
    pub source: &'static str,
}

/// What the accepting verdicts say about who judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub families: Vec<String>,
    pub entries: Vec<Entry>,
    pub notes: Vec<String>,
}

impl Report {
    /// The report as the vector contract spells it.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|e| {
                let mut m = Map::new();
                m.insert("seat".into(), Value::String(e.seat.clone()));
                m.insert(
                    "model".into(),
                    e.model.clone().map_or(Value::Null, Value::String),
                );
                m.insert("family".into(), Value::String(e.family.clone()));
                m.insert("source".into(), Value::String(e.source.to_owned()));
                Value::Object(m)
            })
            .collect();
        let mut m = Map::new();
        m.insert(
            "families".into(),
            Value::Array(self.families.iter().cloned().map(Value::String).collect()),
        );
        m.insert("entries".into(), Value::Array(entries));
        m.insert(
            "notes".into(),
            Value::Array(self.notes.iter().cloned().map(Value::String).collect()),
        );
        Value::Object(m)
    }
}

/// A family label: a lower-case letter, then letters, digits or `-`, at most 32 characters in all.
fn is_family_label(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.chars().count() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A model-name stem: a lower-case letter, then letters, digits, `.` or `-`, at most 32 characters in all.
fn is_stem(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.chars().count() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

/// The model id a record declares, or `None`. Declaring is optional and never refused: a missing, non-string,
/// over-long or non-printable value is simply not used (the family falls back to the seat's registered profile).
/// Printable means ASCII space to `~`: model ids are ASCII, and both languages then agree without a Unicode table.
#[must_use]
pub fn clean_model(value: Option<&Value>) -> Option<String> {
    let text = strip(value?.as_str()?);
    if text.is_empty()
        || text.chars().count() > MODEL_MAX
        || !text.chars().all(|c| (' '..='~').contains(&c))
    {
        return None;
    }
    Some(text.to_owned())
}

/// `(stem, family)` pairs from the registry's `model_families`, longest stem first.
#[must_use]
pub fn model_table(identities: &Value) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = identities
        .get("model_families")
        .and_then(Value::as_object)
        .map(|table| {
            table
                .iter()
                .filter_map(|(stem, family)| {
                    let family = family.as_str()?;
                    (is_stem(stem) && is_family_label(family))
                        .then(|| (stem.clone(), family.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    pairs
}

/// The family a model id belongs to per `table`, or `None` for a model the table does not know. The id's last path
/// segment is matched against each stem: equal, or followed by a non-letter (`qwen3.8`, `claude-sonnet-5-5`,
/// `Qwen/Qwen3.8-Coder`, `gpt-5.5`), never by a letter (`qwenish` is not qwen).
#[must_use]
pub fn model_family(model: Option<&str>, table: &[(String, String)]) -> Option<String> {
    let cleaned = clean_model(model.map(|m| Value::String(m.to_owned())).as_ref())?;
    let lowered = cleaned.to_lowercase();
    let name = lowered.rsplit('/').next().unwrap_or("");
    table
        .iter()
        .find(|(stem, _)| {
            name.strip_prefix(stem.as_str())
                .is_some_and(|rest| !rest.chars().next().is_some_and(char::is_alphabetic))
        })
        .map(|(_, family)| family.clone())
}

/// Lower-case alphanumeric runs joined by hyphens: `Ubuntu Worker 01`, `ubuntu_worker_01` and `ubuntu-worker-01` agree.
fn hyphenated(text: &str) -> String {
    let lowered = strip(text).to_lowercase();
    lowered
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|run| !run.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The seat a reviewer label names, or `None` when it names none or more than one. A registered seat is matched by its
/// whole hyphen-delimited name (`ubuntu-worker-01`, never the unknown seat `ubuntu`); every other seat the label names
/// (a second registered seat, or a known seat or model family such as `codex` or `Opus`) makes it ambiguous, so
/// `ubuntu-worker-01 and codex` is not silently read as `ubuntu-worker-01`. Fails closed.
#[must_use]
pub fn resolve_seat(label: &str, registered: &BTreeSet<String>) -> Option<String> {
    let flat = format!("-{}-", hyphenated(label));
    let hits: Vec<&String> = registered
        .iter()
        .filter(|seat| {
            let name = hyphenated(seat);
            !name.is_empty() && flat.contains(&format!("-{name}-"))
        })
        .collect();
    match hits.as_slice() {
        [] => match canonical_reviewer_seat(label) {
            // A hyphenated registered seat is named by its whole name only. Reaching it here means the label matched through
            // the bare-prefix alias (`ubuntu`), so `ubuntu`, `ubuntu-worker` and `ubuntu-worker-011` are near misses of
            // `ubuntu-worker-01`, not that seat: keep the label's own first token, as before the alias existed.
            Some(seat) if seat.contains('-') && registered.contains(&seat) => first_token(label),
            other => other,
        },
        [seat] => {
            let rest = flat.replacen(&format!("-{}-", hyphenated(seat)), "-", 1);
            let mut others = named_seats(&rest);
            others.remove(canonical(seat));
            others.is_empty().then(|| (*seat).clone())
        }
        _ => None,
    }
}

/// The first lower-case alphanumeric run of a label, the seat name an unknown label keeps for itself.
fn first_token(label: &str) -> Option<String> {
    strip(label)
        .to_lowercase()
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .find(|run| !run.is_empty())
        .map(str::to_owned)
}

/// The registered `profile.family` of a seat, if it declares a well-formed one.
fn registered_family(identities: &Value, seat: &str) -> Option<String> {
    let family = strip(
        identities
            .get("seats")?
            .get(seat)?
            .get("profile")?
            .get("family")?
            .as_str()?,
    );
    is_family_label(family).then(|| family.to_owned())
}

/// What the accepting verdicts (`counted`: records with a `reviewer` and an optional `model`) say about who judged,
/// against the identity registry. One entry per accepting seat and declared model. The notes are the self-report:
/// an unclassified model, a model that differs from the registered profile, and accepts that share a family.
#[must_use]
pub fn independence(counted: &[Value], identities: &Value) -> Report {
    let registered: BTreeSet<String> = identities
        .get("seats")
        .and_then(Value::as_object)
        .map(|seats| seats.keys().cloned().collect())
        .unwrap_or_default();
    let table = model_table(identities);
    let mut entries: Vec<Entry> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for record in counted {
        let Some(seat) = record
            .get("reviewer")
            .and_then(Value::as_str)
            .and_then(|label| resolve_seat(label, &registered))
        else {
            continue;
        };
        let model = clean_model(record.get("model"));
        let registered_family = registered_family(identities, &seat);
        let declared = model_family(model.as_deref(), &table);
        let (family, source) = match (&declared, &registered_family) {
            (Some(family), _) => (family.clone(), "declared"),
            (None, Some(family)) => (family.clone(), "registered"),
            (None, None) => (format!("seat:{seat}"), "seat"),
        };
        if entries
            .iter()
            .any(|e| e.seat == seat && e.model == model && e.family == family)
        {
            continue;
        }
        if let (Some(model), None) = (&model, &declared) {
            notes.push(format!(
                "{seat} declared model {}, which model_families does not classify; counted as {family} ({source})",
                py_repr_str(model)
            ));
        } else if let (Some(model), Some(declared), Some(registered)) =
            (&model, &declared, &registered_family)
        {
            if declared != registered {
                notes.push(format!(
                    "{seat} is registered as {registered} but ran {} ({declared}): the declared model is used",
                    py_repr_str(model)
                ));
            }
        }
        entries.push(Entry {
            seat,
            model,
            family,
            source,
        });
    }
    let families: Vec<String> = entries
        .iter()
        .map(|e| e.family.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    for family in &families {
        let sharing: Vec<&str> = entries
            .iter()
            .filter(|e| &e.family == family)
            .map(|e| e.seat.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if sharing.len() > 1 {
            notes.push(format!(
                "{} are the same model family ({family}): counted as one judge",
                sharing.join(", ")
            ));
        }
    }
    Report {
        families,
        entries,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::{independence, model_family, model_table, resolve_seat};
    use serde_json::json;
    use std::collections::BTreeSet;

    fn registry() -> serde_json::Value {
        json!({
            "seats": {
                "opencode": {"profile": {"family": "mimo"}},
                "ubuntu-worker-01": {"profile": {"family": "qwen"}},
                "claude": {"profile": {"family": "anthropic"}},
                "codex": {}
            },
            "model_families": {"qwen": "qwen", "mimo": "mimo", "claude": "anthropic", "gpt": "openai"}
        })
    }

    #[test]
    fn a_stem_matches_a_name_not_a_prefix_of_a_longer_word() {
        let table = model_table(&registry());
        for (model, family) in [
            ("qwen3.8", Some("qwen")),
            ("Qwen/Qwen3.8-Coder-14B", Some("qwen")),
            ("claude-sonnet-5-5", Some("anthropic")),
            ("gpt-5.5", Some("openai")),
            ("qwenish", None),
            ("llama-4", None),
        ] {
            assert_eq!(
                model_family(Some(model), &table).as_deref(),
                family,
                "{model}"
            );
        }
    }

    #[test]
    fn a_registered_multi_word_seat_resolves_by_its_whole_name() {
        let registered: BTreeSet<String> = ["claude", "ubuntu-worker-01", "ubuntu-worker-02"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        for label in ["ubuntu-worker-01", "Ubuntu Worker 01", "UBUNTU_WORKER_01"] {
            assert_eq!(
                resolve_seat(label, &registered).as_deref(),
                Some("ubuntu-worker-01"),
                "{label}"
            );
        }
        for label in ["xubuntu-worker-01", "ubuntu-worker-011", "ubuntu-worker"] {
            assert_ne!(
                resolve_seat(label, &registered).as_deref(),
                Some("ubuntu-worker-01"),
                "{label}: a near miss is not the registered seat"
            );
        }
        assert_eq!(
            resolve_seat("ubuntu-worker-01 and ubuntu-worker-02", &registered),
            None
        );
        // A near miss keeps its own first token and is never a registered seat.
        assert_eq!(
            resolve_seat("ubuntu", &registered).as_deref(),
            Some("ubuntu")
        );
        assert_eq!(
            resolve_seat("ubuntu-worker-011", &registered).as_deref(),
            Some("ubuntu")
        );
        assert_eq!(
            resolve_seat("ubuntu-worker-03", &registered).as_deref(),
            Some("ubuntu")
        );
        assert_eq!(
            resolve_seat("ubuntu-worker-02", &registered).as_deref(),
            Some("ubuntu-worker-02")
        );
    }

    #[test]
    fn a_label_naming_a_registered_seat_and_another_seat_is_ambiguous() {
        // Codex review of #186: "ubuntu-worker-01 and codex" resolved to ubuntu-worker-01 and dropped codex.
        let registered: BTreeSet<String> = [
            "claude",
            "codex",
            "gemini",
            "ubuntu-worker-01",
            "ubuntu-worker-02",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        for label in [
            "ubuntu-worker-01 and codex",
            "codex, ubuntu-worker-01",
            "Ubuntu Worker 01 + Gemini",
            "ubuntu-worker-01 / Opus 5.5",
            "ubuntu-worker-01 and ubuntu-worker-02",
            "ubuntu-worker-01-codex",
            // the model belongs in the record's `model` field, not the label: a model name that is also a seat alias is a second seat
            "Ubuntu Worker 01 (qwen3.8)",
        ] {
            assert_eq!(resolve_seat(label, &registered), None, "{label}");
        }
        for (label, seat) in [
            ("ubuntu-worker-01", "ubuntu-worker-01"),
            ("Ubuntu Worker 01", "ubuntu-worker-01"),
            ("UBUNTU_WORKER_01", "ubuntu-worker-01"),
            ("claude-sonnet-5-5", "claude"),
            ("Claude Sonnet 5.5", "claude"),
            ("gpt-5.5", "codex"),
        ] {
            assert_eq!(
                resolve_seat(label, &registered).as_deref(),
                Some(seat),
                "{label}"
            );
        }
    }

    #[test]
    fn the_same_hardware_on_a_different_model_is_two_judges_and_one_model_is_one() {
        let different = independence(
            &[
                json!({"reviewer": "opencode"}),
                json!({"reviewer": "ubuntu-worker-01"}),
            ],
            &registry(),
        );
        assert_eq!(different.families, ["mimo", "qwen"]);
        assert!(different.notes.is_empty());
        let same = independence(
            &[
                json!({"reviewer": "opencode", "model": "mimo-v2.6"}),
                json!({"reviewer": "ubuntu-worker-01", "model": "mimo-v2.6-flash"}),
            ],
            &registry(),
        );
        assert_eq!(same.families, ["mimo"]);
        assert!(
            same.notes
                .iter()
                .any(|n| n.contains("same model family (mimo)")),
            "{:?}",
            same.notes
        );
    }

    #[test]
    fn an_unclassified_model_is_reported_and_buys_no_new_family() {
        let report = independence(
            &[json!({"reviewer": "ubuntu-worker-01", "model": "brandnew-9"})],
            &registry(),
        );
        assert_eq!(report.families, ["qwen"]);
        assert!(report.notes[0].contains("does not classify"));
    }

    #[test]
    fn a_malformed_model_or_registry_is_ignored_never_a_crash() {
        for bad in [
            json!(7),
            json!(["qwen"]),
            json!("x".repeat(200)),
            json!("a\nb"),
        ] {
            let report = independence(
                &[json!({"reviewer": "ubuntu-worker-01", "model": bad})],
                &registry(),
            );
            assert_eq!(report.families, ["qwen"]);
        }
        for registry in [
            json!(null),
            json!([]),
            json!({"seats": []}),
            json!({"seats": {"x": "y"}}),
        ] {
            let report = independence(&[json!({"reviewer": "codex"})], &registry);
            assert_eq!(report.families, ["seat:codex"]);
        }
    }
}
