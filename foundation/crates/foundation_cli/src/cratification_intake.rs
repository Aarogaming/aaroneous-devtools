//! M49 — Cratification intake: turn a harvested candidate into evidence for
//! a real `code-draft` job, not an automatic transform.
//!
//! Corrected framing (see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s
//! M49 section): `harvest`/`scaffold` don't chain. `cratify harvest` is
//! read-only analysis that produces a `CrateSpec` describing what exists in
//! an external candidate crate; `scaffold`/`generate` only creates an empty
//! boilerplate skeleton and has no parameter for `harvest`'s output. Porting
//! external logic into this framework's idioms is a judgment call -- exactly
//! the kind of thing `contracts/FRONTIER_FILTRATION_STANDARD_V1.md` already
//! assigns to "the local agent extraction loop," not a mechanical transform.
//!
//! This module is the missing front door: it reads Aaroneous's exported
//! `Vec<CrateSpec>` JSON (from `cratify harvest --output <path>`, added on
//! the Aaroneous side as part of this same milestone) and emits one
//! `code-draft` `worker-jobs/*.toml` packet per harvested crate, modeled
//! directly on `frontier_discovery::create_frontier_job` and
//! `capability_composition::create_composition_job`. Each packet's evidence
//! points at both the original source location and the harvested JSON
//! export itself, and its instruction asks for a compliant Rust port --
//! everything downstream (M44's dispatch/pre-proof, M45's `cratify_audit`
//! compliance check) already exists and treats this like any other
//! code-draft job.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Mirrors Aaroneous's `CodeInfo` shape loosely. Per `FOUNDATION_BOUNDARY.md`
/// this module takes no Cargo dependency on Aaroneous or its crates and
/// doesn't assume the exact per-item field shape of `functions`/`structs`/
/// `enums` beyond "likely has a `name`" -- this producer only needs
/// *counts* to summarize what was found in the emitted instruction text, so
/// each field is `Vec<serde_json::Value>` rather than a fully-typed mirror.
/// A reviewer who wants richer per-item detail in the instruction later can
/// tighten this without changing the file contract.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeInfo {
    #[serde(default)]
    pub functions: Vec<serde_json::Value>,
    #[serde(default)]
    pub structs: Vec<serde_json::Value>,
    #[serde(default)]
    pub enums: Vec<serde_json::Value>,
}

/// Mirrors Aaroneous's `CrateSpec` shape as exported by
/// `cratify harvest --output <path>`. Defined locally, not imported from
/// Aaroneous -- see module docs and `CapabilityDescriptor` in
/// `capability_composition.rs` for the same precedent.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateSpec {
    pub name: String,
    pub source_path: String,
    #[serde(default)]
    pub code_info: CodeInfo,
}

/// Reads the harvested `CrateSpec` JSON export from an explicit absolute
/// path. No ambient discovery, per `FOUNDATION_BOUNDARY.md` and
/// `CONTRIBUTING.md`'s "Foundation commands require explicit input and
/// output paths" rule.
pub fn read_crate_specs(path: &Path) -> Result<Vec<CrateSpec>> {
    if !path.is_absolute() {
        bail!(
            "crate specs path must be an absolute path: {}",
            path.display()
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read crate specs export: {}", path.display()))?;
    let specs: Vec<CrateSpec> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse crate specs JSON: {}", path.display()))?;
    Ok(specs)
}

/// Slugify a crate name into an id-safe `exploration_group` key: lowercase,
/// non-alphanumeric runs collapsed to a single `_`, trimmed of leading/
/// trailing `_`.
///
/// Duplicated from (not imported from) `frontier_discovery::slugify_query`:
/// that function is private (`fn`, not `pub` or `pub(crate)`) to its module,
/// and the two producers' slugging inputs are conceptually different (a
/// free-text search query there, a crate name here) even though the
/// character-class logic happens to coincide today. Promoting it to a
/// shared `pub(crate)` helper is a reasonable follow-up if a third producer
/// needs the same logic, but isn't justified for two ~15-line call sites --
/// same call the M43 task made for its own small duplicated helper.
fn slugify_crate_name(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_sep = true; // suppress a leading separator
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    slug
}

/// Sanitizes a crate name for safe use as a packet id component: uppercase,
/// non-alphanumeric runs replaced with `_`. Mirrors `frontier_discovery::
/// create_frontier_job`'s `candidate.name.to_uppercase().replace('-', "_")`
/// id-sanitization style, generalized to replace any non-alphanumeric
/// character (crate names harvested from arbitrary external sources aren't
/// guaranteed to only use `-`).
fn sanitize_for_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_uppercase()
}

fn sanitize_for_filename(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

/// Emits one `code-draft` `worker-jobs/*.toml` packet for a harvested
/// `CrateSpec`, modeled directly on `frontier_discovery::create_frontier_job`
/// and `capability_composition::create_composition_job`.
///
/// `evidence` lists the original source path plus the path to the harvested
/// JSON export, so a drafting session can see both where the candidate code
/// actually lives and the full analysis it was selected from.
///
/// M46 tagging: `goal` is the crate's own name, `exploration_group` is a
/// slug of it, and `perspective` is the fixed literal `"initial-port"` --
/// M49 emits exactly one packet per harvested crate, not yet multiple
/// perspectives on porting the same candidate (that's M48's job). A
/// single-crate run still gets tagged; M46's own grouping step drops groups
/// with fewer than two members, so tagging unconditionally is harmless.
pub fn create_cratification_job(
    spec: &CrateSpec,
    output_dir: &Path,
    crate_specs_source_path: &str,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }

    let id_sanitized = sanitize_for_id(&spec.name);
    let job_id = format!("CRATIFY-{id_sanitized}");
    let filename = format!("cratify-{}.toml", sanitize_for_filename(&spec.name));
    let destination = output_dir.join(&filename);

    let function_count = spec.code_info.functions.len();
    let struct_count = spec.code_info.structs.len();
    let enum_count = spec.code_info.enums.len();

    let instruction = format!(
        "Port harvested external crate '{name}' into this framework as a compliant Rust \
draft. Harvest found {function_count} function(s), {struct_count} struct(s), and \
{enum_count} enum(s) at source path '{source_path}'; see the linked evidence for the full \
per-item analysis.\n\
\n\
Follow CONTRIBUTING.md's rules for product code exactly: no ambient authority (no \
std::env::var, no SystemTime::now() outside an explicit bootstrap path), no heap \
allocation on hot paths where the ported logic runs in one, replace every .unwrap()/\
.expect() with explicit Result propagation, and use #[repr(C)] with explicit padding \
fields anywhere the ported type's memory layout matters for FFI or cross-boundary \
transfer. Produce a working port, not a skeleton -- this is a code-draft job, not an \
audit.",
        name = spec.name,
        source_path = spec.source_path,
    );

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "code-draft",
        "evidence": [spec.source_path.clone(), crate_specs_source_path],
        "instruction": instruction,
        "perspective": "initial-port",
        "goal": spec.name,
        "exploration_group": slugify_crate_name(&spec.name),
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str).with_context(|| {
        format!(
            "failed to write cratification packet: {}",
            destination.display()
        )
    })?;
    Ok(destination)
}

/// Ties read -> emit together for the CLI subcommand: reads the harvested
/// `CrateSpec` JSON from `crate_specs_path` and emits one packet per entry
/// into `output_dir`.
pub fn run(crate_specs_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }
    let specs = read_crate_specs(crate_specs_path)?;
    let source_path_str = crate_specs_path.display().to_string();

    let mut written = Vec::with_capacity(specs.len());
    for spec in &specs {
        let path = create_cratification_job(spec, output_dir, &source_path_str)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn spec(name: &str) -> CrateSpec {
        CrateSpec {
            name: name.to_string(),
            source_path: format!("/external/{name}/src/lib.rs"),
            code_info: CodeInfo {
                functions: vec![serde_json::json!({"name": "do_thing"})],
                structs: vec![
                    serde_json::json!({"name": "Widget"}),
                    serde_json::json!({"name": "Gadget"}),
                ],
                enums: vec![],
            },
        }
    }

    #[test]
    fn read_crate_specs_rejects_relative_path() {
        let err = read_crate_specs(Path::new("relative/specs.json")).unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn read_crate_specs_parses_documented_shape() {
        let temp = tempdir().unwrap();
        let json_path = temp.path().join("specs.json");
        let json = r#"[
            {
                "name": "some_crate_name",
                "source_path": "/abs/path/to/source",
                "code_info": {
                    "functions": [{"name": "f1"}],
                    "structs": [{"name": "S1"}],
                    "enums": [{"name": "E1"}]
                }
            }
        ]"#;
        std::fs::write(&json_path, json).unwrap();

        let specs = read_crate_specs(&json_path).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "some_crate_name");
        assert_eq!(specs[0].source_path, "/abs/path/to/source");
        assert_eq!(specs[0].code_info.functions.len(), 1);
        assert_eq!(specs[0].code_info.structs.len(), 1);
        assert_eq!(specs[0].code_info.enums.len(), 1);
    }

    #[test]
    fn create_cratification_job_rejects_relative_output_dir() {
        let s = spec("foo-crate");
        let err = create_cratification_job(&s, Path::new("relative/dir"), "src.json").unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn create_cratification_job_writes_parseable_toml_with_expected_fields() {
        let temp = tempdir().unwrap();
        let s = spec("some-crate-name");

        let path = create_cratification_job(&s, temp.path(), "harvest-export.json").unwrap();
        assert!(path.exists());

        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();

        assert_eq!(parsed["schema_version"].as_integer(), Some(1));
        assert_eq!(parsed["id"].as_str(), Some("CRATIFY-SOME_CRATE_NAME"));
        assert_eq!(parsed["mode"].as_str(), Some("code-draft"));

        let evidence = parsed["evidence"].as_array().unwrap();
        assert!(
            evidence
                .iter()
                .any(|v| v.as_str() == Some(s.source_path.as_str()))
        );
        assert!(
            evidence
                .iter()
                .any(|v| v.as_str() == Some("harvest-export.json"))
        );

        let instruction = parsed["instruction"].as_str().unwrap();
        assert!(instruction.contains("CONTRIBUTING.md"));
        assert!(instruction.contains("std::env::var"));
        assert!(instruction.contains("SystemTime::now()"));
        assert!(instruction.contains("#[repr(C)]"));
        assert!(instruction.contains(".unwrap()"));
        assert!(instruction.contains("some-crate-name"));

        assert_eq!(parsed["perspective"].as_str(), Some("initial-port"));
        assert_eq!(parsed["goal"].as_str(), Some("some-crate-name"));
        assert_eq!(
            parsed["exploration_group"].as_str(),
            Some("some_crate_name")
        );
    }

    #[test]
    fn slugify_crate_name_matches_frontier_discovery_style() {
        assert_eq!(slugify_crate_name("some-crate-name"), "some_crate_name");
        assert_eq!(slugify_crate_name("  Weird__Crate!! "), "weird_crate");
        assert_eq!(slugify_crate_name("zero_copy"), "zero_copy");
    }

    #[test]
    fn run_reads_specs_and_emits_packets_end_to_end() {
        let temp = tempdir().unwrap();
        let json_path = temp.path().join("specs.json");
        let specs = vec![spec("crate-a"), spec("crate-b")];
        std::fs::write(&json_path, serde_json::to_string(&specs).unwrap()).unwrap();

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(&json_path, &output_dir).unwrap();
        assert_eq!(written.len(), 2);
        for path in &written {
            assert!(path.exists());
        }
    }

    #[test]
    fn run_rejects_relative_output_dir() {
        let temp = tempdir().unwrap();
        let json_path = temp.path().join("specs.json");
        std::fs::write(&json_path, "[]").unwrap();
        let err = run(&json_path, Path::new("relative/dir")).unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }
}
