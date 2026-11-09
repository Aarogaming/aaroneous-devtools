//! `review-carry <git> <repo> <reviewed> <current> <main> [<reviewed-base>]`: a read-only report of which files of a pull request
//! are still byte-identical to the head a reviewer saw (lane C190). Every commit is a full lower-case object id. Prints the JSON
//! summary from `review_carry::summarize`; exits 0 when every path of the pull request's own change is carried, 2 when at least
//! one path needs a fresh look, 1 on any error. It reads the repository and writes nothing.

use std::path::PathBuf;
use std::process::ExitCode;
use steward::review_carry::{compare_commits, summarize};

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (git, repo, reviewed, current, main, base) = match args.as_slice() {
        [git, repo, reviewed, current, main] => (git, repo, reviewed, current, main, None),
        [git, repo, reviewed, current, main, base] => {
            (git, repo, reviewed, current, main, Some(base.as_str()))
        }
        _ => {
            return Err("usage: review-carry <absolute git path> <absolute repo path> <reviewed sha> <current sha> <main sha> [<reviewed base sha>]".to_owned());
        }
    };
    let report = compare_commits(
        &PathBuf::from(git),
        &PathBuf::from(repo),
        reviewed,
        base,
        current,
        main,
    )?;
    println!("{}", summarize(&report));
    Ok(report.all_carried())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(message) => {
            eprintln!("review-carry: {message}");
            ExitCode::from(1)
        }
    }
}
