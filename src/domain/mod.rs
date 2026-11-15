//! Pure functions over plain data. No IO, no clock, no mutation: `rules/` enforces it.

mod baseline;
mod baseline_selection;
mod check;
mod doctor;
mod git_paths;
mod hook;
mod import;
mod paths;
mod policy;
mod pr;
mod recipes;
mod report;
mod review;
mod rubric;
mod sarif;
mod waiver;
mod working_status;

pub use baseline::{Baseline, new_findings, parse_baseline, record, render_baseline};
pub use baseline_selection::{BaselineSelectionError, baseline_checks, baseline_selection_failure};
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
pub use recipes::{InitOptionsError, init_options_failure, recipe_checks};
pub use report::{explain_configuration, render_json, render_verdict_json, summary, waived_lines};
pub use review::{ReviewGap, Row, parse_review, verify_review};
pub use rubric::render_rubric;
pub use sarif::{Finding, parse_sarif};
pub use waiver::WaiverGap;

pub use working_status::{WorkingStatusError, parse_working_status, working_status_failure};

mod execution_report;
pub use execution_report::execution_report;

mod execution_validation;
pub use execution_validation::{ExecutionError, validate_execution};
