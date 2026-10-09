//! Which seats are engaged: stalls, attribution quality and push credit.
//! Port of `steward_run.stalled_seats`, `attribution_weak` and `pushes` (vector families `stalls` and `pushes`).

use std::collections::{BTreeMap, BTreeSet};

/// One entry of the steward's event log, reduced to the fields these rules read.
#[derive(Debug, Clone, Default)]
pub struct Event {
    /// `"ingest"`, `"cards"` or anything else.
    pub kind: String,
    pub tick: Option<i64>,
    /// `cards` events: the agents that were issued cards.
    pub agents: Vec<String>,
    /// `ingest` events: comments attributed to each seat.
    pub per_seat: BTreeMap<String, i64>,
    /// `ingest` events: pushes credited to each seat.
    pub pushes: BTreeMap<String, i64>,
    /// `ingest` events: comments no seat could be attributed.
    pub unattributed: i64,
}

/// `items[-count:]` with Python's slice rules: a count of zero keeps everything, and a count larger than the list
/// keeps everything.
fn tail<T>(items: &[T], count: i64) -> &[T] {
    let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
    // `items[-count:]` starts at index `-count`: counted from the end when negative, from the front otherwise.
    let start = if -count < 0 {
        (len - count).max(0)
    } else {
        (-count).min(len)
    };
    let start = usize::try_from(start).unwrap_or(0);
    &items[start.min(items.len())..]
}

/// Seats that were issued cards in each of the last `stall_after` ticks but ingested no artifact (a comment or a push)
/// in them. Fewer recorded ticks than `stall_after` means nobody is stalled yet.
#[must_use]
pub fn stalled_seats(events: &[Event], agents: &[String], stall_after: i64) -> Vec<String> {
    let mut ticks: BTreeMap<i64, (BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    for event in events {
        let Some(tick) = event.tick else { continue };
        let slot = ticks.entry(tick).or_default();
        if event.kind == "cards" {
            slot.0.extend(event.agents.iter().map(String::as_str));
        }
        if event.kind == "ingest" {
            for counts in [&event.per_seat, &event.pushes] {
                slot.1.extend(
                    counts
                        .iter()
                        .filter(|(_, n)| **n != 0)
                        .map(|(seat, _)| seat.as_str()),
                );
            }
        }
    }
    let ordered: Vec<i64> = ticks.keys().copied().collect();
    let recent = tail(&ordered, stall_after);
    if i64::try_from(recent.len()).unwrap_or(i64::MAX) < stall_after {
        return Vec::new();
    }
    agents
        .iter()
        .filter(|agent| {
            recent.iter().all(|t| {
                let (issued, produced) = &ticks[t];
                issued.contains(agent.as_str()) && !produced.contains(agent.as_str())
            })
        })
        .cloned()
        .collect()
}

/// True when most recently ingested comments could not be attributed to a seat, so a missing artifact is not evidence
/// of a stall. Pushes do not count as attributed comments.
#[must_use]
pub fn attribution_weak(events: &[Event], window: i64, minimum: i64) -> bool {
    let ingests: Vec<&Event> = events.iter().filter(|e| e.kind == "ingest").collect();
    let recent = tail(&ingests, window);
    let attributed: i64 = recent
        .iter()
        .map(|e| e.per_seat.values().sum::<i64>())
        .sum();
    let total = attributed + recent.iter().map(|e| e.unattributed).sum::<i64>();
    total >= minimum && total > 0 && attributed * 2 < total
}

/// A pull request as push credit sees it.
#[derive(Debug, Clone, Default)]
pub struct PushPr {
    pub number: i64,
    pub head: String,
    pub seat: String,
}

/// `(heads recorded, seats credited)`. A PR whose head moved, or that opened, since the previous ingest is credited to
/// its own seat. `previous` of `None` means no earlier record: the heads are recorded and nothing is credited, so the
/// first run after an upgrade does not credit every open PR. A PR with no known seat is recorded but credits nobody.
#[must_use]
pub fn pushes(
    prs: &[PushPr],
    previous: Option<&BTreeMap<String, String>>,
) -> (BTreeMap<String, String>, BTreeMap<String, i64>) {
    let mut heads = BTreeMap::new();
    let mut credited: BTreeMap<String, i64> = BTreeMap::new();
    for pr in prs {
        let number = pr.number.to_string();
        heads.insert(number.clone(), pr.head.clone());
        let Some(previous) = previous else { continue };
        if previous.get(&number) == Some(&pr.head) {
            continue;
        }
        let seat = pr.seat.to_lowercase();
        if !seat.is_empty() {
            *credited.entry(seat).or_insert(0) += 1;
        }
    }
    (heads, credited)
}

#[cfg(test)]
mod tests {
    use super::{Event, PushPr, attribution_weak, pushes, stalled_seats, tail};
    use std::collections::BTreeMap;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn cards(tick: i64, agents: &[&str]) -> Event {
        Event {
            kind: "cards".into(),
            tick: Some(tick),
            agents: strings(agents),
            ..Event::default()
        }
    }

    fn ingest(tick: i64, per_seat: &[(&str, i64)], unattributed: i64) -> Event {
        Event {
            kind: "ingest".into(),
            tick: Some(tick),
            per_seat: per_seat
                .iter()
                .map(|(s, n)| ((*s).to_owned(), *n))
                .collect(),
            unattributed,
            ..Event::default()
        }
    }

    #[test]
    fn tail_follows_python_slices() {
        let items = [1, 2, 3, 4];
        assert_eq!(tail(&items, 2), [3, 4]);
        assert_eq!(tail(&items, 9), [1, 2, 3, 4]);
        assert_eq!(
            tail(&items, 0),
            [1, 2, 3, 4],
            "items[-0:] is the whole list"
        );
        assert_eq!(tail(&items, -1), [2, 3, 4], "items[1:]");
        assert_eq!(tail(&items, -9), Vec::<i32>::new(), "items[9:]");
    }

    #[test]
    fn a_seat_issued_cards_with_no_artifact_in_every_recent_tick_is_stalled() {
        let agents = strings(&["claude", "mimo"]);
        let events = [
            ingest(1, &[], 0),
            cards(1, &["claude", "mimo"]),
            ingest(2, &[("mimo", 1)], 0),
            cards(2, &["claude", "mimo"]),
        ];
        assert_eq!(stalled_seats(&events, &agents, 2), ["claude"]);
        assert!(
            stalled_seats(&events, &agents, 3).is_empty(),
            "fewer ticks than the threshold"
        );
    }

    #[test]
    fn a_zero_count_per_seat_is_not_an_artifact() {
        let agents = strings(&["claude"]);
        let events = [ingest(1, &[("claude", 0)], 0), cards(1, &["claude"])];
        assert_eq!(stalled_seats(&events, &agents, 1), ["claude"]);
    }

    #[test]
    fn weakness_needs_three_comments_and_under_half_attributed() {
        assert!(attribution_weak(&[ingest(1, &[], 3)], 1, 3));
        assert!(
            !attribution_weak(&[ingest(1, &[("mimo", 3)], 3)], 1, 3),
            "exactly half is not weak"
        );
        assert!(!attribution_weak(&[ingest(1, &[], 2)], 1, 3));
        assert!(
            !attribution_weak(&[], 1, 0),
            "no comments at all is never weak"
        );
    }

    #[test]
    fn pushes_credit_a_moved_or_new_head_to_its_seat_once_a_baseline_exists() {
        let prs = [
            PushPr {
                number: 1,
                head: "c".into(),
                seat: "Claude".into(),
            },
            PushPr {
                number: 2,
                head: "b".into(),
                seat: String::new(),
            },
        ];
        let previous = BTreeMap::from([("1".to_owned(), "a".to_owned())]);
        let (heads, credited) = pushes(&prs, Some(&previous));
        assert_eq!(heads.len(), 2);
        assert_eq!(credited, BTreeMap::from([("claude".to_owned(), 1)]));
        assert!(
            pushes(&prs, None).1.is_empty(),
            "no earlier record credits nobody"
        );
    }
}
