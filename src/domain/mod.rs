//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod baseline;
mod check;
mod doctor;
mod git_paths;
mod hook;
mod import;
mod paths;
mod policy;
mod pr;
mod report;
mod review;
mod rubric;
mod sarif;
mod waiver;

pub use baseline::{Baseline, new_findings, parse_baseline, record, render_baseline};
pub use check::{Line, Outcome, changed_outcomes, select, terminal_text, verdict};
pub use doctor::{File, Gap, GapKind, diagnose};
pub use git_paths::{GitPathsError, invalid_path_event};
pub use hook::{edited_file, render_hook_output};
pub use import::{ImportError, import_failure};
pub use paths::{normalise, same_path};
pub use policy::{
    Artifact, Check, Layer, Mechanism, Output, ParseError, Policy, Rule, Waiver, parse_mechanism,
    parse_policy,
};
pub use pr::{PrGap, verify_pr};
pub use report::{render_json, render_verdict_json, summary, waived_lines};
pub use review::{ReviewGap, Row, parse_review, verify_review};
pub use rubric::render_rubric;
pub use sarif::{Finding, parse_sarif};
pub use waiver::WaiverGap;
