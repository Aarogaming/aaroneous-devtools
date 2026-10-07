//! Rust port of the devtools steward (lane C99).
//!
//! The Python steward in `scripts/` is the origin and the oracle. `scripts/golden/steward/` holds the committed
//! golden vectors, the language-neutral contract; `tests/golden.rs` runs this crate against every vector family and
//! fails when a family exists that is neither ported nor listed as pending, so a new family cannot be skipped
//! silently. Pure functions only: no I/O, no clock, no environment (the adapters that call `gh` and `git` come last).
//!
//! Where the port and the oracle are known to differ (measured with a hostile-mutation differential, 6000 states with
//! wrong types, nulls, deleted keys and odd Unicode, against Python 3.12): on well-formed input they agree (5 seeds x
//! 1600 random states, plus the vectors). On malformed input the port often returns cards where Python raises
//! (it treats a value of the wrong type as absent), and rarely the reverse (`agents` or `prs` as an empty object or
//! string). That leniency is deliberate for now and is not a tested contract; text built from a non-string value
//! (an agent name that is a number) keeps the JSON type in Python and becomes a string here. Unassigned (`Cn`) Unicode
//! code points print raw in a Python-style `repr` because the port has no Unicode table (see `gate::py_repr_str`).

#![forbid(unsafe_code)]

pub mod attribution;
pub mod cards;
pub mod chain;
pub mod checklist;
pub mod coverage;
pub mod engagement;
pub mod gate;
pub mod heads;
pub mod receipts;
pub mod rejected;
pub mod seats;
pub mod text;
pub mod work;
