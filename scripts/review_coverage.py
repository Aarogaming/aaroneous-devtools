#!/usr/bin/env python3
"""Shadow (report-only) review-coverage check, lane C97, per governance/PLATFORM_SPEC.md section 3.

A PR is review-covered at its current head when accepted scopes, chained from a whole-diff accept to
that head, cover base...head and no changes-requested stands at that head.

Inputs are attestation records, never prose:

* Structured (authoritative): a fenced block in a PR comment, tagged ``attestation``, holding one JSON
  object with the fields in the spec (``repo``, ``pr``, ``head``, ``base_sha``, ``scope``, ``from``,
  ``reviewer``, ``implementers`` (non-empty list), ``verdict``). A record counts only for the repository and PR it
  names: ``repo`` and ``pr`` must equal the subject, otherwise it is reported as malformed.
* Legacy (heuristic, reported separately, never counted as covered): existing REVIEW PACKET prose.

* Formal GitHub reviews (authenticated, lane C100): an APPROVED or CHANGES_REQUESTED review whose
  commit equals the PR head, by a login mapped to a seat in an identity map, counts like a whole-diff
  record. The map never lists the shared owner login as a seat, so shared-account reviews never count.

Usage:
  review_coverage.py --snapshot FILE.json                       offline, deterministic (the tests)
  review_coverage.py --repo OWNER/NAME --repo-dir CHECKOUT      live read via the gh CLI (read only);
                                                               CHECKOUT must be a clone of OWNER/NAME

Exit status is 0 for a produced report: this check never blocks anything.
"""

import argparse
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import attest_sig  # noqa: E402

FENCE = re.compile(r"```attestation\s*\n(.*?)\n```", re.DOTALL)
FULL_SHA = re.compile(r"\b[0-9a-f]{40}\b")
# Any fenced block, with its info string. Only a fence labelled ``attestation`` is read; a complete attestation record in
# a differently labelled fence (```json is what seats reach for) is reported by name instead of vanishing.
ANY_FENCE = re.compile(r"```([^\n`]*)\n(.*?)\n```", re.DOTALL)
# A prose verdict: the upper-case VERDICT label the old packets used, or the word "verdict" in any case followed within
# a few words by an accept or reject word ("Independent Codex verdict: ACCEPT ..."). Without the second form a seat
# that writes the label in lower case is invisible: not counted, not malformed, not legacy, so nobody tells it why.
PROSE_VERDICT = re.compile(
    r"\bVERDICT\b|(?i:\bverdict\b[^.\n]{0,40}?\b(?:accept(?:ed)?|reject(?:ed)?|approve[ds]?|changes[ _-]?requested|needs[ _-]?revision)\b)")
ACCEPT = {"accept"}
REJECT = {"changes_requested", "needs_revision"}
REQUIRED = ("repo", "pr", "head", "base_sha", "scope", "reviewer", "implementers", "verdict")


def validate(record):
    """Return a problem string, or None when the record is a usable attestation."""
    if not isinstance(record, dict):
        return f"not a JSON object: {type(record).__name__}"
    missing = [key for key in REQUIRED if key not in record]
    if record.get("scope") == "delta" and "from" not in record:
        missing.append("from")
    if missing:
        return f"missing fields: {missing}"
    implementers = record["implementers"]
    if not isinstance(implementers, list) or not implementers:
        return "implementers must be a non-empty list"
    if record["scope"] not in ("whole", "delta"):
        return f"unknown scope: {record['scope']!r}"
    if not isinstance(record["verdict"], str) or record["verdict"] not in ACCEPT | REJECT:
        return (f"unknown verdict {record['verdict']!r}: use accept, needs_revision or changes_requested "
                "(lower case); nothing was counted")
    return None


def looks_like_attestation(raw):
    """The parsed record when ``raw`` is a JSON object carrying every required field and a full 40-hex head, else None.
    A template with placeholders or a partial example is not one, so quoting the format does not raise a problem."""
    try:
        record = json.loads(raw)
    except ValueError:
        return None
    if not isinstance(record, dict) or any(key not in record for key in REQUIRED):
        return None
    head = record["head"]
    return record if isinstance(head, str) and FULL_SHA.fullmatch(head) else None


def fold_rejection(record):
    """A rejecting verdict spelled in the wrong case still blocks (``NEEDS_REVISION`` is read as ``needs_revision``).
    An accept does not get the same leniency: it must be spelled exactly, so a misspelt accept never covers a PR, but a
    misspelt rejection must never be silently ignored while the PR reads as covered."""
    if isinstance(record, dict) and isinstance(record.get("verdict"), str):
        folded = record["verdict"].strip().lower()
        if folded in REJECT:
            record["verdict"] = folded


def parse_attestations(comments):
    """Return (structured, problems, legacy). Malformed records are reported, never counted."""
    structured, problems, legacy = [], [], []
    for index, comment in enumerate(comments):
        body = comment.get("body", "") if isinstance(comment, dict) else ""
        found = False
        for raw, signature in attest_sig.pairs(body):
            found = True
            try:
                record = json.loads(raw)
            except ValueError as error:
                problems.append({"comment": index, "problem": f"invalid JSON: {error}"})
                continue
            fold_rejection(record)
            problem = validate(record)
            if problem:
                problems.append({"comment": index, "problem": problem})
                continue
            record["_canon"] = attest_sig.canonical(record)
            record["_sig"] = signature
            record["comment"] = index
            structured.append(record)
        for label, raw in ANY_FENCE.findall(body):
            if label.rstrip() == "attestation":  # exactly what FENCE reads; any other spelling is reported below
                continue
            record = looks_like_attestation(raw)
            if record is not None:
                found = True
                problems.append({"comment": index, "problem": (
                    f"fence labelled {label.strip() or 'plain'!r} holds a complete attestation record for head "
                    f"{record['head']}, but only a fence labelled attestation is read; nothing was counted")})
        if not found and PROSE_VERDICT.search(body):
            legacy.append(
                {
                    "comment": index,
                    "heads_mentioned": sorted(set(FULL_SHA.findall(body))),
                    "looks_whole": bool(re.search(r"whole[- ]diff|entire diff", body, re.I)),
                }
            )
    return structured, problems, legacy


# One seat is one seat however its name is spelled: "Gemini", "gemini" and "Antigravity" are the same reviewer. The merge gate's
# "at least two independent seats" box counts len(accept_seats), so two comments from one seat spelled differently (live on
# #96: ['Gemini', 'gemini']) looked like two seats.
SEAT_NAME_ALIASES = {"antigravity": "gemini", "qwen": "mimo"}


# A seat is recognised by a word that STARTS with its name: "claude-sonnet-5-5", "gemini-3.8-flash", "mimo-v2.6-flash-free",
# "Claude\u200b" and "CLAUDE." all name a seat. Taking only the first space-separated word missed every model-id and
# punctuation form (skeptic probe on #99: reviewer "Claude" with implementers ["claude-sonnet-5-5"] was independent).
KNOWN_SEATS = ("claude", "codex", "gemini", "antigravity", "mimo", "qwen")

# A model family named without its seat word still names the seat that runs it: "Opus 5.5" is Claude, "gpt-5.5" is Codex.
# Without this the string kept its own first token as an "unknown seat", so a Claude session labelling itself "Opus"
# was independent of an implementer "claude" and two labels ("Opus", "Sonnet") counted as two reviewing seats
# (skeptic probe on #99 main: both covered a PR whose only implementer was claude).
MODEL_FAMILY_SEATS = {"opus": "claude", "sonnet": "claude", "haiku": "claude", "fable": "claude", "anthropic": "claude",
                      "gpt": "codex", "openai": "codex"}


def seat_names(name):
    """The set of seats a reviewer or implementer string names. A string may name several ("Claude, Mimo",
    "claude+mimo", "Gemini/Antigravity"); a string that names none keeps its first alphanumeric token, so an unknown
    seat still compares with itself."""
    tokens = [t for t in re.split(r"[^a-z0-9]+", str(name).strip().lower()) if t]
    found = set()
    for token in tokens:
        for seat in KNOWN_SEATS:
            if token.startswith(seat):
                found.add(SEAT_NAME_ALIASES.get(seat, seat))
        for family, seat in MODEL_FAMILY_SEATS.items():
            if token.startswith(family):
                found.add(seat)
    if not found and tokens:
        found.add(tokens[0])
    return found


def known_seat_names(name):
    """The KNOWN seats a string names, with no fallback: `seat_names` keeps an unknown string's first token so it can
    compare with itself; here a string that names no seat names none."""
    found = set()
    for token in (t for t in re.split(r"[^a-z0-9]+", str(name).strip().lower()) if t):
        for seat in KNOWN_SEATS:
            if token.startswith(seat):
                found.add(SEAT_NAME_ALIASES.get(seat, seat))
    return found


TRAILER = re.compile(r"^(?:co-authored-by|contribution-id|git-author):(.*)$", re.I | re.M)


def commit_chunks(commit_log):
    """The commits of a `git log --format=%x1egit-author: %an <%ae>%n%B` listing, one string per commit."""
    return [chunk for chunk in str(commit_log or "").split("\x1e") if chunk.strip()]


def git_implementers(commit_log, branch):
    """The seats git itself says implemented a PR: the seat named by the branch prefix ("codex/..."), and every seat
    named in a Co-Authored-By or Contribution-ID trailer or by the commit author's name, for any commit in base..head. The attestation's own
    `implementers` list is the reviewer's word; this is the history's, and an accept from a seat in it is not
    independent however the attestation spells the list (Gemini committed to #116 and attested it as independent)."""
    seats = known_seat_names(str(branch or "").split("/", 1)[0])
    for chunk in commit_chunks(commit_log):
        for trailer in TRAILER.findall(chunk):
            seats |= known_seat_names(trailer)
    return seats


def unattributed_commits(commit_log, branch):
    """How many commits in base..head name no implementing seat: no seat trailer, and the branch prefix names none
    either. With such a commit, git cannot say who wrote part of the diff, so independence rests on the
    attestation's word alone."""
    if known_seat_names(str(branch or "").split("/", 1)[0]):
        return 0
    return sum(1 for chunk in commit_chunks(commit_log)
               if not any(known_seat_names(t) for t in TRAILER.findall(chunk)))


STAND_IN = re.compile(r"\b(?:in lieu of|on behalf of|acting for|operating for|standing in for)\s+(Claude|Codex|Gemini|Mimo|Antigravity)\b",
                      re.I)


def stand_in_seats(comments):
    """The seats whose behalf a comment claims to act on ("Author Update (operating in lieu of Codex)"): the shared login
    posts for every seat, so such a comment is written by a seat it does not name, and the seat it does name is not the
    one that worked. Only the first non-blank line is read, and "in lieu of X" never attributes the comment to X.
    Scout sample: three comments on #20, #47 and #60 said this about Codex; text attribution rightly leaves them
    unattributed, but nothing told a reader that someone other than the branch's seat was pushing."""
    found = []
    for comment in comments or []:
        body = comment.get("body", "") if isinstance(comment, dict) else ""
        first = next((line for line in body.splitlines() if line.strip()), "")
        found.extend(SEAT_NAME_ALIASES.get(m.lower(), m.lower()) for m in STAND_IN.findall(first))
    return sorted(set(found))


def seat_name(name):
    """One seat for display and de-duplication: the first of the sorted seats `seat_names` finds."""
    seats = sorted(seat_names(name))
    return seats[0] if seats else ""


def canonical_reviewer_seat(name):
    """The single canonical seat an attestation reviewer resolves to.
    Fails closed (returns None) if the reviewer is ambiguous (names multiple distinct seats) or empty
    (Codex P2 on #111: an ambiguous reviewer like 'Opus/gpt-5.5' cannot contribute multiple seats to accept_seats)."""
    seats = seat_names(name)
    return next(iter(seats)) if len(seats) == 1 else None


def _hyphenated(text):
    """Lower-case alphanumeric runs joined by hyphens: 'Ubuntu Worker 01', 'ubuntu_worker_01' and 'ubuntu-worker-01' agree."""
    return "-".join(t for t in re.split(r"[^a-z0-9]+", str(text).strip().lower()) if t)


def named_seats(name):
    """The known seats a string names (seat word, alias or model family), with no fallback: `seat_names` plus nothing."""
    found = set()
    for token in (t for t in re.split(r"[^a-z0-9]+", str(name).strip().lower()) if t):
        for seat in KNOWN_SEATS:
            if token.startswith(seat):
                found.add(SEAT_NAME_ALIASES.get(seat, seat))
        for family, seat in MODEL_FAMILY_SEATS.items():
            if token.startswith(family):
                found.add(seat)
    return found


def resolve_seat(label, registered=None):
    """The seat a reviewer label names, or None when it names none or more than one. A registered seat is matched by its
    whole hyphen-delimited name; every other seat the label names (a second registered seat, or a known seat or model family
    such as `codex` or `Opus`) makes it ambiguous, so `ubuntu-worker-01 and codex` is not read as `ubuntu-worker-01`."""
    flat = "-" + _hyphenated(label) + "-"
    hits = sorted(seat for seat in (registered or ()) if _hyphenated(seat) and "-" + _hyphenated(seat) + "-" in flat)
    if not hits:
        return canonical_reviewer_seat(label)
    if len(hits) > 1:
        return None
    rest = flat.replace("-" + _hyphenated(hits[0]) + "-", "-", 1)
    others = named_seats(rest) - {SEAT_NAME_ALIASES.get(hits[0], hits[0])}
    return None if others else hits[0]


FAMILY_LABEL = re.compile(r"[a-z][a-z0-9-]{0,31}")
STEM = re.compile(r"[a-z][a-z0-9.-]{0,31}")
MODEL_MAX = 80


def clean_model(value):
    """The model id a record declares, or None. Declaring is optional and never refused: a missing, non-string, over-long or
    non-printable value is simply not used (the family falls back to the seat's registered profile). Printable means ASCII space to
    '~': model ids are ASCII, and the Rust port then agrees without a Unicode table."""
    if not isinstance(value, str):
        return None
    value = value.strip()
    if not value or len(value) > MODEL_MAX or any(not " " <= c <= "~" for c in value):
        return None
    return value


def seat_profiles(identities_data):
    """{seat: {model, family, runtime, ...}} from the optional `profile` each registered seat may carry: what the seat last
    reported about itself at registration. Gathered, never required, never a reason to refuse anyone."""
    seats = identities_data.get("seats") if isinstance(identities_data, dict) else None
    out = {}
    if isinstance(seats, dict):
        for seat, entry in seats.items():
            profile = entry.get("profile") if isinstance(entry, dict) else None
            if isinstance(profile, dict):
                clean = {k: v.strip() for k, v in profile.items() if isinstance(v, str) and v.strip()}
                if "family" in clean and not FAMILY_LABEL.fullmatch(clean["family"]):
                    del clean["family"]
                out[seat] = clean
    return out


def model_table(identities_data):
    """[(stem, family)] longest stem first, from the registry's `model_families` table."""
    table = identities_data.get("model_families") if isinstance(identities_data, dict) else None
    pairs = []
    if isinstance(table, dict):
        for key, family in table.items():
            if isinstance(key, str) and STEM.fullmatch(key) and isinstance(family, str) and FAMILY_LABEL.fullmatch(family):
                pairs.append((key, family))
    return sorted(pairs, key=lambda kv: (-len(kv[0]), kv[0]))


def model_family(model, table):
    """The family a model id belongs to per `table`, or None for a model the table does not know. The id's last path segment
    is matched against each stem: equal, or followed by a non-letter ('qwen3.8', 'claude-sonnet-5-5', 'Qwen/Qwen3.8-Coder',
    'gpt-5.5'), never by a letter ('qwenish' is not qwen)."""
    model = clean_model(model)
    if model is None:
        return None
    name = model.lower().rsplit("/", 1)[-1]
    for key, family in table:
        if name.startswith(key) and (len(name) == len(key) or not name[len(key)].isalpha()):
            return family
    return None


def independence(counted, registered, profiles, table):
    """What the accepting verdicts say about who judged: (families, entries, notes). One entry per accepting seat and declared
    model: the family comes from the model the signed record declares, else the seat's registered profile, else the seat
    stands alone. The notes are the self-report: an unclassified model, a model that differs from the registered profile,
    and accepts that share a family. Nothing here refuses anyone; the gate only counts distinct families."""
    entries, notes, seen = [], [], set()
    for record in counted:
        seat = resolve_seat(record["reviewer"], registered)
        if not seat:
            continue
        model = clean_model(record.get("model"))
        registered_family = (profiles.get(seat) or {}).get("family")
        declared = model_family(model, table) if model else None
        if declared:
            family, source = declared, "declared"
        elif registered_family:
            family, source = registered_family, "registered"
        else:
            family, source = f"seat:{seat}", "seat"
        key = (seat, model, family)
        if key in seen:
            continue
        seen.add(key)
        if model and not declared:
            notes.append(f"{seat} declared model {model!r}, which model_families does not classify; counted as {family} ({source})")
        elif declared and registered_family and declared != registered_family:
            notes.append(f"{seat} is registered as {registered_family} but ran {model!r} ({declared}): the declared model is used")
        entries.append({"seat": seat, "model": model, "family": family, "source": source})
    families = sorted({e["family"] for e in entries})
    for family in families:
        sharing = sorted({e["seat"] for e in entries if e["family"] == family})
        if len(sharing) > 1:
            notes.append(f"{', '.join(sharing)} are the same model family ({family}): counted as one judge")
    return families, entries, notes


def independent(record):
    """The reviewer is not among the implementers, comparing seats (see `seat_name`): reviewer "gemini" with implementers
    ["Gemini"], or "Gemini/Antigravity" with ["gemini"], is the same seat and never independent (Codex P1 on #98: the
    raw comparison let an implementer's own verdict count under a spelling variant)."""
    implementers = set().union(*(seat_names(i) for i in record["implementers"])) if record["implementers"] else set()
    return not (seat_names(record["reviewer"]) & implementers)


def evaluate(head, merge_base, records, is_ancestor, registered=None):
    """Pure coverage decision for one PR head.

    ``is_ancestor(a, b)`` says whether commit a is an ancestor of commit b. A delta accept extends
    coverage only when its ``from`` is a covered head AND an ancestor of the delta's own head, so a
    rebased or flattened head cannot inherit coverage by naming an old SHA.
    """
    if merge_base in (None, "", "unknown"):
        return {"covered": False, "reasons": ["merge-base unresolved: coverage cannot be judged"], "accepts_at_head": 0,
                "counted": [], "accept_seats": []}
    at_head = [r for r in records if r["head"] == head]
    # A record is valid only while the merge base equals its base_sha (PLATFORM_SPEC.md, attestation record): a retarget
    # or rebase invalidates a rejection like any other record (Codex P1 on #114). A rejection that names another base is
    # therefore not a blocker, but it is no longer silent: a hand-typed base_sha is the likelier cause, and ignoring
    # such a rejection unannounced is how a reviewer's request for changes gets lost.
    blockers = [r for r in at_head if r["verdict"] in REJECT and r["base_sha"] == merge_base]
    reasons = []
    if blockers:
        reasons.append(f"standing {blockers[0]['verdict']} at head by {blockers[0]['reviewer']}")
    for r in at_head:
        if r["verdict"] in REJECT and r["base_sha"] != merge_base:
            reasons.append(f"a rejection by {r['reviewer']} names base_sha {str(r['base_sha'])[:12]}, but the merge base is "
                           f"{merge_base[:12]}, so it is ignored as stale; if the base_sha was mistyped, that reviewer must "
                           "re-attest against the current merge base")

    valid = [
        r
        for r in records
        if r["verdict"] in ACCEPT and independent(r) and r["base_sha"] == merge_base
        and resolve_seat(r["reviewer"], registered) is not None
    ]
    for r in records:
        if r["verdict"] in ACCEPT and independent(r) and r["base_sha"] == merge_base and resolve_seat(r["reviewer"], registered) is None:
            reasons.append(f"reviewer {r['reviewer']!r} does not name exactly one seat, so its accept counts for no one; "
                           "name your seat alone and put the model in the record's `model` field")
    # Only a `whole` accept seeds coverage (governance/PLATFORM_SPEC.md section 3: "Delta accepts compose only when
    # chained from a whole-diff accept"). A delta that starts at the merge base reviewed the whole PR diff, but its
    # author must say so by labelling it `whole`; the tool tells them (see the reason below) and does not guess.
    covered = {r["head"] for r in valid if r["scope"] == "whole"}
    changed = True
    while changed:
        changed = False
        for r in valid:
            if (
                r["scope"] == "delta"
                and r["from"] in covered
                and r["head"] not in covered
                and is_ancestor(r["from"], r["head"])
            ):
                covered.add(r["head"])
                changed = True
    ok = head in covered and not blockers
    if head not in covered:
        if any(r["verdict"] in ACCEPT and r["head"] == head for r in records):
            reason = "accept at head exists but is not chained to a whole accept at this base"
            if any(r["scope"] == "delta" and r["head"] == head and r["from"] == merge_base for r in valid):
                reason += (f"; its delta starts at the merge base {merge_base[:8]}, which is the whole PR diff, so if the "
                           "reviewer read all of base...head they should re-post it with scope \"whole\"")
            reasons.append(reason)
            for r in at_head:
                if r["verdict"] not in ACCEPT or r in valid:
                    continue
                if r["base_sha"] != merge_base:
                    reasons.append(f"the accept by {r['reviewer']} names base_sha {str(r['base_sha'])[:12]}, but the merge "
                                   f"base is {merge_base[:12]}: re-attest against the current merge base")
                elif not independent(r):
                    reasons.append(f"the accept by {r['reviewer']} is not independent of its implementers "
                                   f"{sorted(r['implementers'])}")
        else:
            reasons.append("no accept at the current head")
    counted = [r for r in valid if r["head"] == head]
    return {
        "covered": ok,
        "reasons": reasons,
        "accepts_at_head": len(at_head),
        "counted": counted,
        "accept_seats": sorted({resolve_seat(r["reviewer"], registered) for r in counted if resolve_seat(r["reviewer"], registered)}),
    }


def read_identities(path):
    """The identities file, validated: a file with a problem stops the tool with the problems listed (see
    `attest_sig.validate_identities`)."""
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
    problems = attest_sig.validate_identities(data)
    if problems:
        raise SystemExit(f"{path} is not a usable identities file:\n  " + "\n  ".join(problems))
    return data


def load_keys(path):
    """Return the raw identities data (seat -> logins/keys) for signature verification."""
    return read_identities(path)


def load_identities(path):
    """Return {login_lower: seat} from governance/identities.json ({"seats": {seat: [logins]}})."""
    data = read_identities(path)
    mapping = {}
    for seat, entry in data.get("seats", {}).items():
        logins = entry.get("logins", []) if isinstance(entry, dict) else entry
        for login in logins:
            mapping[login.lower()] = seat
    return mapping


def reviews_to_records(reviews, repo, number, head, merge_base, implementers, login_to_seat, base_events=()):
    """Turn GitHub PR reviews into attestation-shaped records, plus problems explaining skips.

    Only reviews at the current head by a mapped seat are used. Approval is whole-diff by GitHub's own
    binding of a review to a commit. Implementers must be known, or independence cannot be shown.
    A review submitted before a ``base_events`` timestamp (base_ref_changed / base_ref_force_pushed) is
    skipped: the diff it reviewed is no longer base...head, so only an attestation may restate it.
    """
    records, problems = [], []
    if merge_base in (None, "", "unknown"):
        return records, [{"review": None, "problem": "merge-base unresolved (checkout lacks the PR head?); formal reviews cannot be bound"}]
    # GitHub lists every review chronologically. Only each reviewer's latest decision stands: a later
    # APPROVED supersedes an earlier CHANGES_REQUESTED, and a DISMISSED review is no decision at all.
    latest = {}
    for review in sorted(reviews, key=lambda r: (r.get("submitted_at", ""), r.get("id", 0))):
        if review.get("state") in ("APPROVED", "CHANGES_REQUESTED", "DISMISSED"):
            latest[(review.get("user") or {}).get("login", "").lower()] = review
    for review in latest.values():
        login = (review.get("user") or {}).get("login", "")
        state = review.get("state", "")
        if state not in ("APPROVED", "CHANGES_REQUESTED") or review.get("commit_id") != head:
            continue
        submitted = review.get("submitted_at", "")
        if submitted and any(event > submitted for event in base_events):
            problems.append({"review": review.get("id"), "problem": "base changed after this review was submitted"})
            continue
        seat = login_to_seat.get(login.lower())
        if not seat:
            problems.append({"review": review.get("id"), "problem": f"login {login!r} is not a mapped seat"})
            continue
        if not implementers:
            problems.append({"review": review.get("id"), "problem": "implementers unknown; independence unproven"})
            continue
        records.append(
            {
                "repo": repo,
                "pr": number,
                "head": head,
                "base_sha": merge_base,
                "scope": "whole",
                "reviewer": seat,
                "implementers": list(implementers),
                "verdict": "accept" if state == "APPROVED" else "changes_requested",
                "source": "github_review",
            }
        )
    return records, problems


def pairs_ancestry(pairs):
    known = {(a, b) for a, b in pairs}
    return lambda a, b: a == b or (a, b) in known


def split_subject(records, repo, number):
    """Keep only records that name this repository and PR; the rest are reported, never counted."""
    mine, other = [], []
    for record in records:
        same = record["repo"] == repo and str(record["pr"]) == str(number)
        (mine if same else other).append(record)
    return mine, other


def report_snapshot(snapshot, is_ancestor=None, identities=None, keys=None, require_signatures=False):
    repo = snapshot.get("repo")
    if not repo:
        raise ValueError("snapshot needs a top-level 'repo' (OWNER/NAME) naming the subject repository")
    rows = []
    registered = set(keys["seats"]) if isinstance(keys, dict) and isinstance(keys.get("seats"), dict) else None
    profiles = seat_profiles(keys) if isinstance(keys, dict) else {}
    table = model_table(keys) if isinstance(keys, dict) else []
    for pr in snapshot["prs"]:
        parsed, problems, legacy = parse_attestations(pr.get("comments", []))
        allowed = attest_sig.allowed_signers(keys) if keys else ""
        for record in parsed:
            canon = resolve_seat(record["reviewer"], registered)
            principal = canon or record["reviewer"]
            record["verified"] = bool(record.get("_sig") and canon) and attest_sig.verify(
                record["_canon"], record["_sig"], principal, allowed
            )
        if require_signatures:
            for record in [r for r in parsed if not r["verified"]]:
                problems.append(
                    {"comment": record["comment"], "problem": "unsigned or unverifiable attestation (signatures required)"}
                )
            parsed = [r for r in parsed if r["verified"]]
        structured, foreign = split_subject(parsed, repo, pr["number"])
        for record in foreign:
            problems.append(
                {
                    "comment": record["comment"],
                    "problem": f"attestation subject {record['repo']}#{record['pr']} "
                    f"does not match {repo}#{pr['number']}",
                }
            )
        formal, formal_problems = reviews_to_records(
            pr.get("reviews", []), repo, pr["number"], pr["head"], pr["merge_base"],
            pr.get("implementers", []), identities or {}, pr.get("base_events", []),
        )
        problems = problems + formal_problems
        ancestry = is_ancestor or pairs_ancestry(pr.get("ancestry", []))
        implied = git_implementers(pr.get("commit_log"), pr.get("branch"))
        records, conflicts = [], []
        for record in structured + formal:
            named = set().union(*(seat_names(i) for i in record["implementers"])) if record["implementers"] else set()
            extra = sorted(implied - named)
            if extra and record["verdict"] in ACCEPT and record["head"] == pr["head"] and seat_names(record["reviewer"]) & implied:
                conflicts.append(
                    f"the accept by {record['reviewer']} is not counted: git history names "
                    f"{sorted(seat_names(record['reviewer']) & implied)} as an implementer (branch prefix or "
                    "commit trailer) and the attestation's implementers list does not")
            records.append(dict(record, implementers=list(record["implementers"]) + extra) if extra else record)
        decision = evaluate(pr["head"], pr["merge_base"], records, ancestry, registered)
        families, family_entries, family_notes = independence(decision["counted"], registered, profiles, table)
        decision["reasons"] = decision["reasons"] + conflicts
        loose = unattributed_commits(pr.get("commit_log"), pr.get("branch"))
        if loose:
            decision["reasons"].append(
                f"{loose} commit(s) in base..head name no implementing seat (no Co-Authored-By trailer, no seat-named "
                "branch), so git cannot show who wrote them and independence rests on the attestation's word alone")
        stand_ins = stand_in_seats(pr.get("comments"))
        if stand_ins:
            decision["reasons"].append(
                f"comment(s) on this PR say they were written in lieu of {', '.join(stand_ins)}: the seat that actually "
                "worked on it is not named, so independence rests on the attestation's word alone")
        rows.append(
            {
                "pr": pr["number"],
                "head": pr["head"],
                "base": pr.get("base", "main"),
                "covered": decision["covered"],
                "reasons": decision["reasons"],
                "accept_seats": decision["accept_seats"],
                "accept_families": families,
                "independence": {"entries": family_entries, "notes": family_notes},
                "counted_verdicts": len(decision["counted"]),
                "verdicts_missing_evidence": [r.get("comment") for r in decision["counted"] if not r.get("commands")],
                "structured_attestations": len(structured),
                "authenticated_reviews": len(formal),
                "signed_attestations": sum(1 for r in structured if r.get("verified")),
                "trust": (
                    "signed"
                    if any(r.get("verified") and r["verdict"] == "accept" and r["head"] == pr["head"] for r in structured)
                    else "authenticated"
                    if any(r["verdict"] == "accept" for r in formal)
                    else "attested-only"
                ),
                "malformed": problems,
                "legacy_packets_not_counted": len(legacy),
                "legacy": legacy,
            }
        )
    return rows


def flatten_pages(pages):
    """gh api --paginate --slurp returns a list of pages; each page is a list of items."""
    items = []
    for page in pages:
        items.extend(page if isinstance(page, list) else [page])
    return items


def checkout_matches(remote_url, repo):
    """True when a clone's origin URL names OWNER/NAME (https or ssh form)."""
    cleaned = remote_url.strip().removesuffix(".git").rstrip("/")
    return cleaned.endswith("/" + repo) or cleaned.endswith(":" + repo)


def pr_list_args(repo):
    return ["pr", "list", "-R", repo, "--state", "open", "--limit", "1000", "--json",
            "number,headRefOid,baseRefName,headRefName,isDraft"]


def gh_json(*args):
    out = subprocess.run(
        ["gh", *args], capture_output=True, text=True, encoding="utf-8", errors="replace", check=True
    )
    return json.loads(out.stdout)


def git(repo_dir, *args):
    return subprocess.run(
        ["git", "-C", repo_dir, *args], capture_output=True, text=True, encoding="utf-8", errors="replace",
        check=False,
    )


def live_snapshot(repo, repo_dir):
    remote = git(repo_dir, "remote", "get-url", "origin")
    if remote.returncode != 0 or not checkout_matches(remote.stdout, repo):
        raise SystemExit(
            f"--repo-dir {repo_dir!r} is not a clone of {repo} (origin: {remote.stdout.strip()!r})"
        )
    prs = []
    for item in gh_json(*pr_list_args(repo)):
        pages = gh_json("api", f"repos/{repo}/issues/{item['number']}/comments", "--paginate", "--slurp")
        base = f"origin/{item['baseRefName']}"
        mb = git(repo_dir, "merge-base", base, item["headRefOid"]).stdout.strip() or "unknown"
        log = git(repo_dir, "log", "--format=%x1egit-author: %an <%ae>%n%B", f"{base}..{item['headRefOid']}")
        prs.append(
            {
                "number": item["number"],
                "head": item["headRefOid"],
                "branch": item.get("headRefName", ""),
                "commit_log": log.stdout if log.returncode == 0 else "",
                "base": item["baseRefName"],
                "draft": item["isDraft"],
                "merge_base": mb,
                "comments": flatten_pages(pages),
                "reviews": flatten_pages(
                    gh_json("api", f"repos/{repo}/pulls/{item['number']}/reviews", "--paginate", "--slurp")
                ),
                "base_events": sorted(
                    e.get("created_at", "")
                    for e in flatten_pages(
                        gh_json("api", f"repos/{repo}/issues/{item['number']}/timeline", "--paginate", "--slurp")
                    )
                    if e.get("event") in ("base_ref_changed", "base_ref_force_pushed")
                ),
            }
        )
    return {"repo": repo, "prs": prs}


def live_ancestry(repo_dir):
    return lambda a, b: a == b or git(repo_dir, "merge-base", "--is-ancestor", a, b).returncode == 0


def render(rows):
    lines = ["PR    head      covered  structured  auth  legacy(not counted)  reasons"]
    for r in rows:
        reasons = "; ".join(r["reasons"]) or "-"
        lines.append(
            f"#{r['pr']:<4} {r['head'][:8]}  {str(r['covered']):<7}  {r['structured_attestations']:<10}"
            f"  {r.get('authenticated_reviews', 0):<4}  {r['legacy_packets_not_counted']:<19}  {reasons}"
        )
    return "\n".join(lines)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot")
    parser.add_argument("--repo")
    parser.add_argument("--repo-dir", help="clone of --repo; required with --repo")
    parser.add_argument("--pr", type=int, help="only report this PR")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--identities", help="identity map JSON (seat -> logins/keys); enables formal reviews and signature checks")
    parser.add_argument("--require-signatures", action="store_true",
                        help="enforcement mode: only attestations signed by the reviewer seat's key count")
    args = parser.parse_args(argv)
    if bool(args.snapshot) == bool(args.repo):
        parser.error("give exactly one of --snapshot or --repo")
    if args.repo and not args.repo_dir:
        parser.error("--repo requires an explicit --repo-dir (a clone of that repository)")
    if args.snapshot:
        with open(args.snapshot, encoding="utf-8") as handle:
            snapshot = json.load(handle)
        ancestry = None
    else:
        snapshot = live_snapshot(args.repo, args.repo_dir)
        ancestry = live_ancestry(args.repo_dir)
    if args.pr:
        snapshot["prs"] = [p for p in snapshot["prs"] if p["number"] == args.pr]
    identities = load_identities(args.identities) if args.identities else None
    keys = load_keys(args.identities) if args.identities else None
    if args.require_signatures and not keys:
        parser.error("--require-signatures needs --identities")
    rows = report_snapshot(snapshot, ancestry, identities, keys, args.require_signatures)
    print(json.dumps(rows, indent=2) if args.json else render(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
