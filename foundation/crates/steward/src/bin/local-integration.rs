use std::collections::BTreeMap;
use std::path::PathBuf;
use steward::local_authority::{self, Inputs};
fn run() -> Result<serde_json::Value, String> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments
        .next()
        .ok_or("usage: local-integration sign|collect --name value ...")?;
    let mut options = BTreeMap::new();
    while let Some(name) = arguments.next() {
        if !name.starts_with("--") || options.contains_key(&name) {
            return Err("malformed or duplicate option".into());
        }
        options.insert(name, arguments.next().ok_or("missing option value")?);
    }
    let allowed: &[&str] = match command.as_str() {
        "sign" => &["ssh-keygen", "key", "document"],
        "collect" => &[
            "git",
            "ssh-keygen",
            "repo",
            "repo-name",
            "base-ref",
            "head-ref",
            "authority",
            "state",
            "spool",
        ],
        _ => return Err("unknown command".into()),
    };
    if options.len() != allowed.len()
        || options
            .keys()
            .any(|key| !allowed.iter().any(|name| key == &format!("--{name}")))
    {
        return Err("unknown or missing options".into());
    }
    let get = |name: &str| {
        options
            .get(&format!("--{name}"))
            .cloned()
            .ok_or_else(|| format!("missing --{name}"))
    };
    let path = |name: &str| -> Result<PathBuf, String> {
        let path = PathBuf::from(get(name)?);
        if !path.is_absolute() {
            return Err("paths must be absolute".into());
        }
        Ok(path)
    };
    if command == "sign" {
        return Ok(
            serde_json::json!({"signature":local_authority::sign(&path("ssh-keygen")?,&path("key")?,&path("document")?)?}),
        );
    }
    local_authority::collect(&Inputs {
        git: path("git")?,
        ssh_keygen: path("ssh-keygen")?,
        repo: path("repo")?,
        repo_name: get("repo-name")?,
        base_ref: get("base-ref")?,
        head_ref: get("head-ref")?,
        authority: path("authority")?,
        state: path("state")?,
        spool: path("spool")?,
    })
}
fn main() {
    match run() {
        Ok(value) => println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("JSON value")
        ),
        Err(problem) => {
            eprintln!("local integration blocked: {problem}");
            std::process::exit(1);
        }
    }
}
