//! Local verification file-spool CLI. Every checkout, executable and authority input is explicit.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use steward::local_verification as local;

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} must be a string"))
}
fn read(path: &Path) -> Result<Value, String> {
    if !path.is_absolute() || fs::metadata(path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("bounded absolute input file required".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn run() -> Result<Value, String> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("usage: local_verification issue|run|sign|recover|shadow --name value ...")?;
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !key.starts_with("--") || options.contains_key(&key) {
            return Err("duplicate or malformed option".into());
        }
        options.insert(key, args.next().ok_or("option missing value")?);
    }
    let allowed: &[&str] = match command.as_str() {
        "issue" => &["git", "repo", "repo-name", "head", "base", "suite", "spool"],
        "run" => &["repo", "card", "spool", "node", "config", "trust"],
        "sign" => &["ssh-keygen", "key", "receipt"],
        "recover" => &["spool", "card", "node", "attempt", "old-worker-stopped"],
        "shadow" => &[
            "git",
            "repo",
            "repo-name",
            "head",
            "base",
            "suite",
            "spool",
            "ssh-keygen",
            "allowed-signers",
            "registry",
            "facts",
            "trust",
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
    let get = |name: &str| -> Result<&str, String> {
        options
            .get(&format!("--{name}"))
            .map(String::as_str)
            .ok_or_else(|| format!("missing --{name}"))
    };
    let path = |name: &str| -> Result<PathBuf, String> {
        let p = PathBuf::from(get(name)?);
        if !p.is_absolute() {
            return Err(format!("--{name} must be absolute"));
        }
        Ok(p)
    };
    match command.as_str() {
        "issue" => {
            let store = local::Store::open(&path("spool")?)?;
            let requests = local::issue(
                &path("git")?,
                &path("repo")?,
                get("repo-name")?,
                get("head")?,
                get("base")?,
                get("suite")?,
            )?;
            let cards = requests
                .iter()
                .map(|request| {
                    store.submit(request).map(
                        |path| json!({"id":request.id(), "platform":request.platform, "card":path}),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"cards":cards}))
        }
        "run" => {
            if get("trust")? != "owner-trusted-local" {
                return Err("this adapter is not a public-code sandbox; explicit owner-trusted-local profile required".into());
            }
            let config = read(&path("config")?)?;
            if config.as_object().is_none_or(|map| {
                map.len() != 3
                    || map
                        .keys()
                        .any(|key| !["git", "tools", "environment"].contains(&key.as_str()))
            }) {
                return Err("runner config requires exactly git, tools, environment".into());
            }
            let tools = config["tools"]
                .as_object()
                .ok_or("tools must be object")?
                .iter()
                .map(|(name, value)| {
                    value
                        .as_str()
                        .map(|v| (name.clone(), PathBuf::from(v)))
                        .ok_or("tool path must be string")
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            let environment = config["environment"]
                .as_object()
                .ok_or("environment must be object")?
                .iter()
                .map(|(name, value)| {
                    value
                        .as_str()
                        .map(|v| (name.clone(), v.to_owned()))
                        .ok_or("environment value must be string")
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            let runner = local::Runner {
                git: PathBuf::from(text(&config, "git")?),
                tools,
                environment,
            };
            let store = local::Store::open(&path("spool")?)?;
            let card = local::load_card(&path("card")?)?;
            let claim = store.claim(&card, get("node")?)?;
            let receipt = runner.execute(&claim, &path("repo")?)?;
            Ok(
                json!({"receipt":receipt, "digest":local::receipt_digest(&receipt)?, "attempt":claim.attempt}),
            )
        }
        "sign" => Ok(
            json!({"signature":local::sign(&path("ssh-keygen")?, &path("key")?, &path("receipt")?)?}),
        ),
        "recover" => {
            if get("old-worker-stopped")? != "yes" {
                return Err("stop the prior worker before explicit recovery".into());
            }
            let store = local::Store::open(&path("spool")?)?;
            let card = local::load_card(&path("card")?)?;
            store.recover(
                &card,
                get("node")?,
                get("attempt")?
                    .parse()
                    .map_err(|_| "attempt must be integer")?,
            )?;
            Ok(json!({"recovered":card.id()}))
        }
        "shadow" => {
            if get("trust")? != "owner-trusted-local" {
                return Err("only explicit owner-trusted-local receipts are supported".into());
            }
            let store = local::Store::open(&path("spool")?)?;
            let requests = local::issue(
                &path("git")?,
                &path("repo")?,
                get("repo-name")?,
                get("head")?,
                get("base")?,
                get("suite")?,
            )?;
            let registry: BTreeMap<String, BTreeSet<String>> =
                serde_json::from_value(read(&path("registry")?)?).map_err(|e| e.to_string())?;
            let outcomes = local::ingest(
                &store,
                &path("git")?,
                &path("repo")?,
                &path("ssh-keygen")?,
                &path("allowed-signers")?,
                &requests,
                &registry,
            )?;
            local::shadow_gate(&read(&path("facts")?)?, &requests, &outcomes, &registry)
        }
        _ => unreachable!(),
    }
}
fn main() {
    match run() {
        Ok(output) => println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("JSON value")
        ),
        Err(problem) => {
            eprintln!("local verification blocked: {problem}");
            std::process::exit(1);
        }
    }
}
