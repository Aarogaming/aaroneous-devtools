use crate::dogfood::find_prebuilt_binary;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn launch(checkout: &Path, host: &str, port: u16) -> Result<()> {
    if !checkout.is_absolute() {
        bail!("checkout path must be an absolute directory");
    }
    if !checkout.exists() {
        bail!("checkout directory '{}' does not exist", checkout.display());
    }

    let bin_path = match find_prebuilt_binary("hypervisor", checkout) {
        Some(path) => path,
        None => {
            println!(
                "hypervisor binary not found in prebuilt cache; compiling hypervisor out-of-tree..."
            );
            let status = Command::new("cargo")
                .args(["build", "-p", "hypervisor", "--bin", "hypervisor"])
                .current_dir(checkout)
                .status()
                .context("failed to execute cargo build for hypervisor")?;
            if !status.success() {
                bail!(
                    "cargo build -p hypervisor failed with exit status {:?}",
                    status.code()
                );
            }
            find_prebuilt_binary("hypervisor", checkout)
                .context("hypervisor binary still missing after cargo build")?
        }
    };

    println!("Launching Aaroneous Hypervisor MCP Server:");
    println!("  Binary: {}", bin_path.display());
    println!("  Target: http://{host}:{port}");

    let mut child = Command::new(&bin_path)
        .args(["mcp", "--host", host, "--port", &port.to_string()])
        .current_dir(checkout)
        .spawn()
        .context("failed to spawn hypervisor mcp daemon")?;

    println!("hypervisor mcp daemon running with PID {}", child.id());
    println!("Press Ctrl+C to stop hypervisor mcp daemon...");

    let status = child
        .wait()
        .context("failed waiting for hypervisor mcp daemon")?;
    println!("hypervisor mcp daemon process exited with status: {status}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_launch_rejects_relative_checkout() {
        let err = launch(Path::new("relative/path"), "127.0.0.1", 8766).unwrap_err();
        assert!(err.to_string().contains("absolute directory"));
    }

    #[test]
    fn test_launch_rejects_nonexistent_checkout() {
        let temp = tempdir().unwrap();
        let non_existent = temp.path().join("does_not_exist");
        let err = launch(&non_existent, "127.0.0.1", 8766).unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }
}
