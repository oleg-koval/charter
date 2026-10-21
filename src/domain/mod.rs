//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod baseline;
mod check;
mod doctor;
mod policy;
mod report;
mod sarif;

pub use baseline::{Baseline, new_findings, parse_baseline, record, render_baseline};
pub use check::{Line, Outcome, select, verdict};
pub use doctor::{File, Gap, GapKind, diagnose};
pub use policy::{
    Artifact, Check, Layer, Mechanism, Output, ParseError, Policy, Rule, parse_mechanism,
    parse_policy,
};
pub use report::{render_json, render_verdict_json, summary};
pub use sarif::{Finding, parse_sarif};
