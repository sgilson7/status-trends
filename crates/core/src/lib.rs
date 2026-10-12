//! tool_core: every application rule of Status Trends.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! The path of a file through the core: `csv` reads the text, `columns`
//! and `status` make the default guesses, `settings` reads the person's
//! choices, `analysis` counts per `period`, `insights` applies the rules,
//! `chart` draws, and `dashboard` puts it all in one view. `api` is the one
//! door the shim calls through.

pub mod analysis;
pub mod api;
pub mod chart;
pub mod columns;
pub mod csv;
pub mod dashboard;
pub mod date;
pub mod insights;
pub mod period;
pub mod settings;
pub mod status;
pub mod strings;
