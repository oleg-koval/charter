//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod baseline;
mod check;
mod doctor;
mod hook;
mod policy;
mod pr;
mod report;
mod review;
mod rubric;
mod sarif;

pub use baseline::{Baseline, new_findings, parse_baseline, record, render_baseline};
pub use check::{Line, Outcome, changed_outcomes, select, verdict};
pub use doctor::{File, Gap, GapKind, diagnose};
pub use hook::edited_file;
pub use policy::{
    Artifact, Check, Layer, Mechanism, Output, ParseError, Policy, Rule, parse_mechanism,
    parse_policy,
};
pub use pr::{PrGap, verify_pr};
pub use report::{render_json, render_verdict_json, summary};
pub use review::{ReviewGap, Row, parse_review, verify_review};
pub use rubric::render_rubric;
pub use sarif::{Finding, parse_sarif};
