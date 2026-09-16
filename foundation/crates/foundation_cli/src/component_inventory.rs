use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const REPORT_KIND: &str = "foundation.component-inventory";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct ComponentInventory {
    schema_version: u32,
    artifact_kind: String,
    checkout: String,
    crate_path: String,
    package_name: String,
    rust_sources: Vec<String>,
    test_sources: Vec<String>,
    dependencies: Vec<String>,
    public_item_lines: usize,
    hot_path_markers: usize,
}

pub(crate) fn run(checkout: &Path, crate_path: &Path, output: &Path) -> Result<()> {
    require_checkout(checkout)?;
    require_safe_relative_path("crate path", crate_path)?;
    let crate_root = checkout.join(crate_path);
    require_directory("crate path", &crate_root)?;
    let manifest_path = crate_root.join("Cargo.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: toml::Value =
        toml::from_str(&manifest_text).context("invalid crate Cargo.toml")?;
    let package_name = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("name"))
        .and_then(toml::Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .context("crate Cargo.toml is missing package.name")?
        .to_owned();
    let dependencies = dependency_names(&manifest);
    let files = rust_files(&crate_root)?;
    let mut public_item_lines = 0;
    let mut hot_path_markers = 0;
    let mut rust_sources = Vec::new();
    let mut test_sources = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&crate_root)
            .context("inventory file escaped declared crate path")?
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        public_item_lines += text
            .lines()
            .filter(|line| line.trim_start().starts_with("pub "))
            .count();
        hot_path_markers += text.matches("#[doc = \"hot_path\"]").count();
        if relative.starts_with("tests/") || relative.contains("/tests/") {
            test_sources.push(relative);
        } else {
            rust_sources.push(relative);
        }
    }
    validate_output(output)?;
    let report = ComponentInventory {
        schema_version: 1,
        artifact_kind: REPORT_KIND.to_owned(),
        checkout: checkout.display().to_string(),
        crate_path: crate_path.to_string_lossy().replace('\\', "/"),
        package_name,
        rust_sources,
        test_sources,
        dependencies,
        public_item_lines,
        hot_path_markers,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("failed to serialize component inventory")?,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;
    println!("wrote component inventory: {}", output.display());
    Ok(())
}

fn require_checkout(checkout: &Path) -> Result<()> {
    require_directory("checkout", checkout)?;
    if !checkout.join(".git").exists() {
        bail!("checkout must contain .git: {}", checkout.display());
    }
    Ok(())
}

fn require_directory(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    if !fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?
        .is_dir()
    {
        bail!("{label} must be a directory: {}", path.display());
    }
    Ok(())
}

fn require_safe_relative_path(label: &str, path: &Path) -> Result<()> {
    if path.is_absolute() || path.as_os_str().is_empty() {
        bail!(
            "{label} must be a non-empty relative path: {}",
            path.display()
        );
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "{label} may contain normal path segments only: {}",
            path.display()
        );
    }
    Ok(())
}

fn validate_output(output: &Path) -> Result<()> {
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output.parent().context("output must have a parent")?;
    require_directory("output parent", parent)
}

fn dependency_names(manifest: &toml::Value) -> Vec<String> {
    let mut names = BTreeSet::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(section).and_then(toml::Value::as_table) {
            names.extend(table.keys().cloned());
        }
    }
    names.into_iter().collect()
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to enumerate {}", directory.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_rust_files(&path, files)?;
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_component_path() {
        assert!(require_safe_relative_path("crate path", Path::new("crates/../ipc_bus")).is_err());
    }

    #[test]
    fn writes_a_sorted_component_report() {
        let temp = tempfile::tempdir().expect("test directory");
        let checkout = temp.path().join("checkout");
        let crate_root = checkout.join("crates/example");
        fs::create_dir_all(crate_root.join("tests")).expect("crate directories");
        fs::create_dir_all(checkout.join(".git")).expect("checkout metadata");
        fs::write(
            crate_root.join("Cargo.toml"),
            "[package]\nname = \"example\"\nversion = \"0.1.0\"\n[dependencies]\nserde = \"1\"\n",
        )
        .expect("manifest");
        fs::write(
            crate_root.join("src.rs"),
            "pub struct Frame;\n#[doc = \"hot_path\"]\npub fn tick() {}\n",
        )
        .expect("source");
        fs::write(
            crate_root.join("tests/contract.rs"),
            "#[test]\nfn contract() {}\n",
        )
        .expect("test source");
        let output = temp.path().join("report.json");
        run(&checkout, Path::new("crates/example"), &output).expect("inventory succeeds");
        let report: ComponentInventory =
            serde_json::from_slice(&fs::read(output).expect("report")).expect("valid report");
        assert_eq!(report.package_name, "example");
        assert_eq!(report.dependencies, vec!["serde"]);
        assert_eq!(report.rust_sources, vec!["src.rs"]);
        assert_eq!(report.test_sources, vec!["tests/contract.rs"]);
        assert_eq!(report.public_item_lines, 2);
        assert_eq!(report.hot_path_markers, 1);
    }
}
