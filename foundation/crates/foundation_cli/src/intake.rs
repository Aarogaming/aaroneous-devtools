use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

// Both queue consumers must see the complete queue, including active claims near
// its end. Standardized at 1 MiB (matching PR #258 / Lane C172). Keep a finite
// limit so an oversized file fails closed.
pub(crate) const MAX_COORDINATION_QUEUE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proposal {
    pub schema_version: u32,
    pub id: String,
    pub source: String,
    pub priority: String,
    pub scope: Vec<String>,
    pub dependencies: Vec<String>,
    pub verification: Vec<String>,
    pub decision: String,
}

pub(crate) fn relative_file(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && !value.contains(['\\', ':', '|', '\n', '\r'])
        && !value.chars().any(char::is_whitespace)
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && value.contains('.')
}

impl Proposal {
    pub(crate) fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.id.is_empty()
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || !relative_file(&self.source)
            || !matches!(self.priority.as_str(), "P0" | "P1" | "P2" | "P3")
            || self.scope.is_empty()
            || self.scope.len() > 16
            || self.scope.iter().any(|s| !relative_file(s))
            || self.verification.is_empty()
            || self.verification.len() > 16
            || self.verification.iter().any(|s| s.trim().is_empty())
            || self.dependencies.iter().any(|s| s.trim().is_empty())
            || self.decision.trim().is_empty()
        {
            bail!(
                "proposal requires a valid ID, priority, exact relative file paths and nonempty verification/decision"
            );
        }
        Ok(())
    }
}

pub(crate) fn read_bounded(path: &Path, limit: u64) -> Result<String> {
    let mut data = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut data)?;
    if data.len() as u64 > limit {
        bail!("input exceeds {limit} bytes: {}", path.display());
    }
    Ok(data)
}

pub(crate) fn validate(input: &Path, output: &Path) -> Result<()> {
    if !input.is_absolute() || !output.is_absolute() {
        bail!("proposal and output must be absolute");
    }
    crate::require_absolute_directory("output parent", output.parent().context("output parent")?)?;
    let proposal: Proposal = serde_json::from_str(&read_bounded(input, 65536)?)?;
    proposal.check()?;
    fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version":1,"artifact_kind":"foundation.intake-validation",
            "structurally_valid":true,"requires_owner_review":true,"proposal":proposal
        }))?,
    )?;
    Ok(())
}

/// Publish `bytes` as a brand-new file at `path` without ever replacing or
/// writing through an existing one. The bytes are written and synced to a
/// uniquely named staging file in the same directory, which is then linked to
/// `path` with an exclusive hard link: a pre-existing destination (including a
/// hard link to some other file) is refused and left untouched, and no
/// partial final file is ever exposed. Requires a filesystem with hard links.
pub(crate) fn publish_new_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Staging(PathBuf);
    impl Drop for Staging {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let parent = path.parent().context("output parent")?;
    let mut stage = None;
    for _ in 0..16 {
        let candidate = parent.join(format!(
            ".publish-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                stage = Some((Staging(candidate), file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let (stage, mut file) = stage.context("could not reserve staging file")?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::hard_link(&stage.0, path).with_context(|| {
        format!(
            "output must be new and filesystem must support hard links: {}",
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_prose_and_traversal_scopes() {
        for bad in [
            "Documentation claims regarding performance metrics",
            "../README.md",
            "C:/README.md",
            "/README.md",
            "crates/../README.md",
        ] {
            assert!(!relative_file(bad), "{bad}");
        }
        assert!(relative_file("crates/ipc_bus/src/lib.rs"));
    }
    #[test]
    fn bounded_read_rejects_large_input() {
        let dir = tempfile::tempdir().expect("fixture");
        let file = dir.path().join("evidence.txt");
        fs::write(&file, "12345").expect("write");
        assert!(read_bounded(&file, 4).is_err());
    }

    #[test]
    fn publish_new_bytes_creates_once_and_never_replaces() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("report.json");
        publish_new_bytes(&target, b"first").expect("publish new file");
        assert_eq!(fs::read(&target).expect("read"), b"first");
        let err = publish_new_bytes(&target, b"second").expect_err("existing file is refused");
        assert!(err.to_string().contains("output must be new"));
        assert_eq!(fs::read(&target).expect("read"), b"first");
        let names: Vec<_> = fs::read_dir(dir.path())
            .expect("list")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("report.json")]);
    }

    #[test]
    fn publish_new_bytes_does_not_write_through_a_hard_link() {
        let dir = tempfile::tempdir().expect("tempdir");
        let protected = dir.path().join("Cargo.lock");
        fs::write(&protected, b"original lock bytes").expect("lock");
        let alias = dir.path().join("report.json");
        fs::hard_link(&protected, &alias).expect("hard link");
        publish_new_bytes(&alias, b"report").expect_err("hard-linked destination is refused");
        assert_eq!(fs::read(&protected).expect("lock"), b"original lock bytes");
        assert_eq!(fs::read(&alias).expect("alias"), b"original lock bytes");
    }
}
