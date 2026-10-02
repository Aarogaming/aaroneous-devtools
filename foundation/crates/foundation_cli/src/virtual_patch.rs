//! Immutable Virtual Patch Sets and Reviewer-Selected Materialization
//!
//! Provides cryptographic, digest-bound virtual patch sets for local model drafts.
//! Ensures model proposals remain immutable, revision-bound, and strictly outside
//! Git until explicitly reviewed, selected, and materialized into a target directory.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "foundation.virtual-patch-set";

/// A single atomic hunk in a virtual patch set
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VirtualPatch {
    pub patch_index: usize,
    pub file_path: String,
    /// Expected SHA-256 digest of the target file before patch application.
    /// `None` indicates a newly created file that must not previously exist.
    pub base_digest: Option<String>,
    pub search_block: String,
    pub replace_block: String,
    /// SHA-256 hash of `search_block \0 replace_block`
    pub hunk_digest: String,
}

/// Provenance metadata tracking the generation context
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PatchProvenance {
    pub generator_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub instruction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// An immutable, cryptographically sealed virtual patch set
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VirtualPatchSet {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub patch_set_id: String,
    pub task_id: String,
    pub base_revision: String,
    pub created_at_secs: u64,
    pub patches: Vec<VirtualPatch>,
    /// SHA-256 over canonical patch set contents
    pub patch_set_digest: String,
    pub provenance: PatchProvenance,
}

/// Independent reviewer disposition
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewDisposition {
    Approved,
    Rejected,
    NeedsChanges,
}

/// Reviewer selection specification for materialization
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewSelection {
    pub selected_patch_indices: Vec<usize>,
    pub reviewer: String,
    pub disposition: ReviewDisposition,
    pub rationale: String,
}

/// Result of materializing selected patches
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MaterializationResult {
    pub patch_set_id: String,
    pub applied_patches: Vec<usize>,
    pub unified_diff: String,
    pub files_modified: Vec<String>,
    pub files_created: Vec<String>,
}

// ============================================================================
// Cryptographic Hash Implementation (Pure Rust SHA-256, FIPS 180-4)
// ============================================================================

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_digest(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, part) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([part[0], part[1], part[2], part[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut out = String::with_capacity(64);
    for val in h {
        out.push_str(&format!("{val:08x}"));
    }
    out
}

pub fn compute_hunk_digest(search_block: &str, replace_block: &str) -> String {
    let mut payload = Vec::with_capacity(search_block.len() + replace_block.len() + 1);
    payload.extend_from_slice(search_block.as_bytes());
    payload.push(0);
    payload.extend_from_slice(replace_block.as_bytes());
    sha256_digest(&payload)
}

pub fn compute_patch_set_digest(
    patch_set_id: &str,
    task_id: &str,
    base_revision: &str,
    patches: &[VirtualPatch],
) -> String {
    let mut payload = Vec::new();
    payload.extend_from_slice(patch_set_id.as_bytes());
    payload.push(0);
    payload.extend_from_slice(task_id.as_bytes());
    payload.push(0);
    payload.extend_from_slice(base_revision.as_bytes());
    payload.push(0);
    for p in patches {
        payload.extend_from_slice(p.file_path.as_bytes());
        payload.push(0);
        if let Some(b) = &p.base_digest {
            payload.extend_from_slice(b.as_bytes());
        }
        payload.push(0);
        payload.extend_from_slice(p.hunk_digest.as_bytes());
        payload.push(0);
    }
    sha256_digest(&payload)
}

// ============================================================================
// Path Validation & Traversal Containment
// ============================================================================

pub fn validate_patch_path(file_path: &str) -> Result<()> {
    if file_path.trim().is_empty() {
        bail!("file_path must not be empty");
    }
    let norm = file_path.replace('\\', "/");
    if norm.starts_with('/') {
        bail!("file_path must be relative: {file_path}");
    }
    if norm.contains(':') {
        bail!("file_path must not contain drive separators: {file_path}");
    }

    let p = Path::new(&norm);
    for c in p.components() {
        match c {
            Component::ParentDir => bail!("path traversal ('..') is forbidden: {file_path}"),
            Component::CurDir => {
                bail!("current directory component ('.') is forbidden: {file_path}")
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!("absolute path or prefix is forbidden: {file_path}")
            }
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s == ".git" {
                    bail!("modification of .git directory is strictly forbidden: {file_path}");
                }
            }
        }
    }

    // Framework boundary protection: build/workflow files forbidden from automated patch sets
    if norm.starts_with(".github/")
        || norm.starts_with(".cargo/")
        || norm.starts_with("xtask/")
        || norm == "build.rs"
        || norm.ends_with("/build.rs")
    {
        bail!("framework boundary violation: {file_path} is an infrastructure path");
    }

    Ok(())
}

fn resolve_contained_path(root: &Path, rel_path: &str) -> Result<PathBuf> {
    validate_patch_path(rel_path)?;
    let norm = rel_path.replace('\\', "/");
    let candidate = root.join(&norm);

    // Verify candidate does not escape root
    let canon_root = root
        .canonicalize()
        .with_context(|| format!("target root does not exist: {}", root.display()))?;

    // Check if intermediate existing ancestor has symlinks
    let mut check = root.to_path_buf();
    for seg in norm.split('/') {
        check.push(seg);
        if check.exists() {
            let meta = fs::symlink_metadata(&check)?;
            if meta.file_type().is_symlink() {
                bail!("symlink detected in patch target path: {}", check.display());
            }
        }
    }

    // If destination exists, ensure canonicalized path starts with canon_root
    if candidate.exists() {
        let canon_candidate = candidate.canonicalize()?;
        if !canon_candidate.starts_with(&canon_root) {
            bail!("target path escaped root: {}", candidate.display());
        }
    }

    Ok(candidate)
}

// ============================================================================
// Virtual Patch Construction & Verification
// ============================================================================

pub struct RawPatchInput {
    pub file_path: String,
    pub base_digest: Option<String>,
    pub search_block: String,
    pub replace_block: String,
}

pub fn create_patch_set(
    patch_set_id: &str,
    task_id: &str,
    base_revision: &str,
    now_secs: u64,
    provenance: PatchProvenance,
    raw_patches: Vec<RawPatchInput>,
) -> Result<VirtualPatchSet> {
    if raw_patches.is_empty() {
        bail!("virtual patch set must contain at least one patch");
    }
    if raw_patches.len() > 64 {
        bail!("virtual patch set exceeds 64 patch limit");
    }

    let mut patches = Vec::with_capacity(raw_patches.len());
    for (idx, raw) in raw_patches.into_iter().enumerate() {
        validate_patch_path(&raw.file_path)?;
        let hunk_digest = compute_hunk_digest(&raw.search_block, &raw.replace_block);
        patches.push(VirtualPatch {
            patch_index: idx,
            file_path: raw.file_path,
            base_digest: raw.base_digest,
            search_block: raw.search_block,
            replace_block: raw.replace_block,
            hunk_digest,
        });
    }

    let patch_set_digest = compute_patch_set_digest(patch_set_id, task_id, base_revision, &patches);

    Ok(VirtualPatchSet {
        schema_version: SCHEMA_VERSION,
        artifact_kind: ARTIFACT_KIND.into(),
        patch_set_id: patch_set_id.into(),
        task_id: task_id.into(),
        base_revision: base_revision.into(),
        created_at_secs: now_secs,
        patches,
        patch_set_digest,
        provenance,
    })
}

pub fn verify_patch_set(patch_set: &VirtualPatchSet) -> Result<()> {
    if patch_set.schema_version != SCHEMA_VERSION {
        bail!("unsupported schema version: {}", patch_set.schema_version);
    }
    if patch_set.artifact_kind != ARTIFACT_KIND {
        bail!("invalid artifact kind: {}", patch_set.artifact_kind);
    }
    if patch_set.patches.is_empty() {
        bail!("patch set has zero patches");
    }

    for (idx, patch) in patch_set.patches.iter().enumerate() {
        if patch.patch_index != idx {
            bail!(
                "patch index mismatch at position {idx}: {}",
                patch.patch_index
            );
        }
        validate_patch_path(&patch.file_path)?;
        let expected_hunk_digest = compute_hunk_digest(&patch.search_block, &patch.replace_block);
        if patch.hunk_digest != expected_hunk_digest {
            bail!(
                "hunk digest verification failed for patch {idx} ({}): expected {expected_hunk_digest}, got {}",
                patch.file_path,
                patch.hunk_digest
            );
        }
    }

    let expected_patch_set_digest = compute_patch_set_digest(
        &patch_set.patch_set_id,
        &patch_set.task_id,
        &patch_set.base_revision,
        &patch_set.patches,
    );
    if patch_set.patch_set_digest != expected_patch_set_digest {
        bail!(
            "patch set digest mismatch: expected {expected_patch_set_digest}, got {}",
            patch_set.patch_set_digest
        );
    }

    Ok(())
}

// ============================================================================
// Patch Application (In-Memory) & Unified Diff Rendering
// ============================================================================

pub fn apply_patch_in_memory(content: &str, patch: &VirtualPatch) -> Result<String> {
    if patch.search_block.is_empty() {
        // Complete file creation or replacement
        return Ok(patch.replace_block.clone());
    }

    let matches = content.matches(&patch.search_block).count();
    if matches == 0 {
        bail!(
            "orphaned search block in '{}': search block not found in base content",
            patch.file_path
        );
    }
    if matches > 1 {
        bail!(
            "ambiguous search block in '{}': search block matched {matches} times; deterministic patch requires exactly 1 match",
            patch.file_path
        );
    }

    Ok(content.replace(&patch.search_block, &patch.replace_block))
}

pub fn render_unified_diff(file_path: &str, original: &str, modified: &str) -> String {
    let mut diff = String::new();
    diff.push_str(&format!("--- a/{file_path}\n"));
    diff.push_str(&format!("+++ b/{file_path}\n"));

    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();

    diff.push_str(&format!(
        "@@ -1,{} +1,{} @@\n",
        orig_lines.len(),
        mod_lines.len()
    ));
    for line in &orig_lines {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &mod_lines {
        diff.push_str(&format!("+{line}\n"));
    }
    diff
}

/// Scans all Rust patches in the patch set for Aaroneous floor rule violations.
///
/// Returns a list of (file path, violations) found in the replacement blocks.
pub fn check_patch_set_conformance(
    patch_set: &VirtualPatchSet,
) -> Vec<(String, Vec<crate::conformance::Violation>)> {
    check_selected_patch_conformance(patch_set, None)
}

/// Like [`check_patch_set_conformance`], but when `selected` is `Some`, only patches whose
/// `patch_index` is in that list are scanned, so a violation in a patch the reviewer did not
/// select cannot block materializing the ones they did.
pub fn check_selected_patch_conformance(
    patch_set: &VirtualPatchSet,
    selected: Option<&[usize]>,
) -> Vec<(String, Vec<crate::conformance::Violation>)> {
    let mut violations = Vec::new();
    for patch in &patch_set.patches {
        if selected.is_some_and(|indices| !indices.contains(&patch.patch_index)) {
            continue;
        }
        if patch.file_path.ends_with(".rs") {
            let hits = crate::conformance::scan_draft(&patch.replace_block);
            if !hits.is_empty() {
                violations.push((patch.file_path.clone(), hits));
            }
        }
    }
    violations
}

// ============================================================================
// Reviewer-Selected Materialization (No Git Mutation)
// ============================================================================

/// Materializes approved virtual patches into an explicitly supplied target root.
///
/// Guarantees:
/// - Rejects unapproved or needs-changes reviewer selections.
/// - Validates all digests (stale base rejection).
/// - Enforces path containment (traversal, symlink, forbidden directories).
/// - Runs atomic dry-run before making any disk modifications.
/// - **Zero Git mutations**: never runs git commands, creates branches, commits, or alters .git.
pub fn materialize(
    target_root: &Path,
    patch_set: &VirtualPatchSet,
    selection: &ReviewSelection,
) -> Result<MaterializationResult> {
    verify_patch_set(patch_set)?;

    let conformance_violations =
        check_selected_patch_conformance(patch_set, Some(&selection.selected_patch_indices));
    if !conformance_violations.is_empty() {
        let summary = conformance_violations
            .iter()
            .map(|(path, hits)| {
                format!(
                    "{path}: {} violation(s) [{}]",
                    hits.len(),
                    hits.iter()
                        .map(|h| format!("line {}: {}", h.line, h.rule))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        bail!("materialization blocked: patch set contains conformance violation(s): {summary}");
    }

    if selection.disposition != ReviewDisposition::Approved {
        bail!(
            "cannot materialize patch set with disposition '{:?}': must be Approved by independent reviewer",
            selection.disposition
        );
    }

    if selection.selected_patch_indices.is_empty() {
        bail!("no patches selected for materialization");
    }

    if !target_root.exists() {
        bail!(
            "target root directory does not exist: {}",
            target_root.display()
        );
    }

    // Step 1: Pre-flight dry-run of all selected patches
    struct PlannedWrite {
        patch_index: usize,
        target_path: PathBuf,
        relative_path: String,
        new_content: String,
        is_new_file: bool,
        original_content: String,
    }

    let mut planned = Vec::with_capacity(selection.selected_patch_indices.len());

    for &patch_idx in &selection.selected_patch_indices {
        let patch = patch_set
            .patches
            .iter()
            .find(|p| p.patch_index == patch_idx)
            .with_context(|| format!("selected patch index {patch_idx} not found in patch set"))?;

        let target_path = resolve_contained_path(target_root, &patch.file_path)?;

        if target_path.exists() {
            let meta = fs::symlink_metadata(&target_path)?;
            if meta.file_type().is_symlink() {
                bail!(
                    "refusing to modify symlink target: {}",
                    target_path.display()
                );
            }

            let current_content = fs::read_to_string(&target_path).with_context(|| {
                format!("failed to read existing file: {}", target_path.display())
            })?;

            let current_digest = sha256_digest(current_content.as_bytes());

            match &patch.base_digest {
                Some(expected) => {
                    if &current_digest != expected {
                        bail!(
                            "stale-base rejection on '{}': expected base digest {expected}, but target file digest is {current_digest}",
                            patch.file_path
                        );
                    }
                }
                None => {
                    bail!(
                        "stale-base rejection on '{}': patch expects new file, but target already exists",
                        patch.file_path
                    );
                }
            }

            let new_content = apply_patch_in_memory(&current_content, patch)?;
            planned.push(PlannedWrite {
                patch_index: patch_idx,
                target_path,
                relative_path: patch.file_path.clone(),
                new_content,
                is_new_file: false,
                original_content: current_content,
            });
        } else {
            // New file creation
            if patch.base_digest.is_some() {
                bail!(
                    "stale-base rejection on '{}': patch expects existing base digest, but file does not exist",
                    patch.file_path
                );
            }

            let new_content = apply_patch_in_memory("", patch)?;
            planned.push(PlannedWrite {
                patch_index: patch_idx,
                target_path,
                relative_path: patch.file_path.clone(),
                new_content,
                is_new_file: true,
                original_content: String::new(),
            });
        }
    }

    // Step 2: Atomic materialization write phase
    let mut files_modified = Vec::new();
    let mut files_created = Vec::new();
    let mut unified_diff = String::new();
    let mut applied_patches = Vec::new();

    for plan in planned {
        if let Some(parent) = plan.target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&plan.target_path, &plan.new_content).with_context(|| {
            format!(
                "failed to write materialized file: {}",
                plan.target_path.display()
            )
        })?;

        applied_patches.push(plan.patch_index);
        let diff = render_unified_diff(
            &plan.relative_path,
            &plan.original_content,
            &plan.new_content,
        );
        unified_diff.push_str(&diff);

        if plan.is_new_file {
            files_created.push(plan.relative_path);
        } else {
            files_modified.push(plan.relative_path);
        }
    }

    Ok(MaterializationResult {
        patch_set_id: patch_set.patch_set_id.clone(),
        applied_patches,
        unified_diff,
        files_modified,
        files_created,
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sha256_known_vectors() {
        assert_eq!(
            sha256_digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_path_validation_traversal_and_boundaries() {
        assert!(validate_patch_path("src/lib.rs").is_ok());
        assert!(validate_patch_path("crates/demo/tests/test.rs").is_ok());

        assert!(validate_patch_path("../target").is_err());
        assert!(validate_patch_path("src/../../escape").is_err());
        assert!(validate_patch_path("/root/file.rs").is_err());
        assert!(validate_patch_path("C:\\windows\\file.rs").is_err());
        assert!(validate_patch_path(".git/HEAD").is_err());
        assert!(validate_patch_path(".github/workflows/ci.yml").is_err());
        assert!(validate_patch_path(".cargo/config.toml").is_err());
        assert!(validate_patch_path("xtask/src/main.rs").is_err());
        assert!(validate_patch_path("build.rs").is_err());
    }

    #[test]
    fn test_create_and_verify_patch_set_integrity() {
        let provenance = PatchProvenance {
            generator_model: "qwen3.5:9b-q6".into(),
            endpoint: Some("http://localhost:11434".into()),
            instruction: "add test function".into(),
            rationale: Some("improves coverage".into()),
        };

        let raw = vec![RawPatchInput {
            file_path: "src/sample.rs".into(),
            base_digest: None,
            search_block: String::new(),
            replace_block: "pub fn sample() -> u32 { 42 }\n".into(),
        }];

        let ps = create_patch_set("PS-001", "TASK-001", "abc1234", 1700000000, provenance, raw)
            .expect("create_patch_set");

        assert!(verify_patch_set(&ps).is_ok());

        // Tamper test
        let mut tampered = ps.clone();
        tampered.patches[0].replace_block = "malicious code".into();
        assert!(verify_patch_set(&tampered).is_err());
    }

    #[test]
    fn test_stale_base_rejection() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("code.rs");
        fs::write(&file_path, "fn hello() {}\n").expect("write");

        let expected_digest = sha256_digest(b"fn hello() {}\n");

        let ps = create_patch_set(
            "PS-STALE",
            "TASK-1",
            "commit-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "replace hello".into(),
                rationale: None,
            },
            vec![RawPatchInput {
                file_path: "code.rs".into(),
                base_digest: Some(expected_digest),
                search_block: "fn hello() {}\n".into(),
                replace_block: "fn hello_world() {}\n".into(),
            }],
        )
        .expect("patch set");

        let selection = ReviewSelection {
            selected_patch_indices: vec![0],
            reviewer: "claude".into(),
            disposition: ReviewDisposition::Approved,
            rationale: "good".into(),
        };

        // Mutate target file to cause stale base rejection
        fs::write(&file_path, "fn modified_by_other_agent() {}\n").expect("modify");

        let err = materialize(dir.path(), &ps, &selection).expect_err("must reject stale base");
        assert!(err.to_string().contains("stale-base rejection"));
    }

    #[test]
    fn test_unselected_violating_patch_does_not_block_selected_patches() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("ok.rs"),
            "pub fn a() {}
",
        )
        .expect("write ok");
        fs::write(
            dir.path().join("bad.rs"),
            "pub fn b() {}
",
        )
        .expect("write bad");
        let ps = create_patch_set(
            "PS-SEL",
            "TASK-SEL",
            "commit-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "two patches".into(),
                rationale: None,
            },
            vec![
                RawPatchInput {
                    file_path: "ok.rs".into(),
                    base_digest: Some(sha256_digest(
                        b"pub fn a() {}
",
                    )),
                    search_block: "pub fn a() {}
"
                    .into(),
                    replace_block: "pub fn a2() {}
"
                    .into(),
                },
                RawPatchInput {
                    file_path: "bad.rs".into(),
                    base_digest: Some(sha256_digest(
                        b"pub fn b() {}
",
                    )),
                    search_block: "pub fn b() {}
"
                    .into(),
                    replace_block: "pub fn b(x: Option<u8>) -> u8 { x.unwrap() }
"
                    .into(),
                },
            ],
        )
        .expect("patch set");
        let only_ok = ReviewSelection {
            selected_patch_indices: vec![0],
            reviewer: "claude".into(),
            disposition: ReviewDisposition::Approved,
            rationale: "only the conforming patch".into(),
        };
        materialize(dir.path(), &ps, &only_ok).expect("selected conforming patch materializes");
        let bad_selected = ReviewSelection {
            selected_patch_indices: vec![1],
            ..only_ok
        };
        let err =
            materialize(dir.path(), &ps, &bad_selected).expect_err("selected violation blocks");
        assert!(err.to_string().contains("conformance violation"));
    }

    #[test]
    fn test_partial_selection_materialization() {
        let dir = tempdir().expect("tempdir");
        let f1 = dir.path().join("f1.rs");
        let f2 = dir.path().join("f2.rs");
        fs::write(&f1, "let a = 1;\n").expect("write f1");
        fs::write(&f2, "let b = 2;\n").expect("write f2");

        let d1 = sha256_digest(b"let a = 1;\n");
        let d2 = sha256_digest(b"let b = 2;\n");

        let ps = create_patch_set(
            "PS-PARTIAL",
            "TASK-2",
            "commit-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "update variables".into(),
                rationale: None,
            },
            vec![
                RawPatchInput {
                    file_path: "f1.rs".into(),
                    base_digest: Some(d1),
                    search_block: "let a = 1;\n".into(),
                    replace_block: "let a = 100;\n".into(),
                },
                RawPatchInput {
                    file_path: "f2.rs".into(),
                    base_digest: Some(d2),
                    search_block: "let b = 2;\n".into(),
                    replace_block: "let b = 200;\n".into(),
                },
            ],
        )
        .expect("patch set");

        // Select ONLY patch index 1 (f2.rs), leaving patch index 0 (f1.rs) unselected
        let selection = ReviewSelection {
            selected_patch_indices: vec![1],
            reviewer: "reviewer".into(),
            disposition: ReviewDisposition::Approved,
            rationale: "only f2 is approved".into(),
        };

        let result = materialize(dir.path(), &ps, &selection).expect("materialize");
        assert_eq!(result.applied_patches, vec![1]);
        assert_eq!(fs::read_to_string(&f1).unwrap(), "let a = 1;\n");
        assert_eq!(fs::read_to_string(&f2).unwrap(), "let b = 200;\n");
    }

    #[test]
    fn test_reviewer_rejection_prevents_materialization() {
        let dir = tempdir().expect("tempdir");
        let ps = create_patch_set(
            "PS-REJECT",
            "TASK-3",
            "commit-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "do something".into(),
                rationale: None,
            },
            vec![RawPatchInput {
                file_path: "new.rs".into(),
                base_digest: None,
                search_block: String::new(),
                replace_block: "fn bad() {}\n".into(),
            }],
        )
        .expect("patch set");

        let selection = ReviewSelection {
            selected_patch_indices: vec![0],
            reviewer: "reviewer".into(),
            disposition: ReviewDisposition::Rejected,
            rationale: "unsafe design".into(),
        };

        let err =
            materialize(dir.path(), &ps, &selection).expect_err("must not materialize rejected");
        assert!(
            err.to_string()
                .contains("cannot materialize patch set with disposition 'Rejected'")
        );
    }

    #[test]
    fn test_zero_git_mutation_invariant() {
        let dir = tempdir().expect("tempdir");
        // Initialize dummy git repository
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).expect("create .git");
        let head_file = git_dir.join("HEAD");
        fs::write(&head_file, "ref: refs/heads/main\n").expect("write HEAD");
        let head_before = fs::read(&head_file).expect("read HEAD");

        let ps = create_patch_set(
            "PS-GIT-INVARIANT",
            "TASK-GIT",
            "git-rev-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "create new module".into(),
                rationale: None,
            },
            vec![RawPatchInput {
                file_path: "src/new_mod.rs".into(),
                base_digest: None,
                search_block: String::new(),
                replace_block: "pub fn mod_func() {}\n".into(),
            }],
        )
        .expect("patch set");

        let selection = ReviewSelection {
            selected_patch_indices: vec![0],
            reviewer: "auditor".into(),
            disposition: ReviewDisposition::Approved,
            rationale: "clean addition".into(),
        };

        let result = materialize(dir.path(), &ps, &selection).expect("materialize");
        assert_eq!(result.files_created, vec!["src/new_mod.rs"]);

        // Assert .git was byte-for-byte unchanged
        let head_after = fs::read(&head_file).expect("read HEAD after");
        assert_eq!(head_before, head_after);
        assert!(!git_dir.join("index").exists());
    }

    #[test]
    fn test_conformance_violations_block_materialization() {
        let dir = tempdir().expect("tempdir");
        let f1 = dir.path().join("dirty.rs");
        fs::write(&f1, "fn ok() {}\n").expect("write f1");

        let ps = create_patch_set(
            "PS-NONCONFORMING",
            "TASK-VIOLATION",
            "commit-1",
            1000,
            PatchProvenance {
                generator_model: "qwen".into(),
                endpoint: None,
                instruction: "add bad unwrap".into(),
                rationale: None,
            },
            vec![RawPatchInput {
                file_path: "dirty.rs".into(),
                base_digest: Some(sha256_digest(b"fn ok() {}\n")),
                search_block: "fn ok() {}\n".into(),
                replace_block: "fn ok() { let x: Option<u8> = None; x.unwrap(); }\n".into(),
            }],
        )
        .expect("patch set");

        let violations = check_patch_set_conformance(&ps);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, "dirty.rs");
        assert_eq!(violations[0].1[0].rule, "panic-on-input");

        let selection = ReviewSelection {
            selected_patch_indices: vec![0],
            reviewer: "reviewer".into(),
            disposition: ReviewDisposition::Approved,
            rationale: "attempting bad materialization".into(),
        };

        let err =
            materialize(dir.path(), &ps, &selection).expect_err("must block nonconforming patch");
        assert!(
            err.to_string()
                .contains("materialization blocked: patch set contains conformance violation(s)")
        );
    }
}
