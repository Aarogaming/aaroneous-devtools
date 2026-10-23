//! `release-plan <git> <repo> <commit> <policy.toml>`: a read-only dry run of the release export for one commit. Prints the JSON
//! summary from `release_tree::summarize`; exits 0 when nothing blocks a release, 2 when unsafe or secret-shaped paths would, 1 on
//! any error. It reads the repository and the policy file and writes nothing.

use std::path::PathBuf;
use std::process::ExitCode;
use steward::release::{ExportPolicy, Withheld};
use steward::release_tree::{plan_commit, summarize};

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [git, repo, commit, policy_path] = args.as_slice() else {
        return Err("usage: release-plan <absolute git path> <absolute repo path> <full commit sha> <policy.toml>".to_owned());
    };
    let text =
        std::fs::read_to_string(policy_path).map_err(|e| format!("cannot read the policy: {e}"))?;
    let policy = ExportPolicy::parse(&text)?;
    let plan = plan_commit(&PathBuf::from(git), &PathBuf::from(repo), commit, &policy)?;
    println!("{}", summarize(&plan));
    Ok(!plan
        .withheld
        .iter()
        .any(|(_, w)| matches!(w, Withheld::Unsafe(_) | Withheld::Sensitive(_))))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(message) => {
            eprintln!("release-plan: {message}");
            ExitCode::from(1)
        }
    }
}
