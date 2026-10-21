use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Finding, ParseError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    /// Check id to the fingerprints of findings accepted as existing.
    #[serde(default)]
    pub checks: BTreeMap<String, BTreeSet<String>>,
}

impl Default for Baseline {
    fn default() -> Self {
        Self {
            version: 1,
            checks: BTreeMap::new(),
        }
    }
}

/// # Errors
/// The text is not a valid baseline file.
pub fn parse_baseline(text: &str) -> Result<Baseline, ParseError> {
    serde_json::from_str(text).map_err(|e| ParseError(format!("invalid baseline: {e}")))
}

/// Sorted maps and sets make the output stable, so a re-run produces no diff.
///
/// # Errors
/// Serialization failed, which a map of strings cannot do in practice.
pub fn render_baseline(baseline: &Baseline) -> Result<String, ParseError> {
    serde_json::to_string_pretty(baseline)
        .map(|json| format!("{json}\n"))
        .map_err(|e| ParseError(format!("cannot render baseline: {e}")))
}

/// An unknown check id has no baselined fingerprints, so everything is new.
#[must_use]
pub fn new_findings(baseline: &Baseline, check_id: &str, findings: &[Finding]) -> Vec<Finding> {
    let accepted = baseline.checks.get(check_id);
    findings
        .iter()
        .filter(|f| !accepted.is_some_and(|set| set.contains(&f.fingerprint)))
        .cloned()
        .collect()
}

#[must_use]
pub fn record(baseline: Baseline, check_id: &str, findings: &[Finding]) -> Baseline {
    let fingerprints = findings.iter().map(|f| f.fingerprint.clone()).collect();
    let checks = baseline
        .checks
        .into_iter()
        .filter(|(id, _)| id != check_id)
        .chain([(check_id.to_owned(), fingerprints)])
        .collect();
    Baseline { checks, ..baseline }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(fingerprint: &str) -> Finding {
        Finding {
            rule_id: "r".to_owned(),
            file: "a.rs".to_owned(),
            line: 1,
            message: "m".to_owned(),
            fingerprint: fingerprint.to_owned(),
        }
    }

    fn with(check_id: &str, prints: &[&str]) -> Baseline {
        record(
            Baseline::default(),
            check_id,
            &prints.iter().map(|p| finding(p)).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn baselined_findings_are_not_new() {
        let baseline = with("ast", &["a"]);
        let fresh = new_findings(&baseline, "ast", &[finding("a"), finding("b")]);
        assert_eq!(fresh, vec![finding("b")]);
    }

    #[test]
    fn unknown_check_id_means_everything_is_new() {
        let baseline = with("ast", &["a"]);
        let fresh = new_findings(&baseline, "other", &[finding("a")]);
        assert_eq!(fresh, vec![finding("a")]);
    }

    #[test]
    fn record_replaces_only_that_checks_set() {
        let baseline = record(with("ast", &["a"]), "lint", &[finding("x")]);
        let baseline = record(baseline, "ast", &[finding("b")]);
        assert_eq!(
            baseline.checks.get("ast"),
            with("ast", &["b"]).checks.get("ast")
        );
        assert_eq!(
            baseline.checks.get("lint"),
            with("lint", &["x"]).checks.get("lint")
        );
    }

    #[test]
    fn render_is_deterministic_and_round_trips() {
        let baseline = with("ast", &["b", "a"]);
        let text = render_baseline(&baseline).unwrap_or_default();
        assert_eq!(
            text,
            "{\n  \"version\": 1,\n  \"checks\": {\n    \"ast\": [\n      \"a\",\n      \"b\"\n    ]\n  }\n}\n"
        );
        assert_eq!(parse_baseline(&text), Ok(baseline));
    }

    #[test]
    fn invalid_baseline_names_a_reason() {
        let message = parse_baseline("[]").err().map(|e| e.to_string());
        assert!(
            message
                .unwrap_or_default()
                .starts_with("invalid baseline: ")
        );
    }
}
