use std::fmt;

use super::{Check, Mechanism, Policy, Rule};

const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum File {
    Rules,
    Checks,
}

impl File {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rules => "rules.toml",
            Self::Checks => "checks.toml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapKind {
    UnsupportedVersion(u32),
    DuplicateRule,
    DuplicateCheck,
    NoEnforcement,
}

impl GapKind {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedVersion(v) => {
                format!("unsupported version {v}, expected {SUPPORTED_VERSION}")
            }
            Self::DuplicateRule => "duplicate rule id".to_owned(),
            Self::DuplicateCheck => "duplicate check id".to_owned(),
            Self::NoEnforcement => "rule has an empty enforce list".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub id: String,
    pub file: File,
    pub kind: GapKind,
}

impl fmt::Display for Gap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  {}  {}",
            self.id,
            self.file.name(),
            self.kind.message()
        )
    }
}

fn gap(id: &str, file: File, kind: GapKind) -> Gap {
    Gap {
        id: id.to_owned(),
        file,
        kind,
    }
}

#[must_use]
pub fn diagnose(policy: &Policy, mechanism: &Mechanism) -> Vec<Gap> {
    version_gaps(policy)
        .chain(rule_gaps(policy))
        .chain(check_gaps(mechanism))
        .collect()
}

fn version_gaps(policy: &Policy) -> impl Iterator<Item = Gap> {
    let bad = policy.version != SUPPORTED_VERSION;
    bad.then(|| {
        gap(
            "version",
            File::Rules,
            GapKind::UnsupportedVersion(policy.version),
        )
    })
    .into_iter()
}

fn rule_gaps(policy: &Policy) -> impl Iterator<Item = Gap> {
    policy
        .rule
        .iter()
        .enumerate()
        .flat_map(|(i, rule)| gaps_for_rule(&policy.rule[..i], rule))
}

fn gaps_for_rule(earlier: &[Rule], rule: &Rule) -> Vec<Gap> {
    [
        earlier
            .iter()
            .any(|r| r.id == rule.id)
            .then_some(GapKind::DuplicateRule),
        rule.enforce.is_empty().then_some(GapKind::NoEnforcement),
    ]
    .into_iter()
    .flatten()
    .map(|kind| gap(&rule.id, File::Rules, kind))
    .collect()
}

fn check_gaps(mechanism: &Mechanism) -> impl Iterator<Item = Gap> {
    mechanism
        .check
        .iter()
        .enumerate()
        .filter_map(|(i, check)| duplicate_check(&mechanism.check[..i], check))
}

fn duplicate_check(earlier: &[Check], check: &Check) -> Option<Gap> {
    earlier
        .iter()
        .any(|c| c.id == check.id)
        .then(|| gap(&check.id, File::Checks, GapKind::DuplicateCheck))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{parse_mechanism, parse_policy};

    const RULES: &str = r#"
version = 1
[[rule]]
id = "A"
section = "S"
text = "t"
enforce = ["check", "review"]
[[rule]]
id = "B"
section = "S"
text = "t"
enforce = ["artifact"]
"#;

    const CHECKS: &str = r#"
[[check]]
id = "lint"
rules = ["A"]
run = "true"
output = "exit"
[[artifact]]
heading = "Problem"
rules = ["B"]
"#;

    fn kinds(rules: &str, checks: &str) -> Vec<(String, GapKind)> {
        match (parse_policy(rules), parse_mechanism(checks)) {
            (Ok(p), Ok(m)) => diagnose(&p, &m)
                .into_iter()
                .map(|g| (g.id, g.kind))
                .collect(),
            _ => vec![("parse".to_owned(), GapKind::NoEnforcement)],
        }
    }

    fn pair(id: &str, kind: GapKind) -> Vec<(String, GapKind)> {
        vec![(id.to_owned(), kind)]
    }

    #[test]
    fn clean_policy_has_no_gaps() {
        assert_eq!(kinds(RULES, CHECKS), vec![]);
    }

    #[test]
    fn flags_unsupported_version() {
        let rules = RULES.replace("version = 1", "version = 2");
        assert_eq!(
            kinds(&rules, CHECKS),
            pair("version", GapKind::UnsupportedVersion(2))
        );
    }

    #[test]
    fn flags_duplicate_rule_id() {
        let rules = format!(
            "{RULES}[[rule]]\nid = \"A\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"review\"]\n"
        );
        assert_eq!(kinds(&rules, CHECKS), pair("A", GapKind::DuplicateRule));
    }

    #[test]
    fn flags_duplicate_check_id() {
        let checks = format!(
            "{CHECKS}[[check]]\nid = \"lint\"\nrules = []\nrun = \"true\"\noutput = \"exit\"\n"
        );
        assert_eq!(kinds(RULES, &checks), pair("lint", GapKind::DuplicateCheck));
    }

    #[test]
    fn flags_rule_with_empty_enforce() {
        let rules = RULES.replace("[\"artifact\"]", "[]");
        let expected = pair("B", GapKind::NoEnforcement);
        assert_eq!(kinds(&rules, CHECKS), expected);
    }

    #[test]
    fn display_is_one_line_with_id_file_and_message() {
        let g = gap("A", File::Rules, GapKind::DuplicateRule);
        assert_eq!(g.to_string(), "A  rules.toml  duplicate rule id");
    }
}
