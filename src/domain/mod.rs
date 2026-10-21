//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod doctor;
mod policy;

pub use doctor::{File, Gap, GapKind, diagnose};
pub use policy::{
    Artifact, Check, Layer, Mechanism, Output, ParseError, Policy, Rule, parse_mechanism,
    parse_policy,
};
