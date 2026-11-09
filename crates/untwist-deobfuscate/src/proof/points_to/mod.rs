//! Allocation-site points-to analysis for the safe-lane built-in proof (v0.4.0, V4-N12).
//!
//! Behind the `BUILTIN_POINTS_TO` rule switch and not yet wired into the alias proof.
//! PR 1 builds the solver and constraint model with a differential harness; PR 2 switches
//! the proof over and deletes the may-hold bits engine.

mod object;
mod solve;

pub const BUILTIN_POINTS_TO: &str = "builtin-points-to";

#[cfg(test)]
mod tests;
