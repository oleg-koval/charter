use std::fmt;

use super::{Baseline, Check, Finding, Mechanism, new_findings};

/// A check with no `on` always runs; one with `on` runs only for that event.
#[must_use]
pub fn select<'a>(mechanism: &'a Mechanism, event: Option<&str>) -> Vec<&'a Check> {
    mechanism
        .check
        .iter()
        .filter(|c| c.on.as_deref().is_none_or(|on| Some(on) == event))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `code` is `None` when the process was killed by a signal.
    Exit { check_id: String, code: Option<i32> },
    Sarif {
        check_id: String,
        findings: Vec<Finding>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line(String);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Empty means pass.
#[must_use]
pub fn verdict(baseline: &Baseline, outcomes: &[Outcome]) -> Vec<Line> {
    outcomes
        .iter()
        .flat_map(|outcome| lines(baseline, outcome))
        .collect()
}

fn lines(baseline: &Baseline, outcome: &Outcome) -> Vec<Line> {
    match outcome {
        Outcome::Exit { code: Some(0), .. } => vec![],
        Outcome::Exit {
            check_id,
            code: Some(code),
        } => vec![Line(format!("{check_id}  exited {code}"))],
        Outcome::Exit {
            check_id,
            code: None,
        } => vec![Line(format!("{check_id}  was killed"))],
        Outcome::Sarif { check_id, findings } => new_findings(baseline, check_id, findings)
            .iter()
            .map(|f| {
                Line(format!(
                    "{check_id}  {}:{}  {}: {}",
                    f.file, f.line, f.rule_id, f.message
                ))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Output, record};

    fn check(id: &str, on: Option<&str>) -> Check {
        Check {
            id: id.to_owned(),
            rules: vec![],
            run: "true".to_owned(),
            output: Output::Exit,
            on: on.map(str::to_owned),
        }
    }

    fn finding(fingerprint: &str) -> Finding {
        Finding {
            rule_id: "r1".to_owned(),
            file: "a.rs".to_owned(),
            line: 3,
            message: "bad".to_owned(),
            fingerprint: fingerprint.to_owned(),
        }
    }

    fn ids(checks: &[&Check]) -> Vec<String> {
        checks.iter().map(|c| c.id.clone()).collect()
    }

    #[test]
    fn select_runs_unconditional_checks_and_matching_events() {
        let mechanism = Mechanism {
            check: vec![
                check("a", None),
                check("b", Some("pr")),
                check("c", Some("ci")),
            ],
            artifact: vec![],
        };
        assert_eq!(ids(&select(&mechanism, None)), ["a"]);
        assert_eq!(ids(&select(&mechanism, Some("pr"))), ["a", "b"]);
    }

    #[test]
    fn verdict_reports_failing_exits_and_kills() {
        let outcomes = [
            Outcome::Exit {
                check_id: "ok".to_owned(),
                code: Some(0),
            },
            Outcome::Exit {
                check_id: "bad".to_owned(),
                code: Some(2),
            },
            Outcome::Exit {
                check_id: "sig".to_owned(),
                code: None,
            },
        ];
        let text: Vec<String> = verdict(&Baseline::default(), &outcomes)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(text, ["bad  exited 2", "sig  was killed"]);
    }

    #[test]
    fn verdict_reports_only_new_findings() {
        let baseline = record(Baseline::default(), "ast", &[finding("old")]);
        let outcomes = [Outcome::Sarif {
            check_id: "ast".to_owned(),
            findings: vec![finding("old"), finding("new")],
        }];
        let text: Vec<String> = verdict(&baseline, &outcomes)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(text, ["ast  a.rs:3  r1: bad"]);
        assert!(verdict(&Baseline::default(), &[]).is_empty());
    }
}
