//! HUD core, step 1 (lane C120, HUD v2 phase P1a): layered settings and the permission engine.
//!
//! Library only. It reads no files, no environment variables and no clock, and spawns nothing; callers pass each
//! settings layer in as text, so the same crate can sit behind an MCP adapter, a loopback HTTP adapter or a native
//! application. The design is in `governance/proposals/HUD_V2.md` (section 4.2a) and the file format in
//! `contracts/hud-settings-v1.md`.
//!
//! Defaults live in one embedded file (`defaults.toml`). Layers, lowest to highest: defaults, user, project, local.
//! A layer can lock setting paths against the layers above it. A bad reload keeps the previous good configuration.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod layers;
pub mod loaded;
pub mod permissions;

pub use config::{Config, Mode};
pub use error::LoadError;
pub use layers::{Layer, LayerKind};
pub use loaded::{DEFAULTS_TOML, EffectiveValue, Loaded, Manager, Reload};
pub use permissions::{Actor, Decision, FLOOR_DENIED, Reason, Rule, Verdict};
