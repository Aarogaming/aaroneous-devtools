//! Rust port of the devtools steward (lane C99).
//!
//! The Python steward in `scripts/` is the origin and the oracle. `scripts/golden/steward/` holds the committed
//! golden vectors, the language-neutral contract; `tests/golden.rs` runs this crate against every vector family and
//! fails when a family exists that is neither ported nor listed as pending, so a new family cannot be skipped
//! silently. Pure functions only: no I/O, no clock, no environment (the adapters that call `gh` and `git` come last).

#![forbid(unsafe_code)]

pub mod attribution;
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
