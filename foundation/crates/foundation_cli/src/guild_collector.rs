//! Native Rust GitHub state collector for Guild scheduling (lane C116 / Area 1).
//!
//! Replaces `scripts/collect_state.py` with pure-Rust collection:
//! - Queries `gh` for open pull requests and metadata
//! - Queries GraphQL review threads for unresolved / unanswered counts
//! - Queries git merge bases, ancestry, and trailers (`git-author:`, `co-authored-by:`, `contribution-id:`)
//! - Evaluates review coverage via `steward::coverage`
//! - Calculates rejected verdicts via `steward::rejected`
//! - Calculates completed cards via `steward::receipts`
//! - Emits the canonical `state` JSON for `steward::cards::build_cards`

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::maintenance_pipeline::{format_iso8601, unix_secs_now};

const THREADS_QUERY: &str = "\
query($owner:String!,$name:String!){\
  repository(owner:$owner,name:$name){\
    pullRequests(states:OPEN,first:100){\
      pageInfo{hasNextPage}\
      nodes{\
        number\
        reviewThreads(first:100){\
          pageInfo{hasNextPage}\
          nodes{\
            isResolved\
            comments(last:1){\
              totalCount\
              nodes{\
                author{login}\
                body\
              }\
            }\
          }\
        }\
      }\
    }\
  }\
}";

const NAMESPACE: &str = "aaroneous-attestation";

/// Check if a repository clone remote URL matches the expected `owner/repo` slug.
pub fn checkout_matches(remote_url: &str, repo: &str) -> bool {
    let mut cleaned = remote_url.trim().trim_end_matches('/');
    if let Some(stripped) = cleaned.strip_suffix(".git") {
        cleaned = stripped.trim_end_matches('/');
    }
    let slash = format!("/{repo}");
    let colon = format!(":{repo}");
    cleaned.ends_with(&slash) || cleaned.ends_with(&colon)
}

/// Run a git command in `repo_dir` returning stdout string on success.
pub fn run_git(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .output()
        .with_context(|| format!("failed to invoke git -C {:?} {:?}", repo_dir, args))?;
    if !out.status.success() {
        bail!(
            "git command failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Run a `gh` CLI command returning parsed JSON Value on success.
pub fn run_gh_json(args: &[&str]) -> Result<Value> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .with_context(|| format!("failed to invoke gh {:?}", args))?;
    if !out.status.success() {
        bail!(
            "gh command failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let val: Value = serde_json::from_slice(&out.stdout)
        .with_context(|| format!("malformed JSON response from gh {:?}", args))?;
    Ok(val)
}

/// Helper to flatten paginated JSON arrays from `gh api --paginate --slurp`.
pub fn flatten_pages(val: &Value) -> Vec<Value> {
    let mut items = Vec::new();
    if let Some(arr) = val.as_array() {
        for page in arr {
            if let Some(page_arr) = page.as_array() {
                items.extend(page_arr.clone());
            } else {
                items.push(page.clone());
            }
        }
    } else {
        items.push(val.clone());
    }
    items
}

/// Parse seat names from git commit log trailers and branch name.
pub fn seats_from_trailers(text: &str, branch: &str) -> Vec<String> {
    let mut seats: Vec<String> = Vec::new();
    let mut add = |seat: &str| {
        let lowered = seat.to_lowercase();
        if !lowered.is_empty() && !seats.contains(&lowered) {
            seats.push(lowered);
        }
    };

    if !branch.is_empty() {
        let prefix = branch.split('/').next().unwrap_or("");
        let found = steward::seats::seat_names(prefix);
        for s in found {
            add(&s);
        }
    }

    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("co-authored-by:")
            || lower.starts_with("contribution-id:")
            || lower.starts_with("git-author:")
        {
            let found = steward::seats::seat_names(&line[line.find(':').unwrap_or(0) + 1..]);
            for s in found {
                add(&s);
            }
        }
    }
    seats
}

pub const KNOWN_KEY_TYPES: &[&str] = &[
    "ssh-ed25519",
    "ssh-rsa",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "sk-ssh-ed25519@openssh.com",
    "sk-ecdsa-sha2-nistp256@openssh.com",
];

pub fn is_plain_login(val: &str) -> bool {
    !val.is_empty() && val == val.trim() && !val.chars().any(char::is_whitespace)
}

pub fn is_valid_key(key: &str) -> bool {
    let parts: Vec<&str> = key.split_whitespace().collect();
    if parts.len() < 2 {
        return false;
    }
    if !KNOWN_KEY_TYPES.contains(&parts[0]) {
        return false;
    }
    let body = parts[1];
    let key_body_re = match Regex::new(r"^[A-Za-z0-9+/]+={0,2}$") {
        Ok(re) => re,
        Err(_) => return false,
    };
    key_body_re.is_match(body)
}

/// Validate identities JSON matching governance/identities.json specification.
pub fn validate_identities(data: &Value) -> Result<()> {
    let mut problems = Vec::new();
    let Some(seats) = data.get("seats").and_then(Value::as_object) else {
        bail!("the file must be a JSON object with a \"seats\" object");
    };

    let mut owner_set = BTreeSet::new();
    if let Some(owners) = data.get("owner") {
        if let Some(owners_arr) = owners.as_array() {
            let mut all_plain = true;
            for o in owners_arr {
                if let Some(s) = o.as_str() {
                    if is_plain_login(s) {
                        owner_set.insert(s.to_lowercase());
                    } else {
                        all_plain = false;
                        break;
                    }
                } else {
                    all_plain = false;
                    break;
                }
            }
            if !all_plain {
                problems.push(
                    "\"owner\" must be a list of plain GitHub logins (no spaces)".to_string(),
                );
                owner_set.clear();
            }
        } else {
            problems
                .push("\"owner\" must be a list of plain GitHub logins (no spaces)".to_string());
        }
    }

    let seat_re = Regex::new(r"^[a-z][a-z0-9_-]{0,31}$").unwrap();
    let mut seen_logins: BTreeMap<String, String> = BTreeMap::new();

    for (seat, entry) in seats {
        if !seat_re.is_match(seat) {
            problems.push(format!(
                "seat name {seat:?} must be lower-case letters, digits, '-' or '_', starting with a letter"
            ));
            continue;
        }

        let logins = match entry.get("logins") {
            Some(Value::Array(arr)) => arr.clone(),
            Some(other) => {
                problems.push(format!("{seat}: logins must be a list, not {other}"));
                Vec::new()
            }
            None => {
                if let Some(arr) = entry.as_array() {
                    arr.clone()
                } else {
                    Vec::new()
                }
            }
        };

        let keys = match entry.get("keys") {
            Some(Value::Array(arr)) => arr.clone(),
            Some(other) => {
                problems.push(format!("{seat}: keys must be a list, not {other}"));
                Vec::new()
            }
            None => Vec::new(),
        };

        for login_val in logins {
            let Some(login) = login_val.as_str() else {
                problems.push(format!("{seat}: login is not a string"));
                continue;
            };
            if !is_plain_login(login) {
                problems.push(format!(
                    "{seat}: login {login:?} is not a plain GitHub login"
                ));
                continue;
            }
            let login_lower = login.to_lowercase();
            if owner_set.contains(&login_lower) {
                problems.push(format!(
                    "{seat}: {login:?} is the shared owner login, which is deliberately not a seat"
                ));
            }
            if let Some(prev_seat) = seen_logins.get(&login_lower) {
                if prev_seat != seat {
                    problems.push(format!(
                        "login {login:?} is listed for both {prev_seat} and {seat}"
                    ));
                }
            }
            seen_logins.insert(login_lower, seat.clone());
        }

        for key_val in keys {
            let Some(key_str) = key_val.as_str() else {
                problems.push(format!("{seat}: key is not a string"));
                continue;
            };
            if !is_valid_key(key_str) {
                let preview: String = key_str.chars().take(40).collect();
                problems.push(format!(
                    "{seat}: key {preview:?} is not '<known key type> <base64>'"
                ));
            }
        }
    }

    if !problems.is_empty() {
        bail!("invalid identities: {}", problems.join("; "));
    }
    Ok(())
}

/// Build OpenSSH allowed_signers string from identities data.
pub fn build_allowed_signers(identities: &Value) -> String {
    let mut lines = Vec::new();
    let Some(seats) = identities.get("seats").and_then(Value::as_object) else {
        return String::new();
    };
    let seat_re = Regex::new(r"^[a-z][a-z0-9_-]{0,31}$").unwrap();

    for (seat, entry) in seats {
        if !seat_re.is_match(seat) {
            continue;
        }
        let keys = entry
            .get("keys")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for key in keys {
            if let Some(key_str) = key.as_str() {
                if is_valid_key(key_str) {
                    let parts: Vec<&str> = key_str.split_whitespace().collect();
                    lines.push(format!(
                        "{seat} namespaces=\"{NAMESPACE}\" {} {}",
                        parts[0], parts[1]
                    ));
                }
            }
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// Verify an attestation signature with ssh-keygen against allowed_signers.
pub fn verify_signature(
    canonical_bytes: &[u8],
    signature: &str,
    seat: &str,
    allowed_signers_text: &str,
) -> bool {
    if signature.trim().is_empty() || allowed_signers_text.trim().is_empty() {
        return false;
    }
    let temp_dir = match tempfile::tempdir() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let allowed_path = temp_dir.path().join("allowed");
    let sig_path = temp_dir.path().join("sig");

    if fs::write(&allowed_path, allowed_signers_text).is_err() {
        return false;
    }
    if fs::write(&sig_path, format!("{}\n", signature.trim())).is_err() {
        return false;
    }

    let mut child = match Command::new("ssh-keygen")
        .args([
            "-Y",
            "verify",
            "-f",
            allowed_path.to_str().unwrap_or(""),
            "-I",
            seat,
            "-n",
            NAMESPACE,
            "-s",
            sig_path.to_str().unwrap_or(""),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(canonical_bytes);
    }

    match child.wait() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Canonicalize an attestation JSON object into deterministic sorted-key JSON bytes.
pub fn canonical_attestation_bytes(rec: &Value) -> Option<Vec<u8>> {
    let obj = rec.as_object()?;
    let sorted: BTreeMap<_, _> = obj.iter().collect();
    serde_json::to_vec(&sorted).ok()
}

/// Parsed attestation results for a single comment.
pub struct ParsedCommentAttestations {
    pub records: Vec<steward::coverage::Record>,
    pub malformed: Vec<steward::rejected::Malformed>,
    pub legacy: Vec<Option<i64>>,
}

/// Parse all attestation blocks, mislabelled fences, and legacy verdicts from comments.
pub fn parse_comment_attestations(
    comments: &[steward::rejected::Comment],
    repo: &str,
    pr_number: i64,
    allowed_signers_text: &str,
    require_signatures: bool,
) -> Result<ParsedCommentAttestations> {
    let pair_re = Regex::new(
        r"(?s)```attestation[ \t]*\n(.*?)\n```(?:[ \t]*\n+```attestation-sig[ \t]*\n(.*?)\n```)?",
    )?;
    let any_fence = Regex::new(r"(?s)```([^\n`]*)\n(.*?)\n```")?;
    let prose_verdict = Regex::new(
        r"(?i)\bVERDICT\b|\bverdict\b[^.\n]{0,40}?\b(?:accept(?:ed)?|reject(?:ed)?|approve[ds]?|changes[ _-]?requested|needs[ _-]?revision)\b",
    )?;
    let full_sha = Regex::new(r"^[0-9a-f]{40}$")?;

    let mut records = Vec::new();
    let mut malformed = Vec::new();
    let mut legacy = Vec::new();

    for (index, comment) in comments.iter().enumerate() {
        let comment_idx = Some(index as i64);
        let normalized = comment.body.replace("\r\n", "\n").replace('\r', "\n");
        let mut found_any = false;

        for caps in pair_re.captures_iter(&normalized) {
            found_any = true;
            let raw_json = caps.get(1).map_or("", |m| m.as_str());
            let sig_text = caps.get(2).map(|m| m.as_str().trim());

            let mut rec: Value = match serde_json::from_str(raw_json) {
                Ok(v) => v,
                Err(e) => {
                    malformed.push(steward::rejected::Malformed {
                        comment: comment_idx,
                        problem: format!("invalid JSON: {e}"),
                    });
                    continue;
                }
            };

            // Fold case on verdict if needed
            if let Some(v) = rec.get("verdict").and_then(Value::as_str) {
                let lower = v.trim().to_lowercase();
                if lower == "needs_revision" || lower == "changes_requested" {
                    rec["verdict"] = json!(lower);
                }
            }

            // Validate required fields
            let required = [
                "repo",
                "pr",
                "head",
                "base_sha",
                "scope",
                "reviewer",
                "implementers",
                "verdict",
            ];
            let mut missing = Vec::new();
            for req in required {
                if rec.get(req).is_none() {
                    missing.push(req);
                }
            }
            if rec.get("scope").and_then(Value::as_str) == Some("delta")
                && rec.get("from").is_none()
            {
                missing.push("from");
            }
            if !missing.is_empty() {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: format!("missing fields: {missing:?}"),
                });
                continue;
            }

            let implementers_arr = rec.get("implementers").and_then(Value::as_array);
            let implementers: Vec<String> = implementers_arr
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            if implementers.is_empty() {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: "implementers must be a non-empty list".to_string(),
                });
                continue;
            }

            let scope = rec.get("scope").and_then(Value::as_str).unwrap_or("");
            if scope != "whole" && scope != "delta" {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: format!("unknown scope: {scope:?}"),
                });
                continue;
            }

            let verdict = rec.get("verdict").and_then(Value::as_str).unwrap_or("");
            if verdict != "accept" && verdict != "needs_revision" && verdict != "changes_requested"
            {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: format!("unknown verdict {verdict:?}: use accept, needs_revision or changes_requested (lower case); nothing was counted"),
                });
                continue;
            }

            // Verify signature if provided or required
            let reviewer = rec.get("reviewer").and_then(Value::as_str).unwrap_or("");
            let canon_seat = steward::coverage::canonical_reviewer_seat(reviewer);
            let is_verified = if let (Some(sig), Some(seat)) = (sig_text, canon_seat.as_deref()) {
                if let Some(bytes) = canonical_attestation_bytes(&rec) {
                    verify_signature(&bytes, sig, seat, allowed_signers_text)
                } else {
                    false
                }
            } else {
                false
            };

            if require_signatures && !is_verified {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: "unsigned or unverifiable attestation (signatures required)"
                        .to_string(),
                });
                continue;
            }

            // Check subject repository and PR match
            let rec_repo = rec.get("repo").and_then(Value::as_str).unwrap_or("");
            let rec_pr = rec.get("pr").and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            });
            if rec_repo != repo || rec_pr != Some(pr_number) {
                malformed.push(steward::rejected::Malformed {
                    comment: comment_idx,
                    problem: format!("the block names a different repository or PR than the one it was posted on ({rec_repo}#{rec_pr:?})"),
                });
                continue;
            }

            records.push(steward::coverage::Record {
                head: rec
                    .get("head")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                base_sha: rec
                    .get("base_sha")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                scope: scope.to_string(),
                reviewer: reviewer.to_string(),
                implementers,
                verdict: verdict.to_string(),
                from: rec.get("from").and_then(Value::as_str).map(String::from),
            });
        }

        // Check for mislabelled fences
        for caps in any_fence.captures_iter(&normalized) {
            let label = caps.get(1).map_or("", |m| m.as_str()).trim();
            if label == "attestation" {
                continue;
            }
            let raw_block = caps.get(2).map_or("", |m| m.as_str());
            if let Ok(val) = serde_json::from_str::<Value>(raw_block) {
                if let Some(head) = val.get("head").and_then(Value::as_str) {
                    if full_sha.is_match(head)
                        && val.get("repo").is_some()
                        && val.get("verdict").is_some()
                    {
                        malformed.push(steward::rejected::Malformed {
                            comment: comment_idx,
                            problem: format!("fence labelled {label:?} holds a complete attestation record for head {head}, but only a fence labelled attestation is read; nothing was counted"),
                        });
                    }
                }
            }
        }

        // Check for legacy prose verdict if no attestation block was found
        if !found_any && prose_verdict.is_match(&normalized) {
            legacy.push(comment_idx);
        }
    }

    Ok(ParsedCommentAttestations {
        records,
        malformed,
        legacy,
    })
}

/// Convert formal GitHub reviews into `steward::coverage::Record` entries.
pub fn reviews_to_records(
    reviews: &[Value],
    head: &str,
    merge_base: &str,
    implementers: &[String],
    login_to_seat: &BTreeMap<String, String>,
    base_events: &[String],
) -> (
    Vec<steward::coverage::Record>,
    Vec<steward::rejected::Malformed>,
) {
    let mut records = Vec::new();
    let mut problems = Vec::new();

    if merge_base.is_empty() || merge_base == "unknown" {
        problems.push(steward::rejected::Malformed {
            comment: None,
            problem: "merge-base unresolved (checkout lacks the PR head?); formal reviews cannot be bound".to_string(),
        });
        return (records, problems);
    }

    // GitHub lists reviews chronologically. Only each reviewer's latest decision stands.
    let mut latest: BTreeMap<String, &Value> = BTreeMap::new();
    for r in reviews {
        let state = r.get("state").and_then(Value::as_str).unwrap_or("");
        if state == "APPROVED" || state == "CHANGES_REQUESTED" || state == "DISMISSED" {
            if let Some(login) = r
                .get("user")
                .and_then(|u| u.get("login"))
                .and_then(Value::as_str)
            {
                latest.insert(login.to_lowercase(), r);
            }
        }
    }

    for review in latest.values() {
        let state = review.get("state").and_then(Value::as_str).unwrap_or("");
        let commit_id = review
            .get("commit_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        if (state != "APPROVED" && state != "CHANGES_REQUESTED") || commit_id != head {
            continue;
        }

        let submitted = review
            .get("submitted_at")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !submitted.is_empty() && base_events.iter().any(|ev| ev.as_str() > submitted) {
            problems.push(steward::rejected::Malformed {
                comment: None,
                problem: "base changed after this review was submitted".to_string(),
            });
            continue;
        }

        let login = review
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let seat = match login_to_seat.get(&login.to_lowercase()) {
            Some(s) => s.clone(),
            None => {
                problems.push(steward::rejected::Malformed {
                    comment: None,
                    problem: format!("login {login:?} is not a mapped seat"),
                });
                continue;
            }
        };

        if implementers.is_empty() {
            problems.push(steward::rejected::Malformed {
                comment: None,
                problem: "implementers unknown; independence unproven".to_string(),
            });
            continue;
        }

        records.push(steward::coverage::Record {
            head: head.to_string(),
            base_sha: merge_base.to_string(),
            scope: "whole".to_string(),
            reviewer: seat,
            implementers: implementers.to_vec(),
            verdict: if state == "APPROVED" {
                "accept".to_string()
            } else {
                "changes_requested".to_string()
            },
            from: None,
        });
    }

    (records, problems)
}

/// Determine whether a review thread has been unanswered by the implementing seats.
pub fn is_thread_unanswered(
    thread: &Value,
    author_seats: &[String],
    attribution: &steward::attribution::Attribution,
) -> bool {
    let comments = match thread.get("comments") {
        Some(c) => c,
        None => return true,
    };
    let total_count = comments
        .get("totalCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let nodes = match comments.get("nodes").and_then(Value::as_array) {
        Some(n) => n,
        None => return true,
    };
    if total_count <= 1 || nodes.is_empty() {
        return true;
    }
    let last = &nodes[nodes.len() - 1];
    let login = last
        .get("author")
        .and_then(|a| a.get("login"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if login.is_empty() || login.ends_with("[bot]") {
        return true;
    }
    let body = last.get("body").and_then(Value::as_str).unwrap_or("");
    let seat = attribution.comment_seat(body, true);
    seat.is_empty()
        || !author_seats
            .iter()
            .any(|s| s.to_lowercase() == seat.to_lowercase())
}

/// Extract standing blocker {verdict, by} from coverage reasons if present.
pub fn parse_standing_blocker(reasons: &[String]) -> Option<Value> {
    let re = Regex::new(r"^standing (\w+) at head by (.+)$").ok()?;
    for reason in reasons {
        if let Some(caps) = re.captures(reason) {
            return Some(json!({
                "verdict": &caps[1],
                "by": &caps[2],
            }));
        }
    }
    None
}

/// Main live collection entrypoint: builds the complete Guild state snapshot.
pub fn collect_live_state(
    repo: &str,
    repo_dir: &Path,
    identities_path: Option<&Path>,
    require_signatures: bool,
) -> Result<Value> {
    // 1. Verify repository checkout origin matches
    let remote_url = run_git(repo_dir, &["remote", "get-url", "origin"])?;
    if !checkout_matches(&remote_url, repo) {
        bail!(
            "repo-dir {:?} is not a clone of {repo} (origin: {:?})",
            repo_dir,
            remote_url.trim()
        );
    }

    // 2. Load identities and registered seat keys
    let identities_val = if let Some(path) = identities_path {
        if !path.exists() {
            bail!("identities file does not exist: {}", path.display());
        }
        let content = fs::read_to_string(path)?;
        let val: Value = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse identities JSON at {}", path.display()))?;
        validate_identities(&val)?;
        Some(val)
    } else {
        let default_path = repo_dir.join("governance").join("identities.json");
        if default_path.exists() {
            let content = fs::read_to_string(&default_path)?;
            let val: Value = serde_json::from_str(&content).with_context(|| {
                format!(
                    "failed to parse identities JSON at {}",
                    default_path.display()
                )
            })?;
            validate_identities(&val)?;
            Some(val)
        } else {
            None
        }
    };

    let allowed_signers = identities_val
        .as_ref()
        .map(build_allowed_signers)
        .unwrap_or_default();

    let mut login_to_seat: BTreeMap<String, String> = BTreeMap::new();
    let mut configured_agents: Vec<String> = Vec::new();
    if let Some(ref id) = identities_val {
        if let Some(seats) = id.get("seats").and_then(Value::as_object) {
            for (seat, entry) in seats {
                configured_agents.push(seat.clone());
                if let Some(logins) = entry.get("logins").and_then(Value::as_array) {
                    for login in logins {
                        if let Some(l) = login.as_str() {
                            login_to_seat.insert(l.to_lowercase(), seat.clone());
                        }
                    }
                }
            }
        }
    }
    if configured_agents.is_empty() {
        configured_agents = vec![
            "claude".to_string(),
            "mimo".to_string(),
            "codex".to_string(),
            "gemini".to_string(),
            "ubuntu-worker-01".to_string(),
        ];
    }

    // 3. Query open PRs with complete metadata
    let pr_list_raw = run_gh_json(&[
        "pr",
        "list",
        "-R",
        repo,
        "--state",
        "open",
        "--limit",
        "1000",
        "--json",
        "number,title,isDraft,mergeStateStatus,updatedAt,headRefName,headRefOid,baseRefName,additions,deletions,changedFiles",
    ])?;
    let pr_nodes = pr_list_raw.as_array().cloned().unwrap_or_default();

    // 4. Query default branch
    let default_branch = run_gh_json(&["repo", "view", repo, "--json", "defaultBranchRef"])
        .ok()
        .and_then(|v| {
            v.get("defaultBranchRef")
                .and_then(|d| d.get("name"))
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| "main".to_string());

    // 5. Query GraphQL review threads
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        bail!("invalid repo slug {repo:?}; expected owner/name");
    }
    let (owner, name) = (parts[0], parts[1]);
    let graphql_resp = run_gh_json(&[
        "api",
        "graphql",
        "-f",
        &format!("query={THREADS_QUERY}"),
        "-F",
        &format!("owner={owner}"),
        "-F",
        &format!("name={name}"),
    ])?;

    let pull_requests = graphql_resp
        .get("data")
        .and_then(|d| d.get("repository"))
        .and_then(|r| r.get("pullRequests"));
    if pull_requests
        .and_then(|p| p.get("pageInfo"))
        .and_then(|pi| pi.get("hasNextPage"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        bail!("more than 100 open pull requests: unresolved-thread counts would be incomplete");
    }

    let mut unresolved_by_pr: BTreeMap<i64, usize> = BTreeMap::new();
    let mut threads_has_next_by_pr: BTreeMap<i64, bool> = BTreeMap::new();
    let mut raw_threads_by_pr: BTreeMap<i64, Vec<Value>> = BTreeMap::new();
    if let Some(nodes) = pull_requests
        .and_then(|p| p.get("nodes"))
        .and_then(Value::as_array)
    {
        for n in nodes {
            let num = n.get("number").and_then(Value::as_i64).unwrap_or(0);
            let threads = n.get("reviewThreads");
            let mut count = 0;
            let mut threads_list = Vec::new();
            if let Some(tnodes) = threads
                .and_then(|t| t.get("nodes"))
                .and_then(Value::as_array)
            {
                for t in tnodes {
                    threads_list.push(t.clone());
                    if !t
                        .get("isResolved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        count += 1;
                    }
                }
            }
            let has_next = threads
                .and_then(|t| t.get("pageInfo"))
                .and_then(|pi| pi.get("hasNextPage"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if has_next {
                count += 1;
            }
            unresolved_by_pr.insert(num, count);
            threads_has_next_by_pr.insert(num, has_next);
            raw_threads_by_pr.insert(num, threads_list);
        }
    }

    // 6. Inspect each PR: fetch comments, reviews, base events, commit log & trailers
    let attribution = steward::attribution::Attribution::new()
        .map_err(|e| anyhow::anyhow!("steward attribution pattern error: {e}"))?;
    let rejected_rules = steward::rejected::Rules::new()
        .map_err(|e| anyhow::anyhow!("steward rejected rules pattern error: {e}"))?;
    let receipts_rules = steward::receipts::Receipts::new()
        .map_err(|e| anyhow::anyhow!("steward receipts rules pattern error: {e}"))?;

    let is_ancestor = |a: &str, b: &str| -> bool {
        if a == b {
            return true;
        }
        Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .args(["merge-base", "--is-ancestor", a, b])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    let commit_exists = |token: &str| -> Option<bool> {
        let out = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .args([
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{token}^{{commit}}"),
            ])
            .output()
            .ok()?;
        match out.status.code() {
            Some(0) => Some(true),
            Some(1) => Some(false),
            _ => None,
        }
    };

    let mut pr_rows = Vec::new();
    let mut steward_prs = Vec::new();
    let mut implementers_by_pr: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let mut repair_owners: BTreeMap<i64, Vec<String>> = BTreeMap::new();

    for item in &pr_nodes {
        let number = item.get("number").and_then(Value::as_i64).unwrap_or(0);
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let head = item
            .get("headRefOid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let branch = item
            .get("headRefName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let base_ref = item
            .get("baseRefName")
            .and_then(Value::as_str)
            .unwrap_or("main")
            .to_string();
        let is_draft = item
            .get("isDraft")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let merge_state_status = item
            .get("mergeStateStatus")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let updated_at = item
            .get("updatedAt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let additions = item.get("additions").and_then(Value::as_i64).unwrap_or(0);
        let deletions = item.get("deletions").and_then(Value::as_i64).unwrap_or(0);
        let changed_files = item
            .get("changedFiles")
            .and_then(Value::as_i64)
            .unwrap_or(0);

        // Fetch comments
        let comments_raw = run_gh_json(&[
            "api",
            &format!("repos/{repo}/issues/{number}/comments"),
            "--paginate",
            "--slurp",
        ])?;
        let comment_nodes = flatten_pages(&comments_raw);
        let steward_comments: Vec<steward::rejected::Comment> = comment_nodes
            .iter()
            .map(|c| steward::rejected::Comment {
                id: c.get("id").and_then(Value::as_i64),
                body: c
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();

        // Fetch formal reviews
        let reviews_raw = run_gh_json(&[
            "api",
            &format!("repos/{repo}/pulls/{number}/reviews"),
            "--paginate",
            "--slurp",
        ])?;
        let review_nodes = flatten_pages(&reviews_raw);

        // Fetch timeline base events
        let timeline_raw = run_gh_json(&[
            "api",
            &format!("repos/{repo}/issues/{number}/timeline"),
            "--paginate",
            "--slurp",
        ])?;
        let timeline_nodes = flatten_pages(&timeline_raw);
        let mut base_events: Vec<String> = timeline_nodes
            .iter()
            .filter(|e| {
                let ev = e.get("event").and_then(Value::as_str).unwrap_or("");
                ev == "base_ref_changed" || ev == "base_ref_force_pushed"
            })
            .filter_map(|e| {
                e.get("created_at")
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .collect();
        base_events.sort();

        // Git merge base
        let base_remote_ref = format!("origin/{base_ref}");
        let mb = run_git(repo_dir, &["merge-base", &base_remote_ref, &head])
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Git log & trailers
        let log_text = run_git(
            repo_dir,
            &[
                "log",
                "--format=%x1egit-author: %an <%ae>%n%B",
                &format!("{base_remote_ref}..{head}"),
            ],
        )
        .unwrap_or_default();

        let implementers = seats_from_trailers(&log_text, &branch);
        let primary_seat = implementers.first().cloned().unwrap_or_default();
        implementers_by_pr.insert(number, implementers.clone());

        // Parse attestations from comments
        let parsed = parse_comment_attestations(
            &steward_comments,
            repo,
            number,
            &allowed_signers,
            require_signatures,
        )?;

        // Formal reviews to records
        let (formal_records, formal_problems) = reviews_to_records(
            &review_nodes,
            &head,
            &mb,
            &implementers,
            &login_to_seat,
            &base_events,
        );

        let mut all_malformed = parsed.malformed;
        all_malformed.extend(formal_problems);

        // Augment records with implied git implementers if needed
        let implied_set: BTreeSet<String> = implementers.iter().cloned().collect();
        let mut combined_records = Vec::new();
        let mut conflicts = Vec::new();

        for mut record in parsed.records.into_iter().chain(formal_records) {
            let named_set: BTreeSet<String> = record
                .implementers
                .iter()
                .flat_map(|i| steward::seats::seat_names(i))
                .collect();
            let extra: Vec<String> = implied_set.difference(&named_set).cloned().collect();
            let reviewer_seats = steward::seats::seat_names(&record.reviewer);
            if !extra.is_empty()
                && record.verdict == "accept"
                && record.head == head
                && !reviewer_seats.is_disjoint(&implied_set)
            {
                let intersect: Vec<String> =
                    reviewer_seats.intersection(&implied_set).cloned().collect();
                conflicts.push(format!(
                    "the accept by {} is not counted: git history names {:?} as an implementer (branch prefix or commit trailer) and the attestation's implementers list does not",
                    record.reviewer, intersect
                ));
            }
            if !extra.is_empty() {
                record.implementers.extend(extra);
            }
            combined_records.push(record);
        }

        // Evaluate review coverage
        let mut decision =
            steward::coverage::evaluate(&head, Some(&mb), &combined_records, &is_ancestor);
        decision.reasons.extend(conflicts);

        // Evaluate rejected verdicts
        let rejected = rejected_rules.rejected_verdicts(
            &steward_comments,
            &head,
            &all_malformed,
            &parsed.legacy,
            Some(&commit_exists),
        );
        let rejected_seats: Vec<String> = rejected
            .iter()
            .map(|r| r.seat.clone())
            .filter(|s| !s.is_empty())
            .collect();
        repair_owners.insert(number, rejected_seats);

        let rejected_verdicts_json: Vec<Value> = rejected
            .into_iter()
            .map(|r| {
                json!({
                    "comment_id": r.comment_id,
                    "seat": r.seat,
                    "reason": r.reason,
                })
            })
            .collect();

        // Calculate unanswered threads
        let unresolved_count = unresolved_by_pr.get(&number).copied().unwrap_or(0);
        let unanswered_count = if let Some(threads) = raw_threads_by_pr.get(&number) {
            let mut count = threads
                .iter()
                .filter(|t| {
                    !t.get("isResolved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        && is_thread_unanswered(t, &implementers, &attribution)
                })
                .count();
            if threads_has_next_by_pr
                .get(&number)
                .copied()
                .unwrap_or(false)
            {
                count += 1;
            }
            count
        } else {
            unresolved_count
        };

        let standing_blocker = parse_standing_blocker(&decision.reasons);

        steward_prs.push(steward::receipts::Pr {
            number,
            head: head.clone(),
            comments: steward_comments,
            implementers: implementers.clone(),
        });

        pr_rows.push(json!({
            "number": number,
            "title": title,
            "head": head,
            "base": base_ref,
            "draft": is_draft,
            "mergeable_state": merge_state_status,
            "updated_at": updated_at,
            "seat": primary_seat,
            "base_sha": if mb == "unknown" { "".to_string() } else { mb },
            "rejected_verdicts": rejected_verdicts_json,
            "size": format!("+{additions}/-{deletions} in {changed_files} file(s)"),
            "implementers": implementers,
            "threads_unresolved": unresolved_count,
            "threads_unanswered": unanswered_count,
            "covered": decision.covered,
            "standing_blocker": standing_blocker,
        }));
    }

    // 7. Compute done receipts
    let done_by_map =
        receipts_rules.done_receipts(&steward_prs, &repair_owners, &implementers_by_pr);
    let done_keys: Vec<String> = done_by_map.keys().cloned().collect();

    // 8. Assemble final canonical state
    let now_utc = format_iso8601(unix_secs_now());
    let state = json!({
        "now": now_utc,
        "agents": configured_agents.into_iter().map(|a| json!({"name": a})).collect::<Vec<_>>(),
        "prs": pr_rows,
        "backlog": [],
        "done": done_keys,
        "done_by": done_by_map,
        "default_branch": default_branch,
    });

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_matches() {
        assert!(checkout_matches(
            "git@github.com:Aarogaming/aaroneous-devtools.git",
            "Aarogaming/aaroneous-devtools"
        ));
        assert!(checkout_matches(
            "https://github.com/Aarogaming/aaroneous-devtools",
            "Aarogaming/aaroneous-devtools"
        ));
        assert!(checkout_matches(
            "https://github.com/Aarogaming/aaroneous-devtools.git/",
            "Aarogaming/aaroneous-devtools"
        ));
        assert!(!checkout_matches(
            "https://github.com/Other/aaroneous-devtools",
            "Aarogaming/aaroneous-devtools"
        ));
    }

    #[test]
    fn test_seats_from_trailers() {
        let text = "Implement core feature\n\nCo-Authored-By: Claude <claude@anthropic.com>\nContribution-Id: gemini\n";
        let seats = seats_from_trailers(text, "mimo/feature-branch");
        assert_eq!(seats, vec!["mimo", "claude", "gemini"]);
    }

    #[test]
    fn test_standing_blocker_parsing() {
        let reasons = vec![
            "standing needs_revision at head by claude".to_string(),
            "another reason".to_string(),
        ];
        let blocker = parse_standing_blocker(&reasons).expect("found blocker");
        assert_eq!(blocker["verdict"], "needs_revision");
        assert_eq!(blocker["by"], "claude");
    }

    #[test]
    fn test_validate_identities_valid() {
        let valid = json!({
            "owner": ["Aarogaming"],
            "seats": {
                "claude": {"logins": ["claude-bot"], "keys": ["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKl7YYxzb4umZVCMjjM/fNtSLhfsP9KV55cj9jQogc0n"]},
                "mimo": {"logins": [], "keys": []}
            }
        });
        assert!(validate_identities(&valid).is_ok());
    }

    #[test]
    fn test_validate_identities_rejects_owner_as_seat_login() {
        let invalid = json!({
            "owner": ["Aarogaming"],
            "seats": {
                "claude": {"logins": ["aarogaming"], "keys": []}
            }
        });
        let err = validate_identities(&invalid).unwrap_err();
        assert!(err.to_string().contains("shared owner login"));
    }

    #[test]
    fn test_validate_identities_rejects_duplicate_login() {
        let invalid = json!({
            "owner": ["Aarogaming"],
            "seats": {
                "claude": {"logins": ["shared-bot"], "keys": []},
                "mimo": {"logins": ["shared-bot"], "keys": []}
            }
        });
        let err = validate_identities(&invalid).unwrap_err();
        assert!(err.to_string().contains("is listed for both"));
    }

    #[test]
    fn test_validate_identities_rejects_invalid_key_format() {
        let invalid = json!({
            "owner": ["Aarogaming"],
            "seats": {
                "claude": {"logins": [], "keys": ["cert-authority ssh-ed25519 not-valid-base64!!!"]}
            }
        });
        let err = validate_identities(&invalid).unwrap_err();
        assert!(
            err.to_string()
                .contains("is not '<known key type> <base64>'")
        );
    }

    #[test]
    fn test_validate_identities_rejects_invalid_seat_name() {
        let invalid = json!({
            "owner": ["Aarogaming"],
            "seats": {
                "Claude\nEvil": {"logins": [], "keys": []}
            }
        });
        let err = validate_identities(&invalid).unwrap_err();
        assert!(err.to_string().contains("seat name"));
    }
}
