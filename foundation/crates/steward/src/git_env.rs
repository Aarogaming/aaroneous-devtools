//! Removing inherited Git environment variables from a child `git` command (lane C187).
//!
//! The local Git adapters (`local_verification`, `local_authority`, `release_tree`) run `git` with a hardened environment: every
//! inherited `GIT_*` variable removed so a caller cannot redirect Git with `GIT_DIR`, `GIT_WORK_TREE` or `GIT_OBJECT_DIRECTORY`.
//! Environment names are case-insensitive on Windows, so there a lower-case `git_dir` redirects Git exactly as `GIT_DIR` does;
//! on other platforms names are case-sensitive and only the upper-case prefix names a Git variable. [`scrub`] applies that rule in
//! one place for all three adapters.

use std::ffi::{OsStr, OsString};
use std::process::Command;

/// True when `key` names a Git variable on this platform: the `GIT_` prefix, ignoring ASCII case on Windows only.
pub fn is_git_env_key(key: &OsStr) -> bool {
    matches_git_prefix(&key.to_string_lossy(), cfg!(windows))
}

fn matches_git_prefix(key: &str, ignore_ascii_case: bool) -> bool {
    match key.as_bytes().get(..4) {
        Some(prefix) if ignore_ascii_case => prefix.eq_ignore_ascii_case(b"GIT_"),
        Some(prefix) => prefix == b"GIT_",
        None => false,
    }
}

/// Remove every key in `inherited` that names a Git variable from `command`'s environment.
pub fn scrub(command: &mut Command, inherited: impl IntoIterator<Item = OsString>) {
    for key in inherited {
        if is_git_env_key(&key) {
            command.env_remove(key);
        }
    }
}

/// Remove every Git variable of the current process environment from `command`.
pub fn scrub_inherited(command: &mut Command) {
    scrub(command, std::env::vars_os().map(|(key, _)| key));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_case_prefix_is_always_a_git_key() {
        for ignore_case in [false, true] {
            assert!(matches_git_prefix("GIT_DIR", ignore_case));
            assert!(matches_git_prefix("GIT_", ignore_case));
            assert!(matches_git_prefix("GIT_WORK_TREE", ignore_case));
        }
    }

    #[test]
    fn mixed_and_lower_case_prefix_follows_the_platform_rule() {
        for key in ["git_dir", "Git_Dir", "gIT_OBJECT_DIRECTORY"] {
            assert!(
                matches_git_prefix(key, true),
                "{key} must match where names are case-insensitive"
            );
            assert!(
                !matches_git_prefix(key, false),
                "{key} must not match where names are case-sensitive"
            );
        }
    }

    #[test]
    fn near_misses_and_short_or_non_ascii_keys_do_not_match() {
        for ignore_case in [false, true] {
            for key in [
                "", "GIT", "GI", "GITX", "GIT-DIR", "XGIT_DIR", "PATH", "GÏT_DIR", "ÿ",
            ] {
                assert!(!matches_git_prefix(key, ignore_case), "{key:?}");
            }
        }
    }

    fn removed_keys(inherited: &[&str]) -> Vec<String> {
        let mut command = Command::new("git");
        scrub(&mut command, inherited.iter().map(OsString::from));
        command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn scrub_marks_exactly_the_git_keys_for_removal() {
        let removed = removed_keys(&["GIT_DIR", "PATH", "GIT_WORK_TREE"]);
        assert!(removed.iter().any(|key| key == "GIT_DIR"));
        assert!(removed.iter().any(|key| key == "GIT_WORK_TREE"));
        assert!(!removed.iter().any(|key| key == "PATH"));
    }

    #[test]
    fn scrub_removes_lower_case_git_dir_where_names_are_case_insensitive() {
        // Separate command: on Windows `Command` folds `git_dir` and `GIT_DIR` into one entry, so mixing them would hide the key.
        let removed = removed_keys(&["git_dir"]);
        assert_eq!(
            removed
                .iter()
                .any(|key| key.eq_ignore_ascii_case("git_dir")),
            cfg!(windows)
        );
    }
}
