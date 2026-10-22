use std::fmt;

use super::waiver::{self, WaiverGap};
use super::{Artifact, Check, Layer, Mechanism, Policy, Rule};

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
    NoRules,
    DuplicateRule,
    DuplicateCheck,
    NoEnforcement,
    CheckUnmapped,
    ArtifactUnmapped,
    UnknownRule { rule: String },
    Waiver(WaiverGap),
}

impl GapKind {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UnsupportedVersion(v) => {
                format!("unsupported version {v}, expected {SUPPORTED_VERSION}")
            }
            Self::NoRules => "no [[rule]] entries".to_owned(),
            Self::DuplicateRule => "duplicate rule id".to_owned(),
            Self::DuplicateCheck => "duplicate check id".to_owned(),
            Self::NoEnforcement => "rule has an empty enforce list".to_owned(),
            Self::CheckUnmapped => "promises check, but no [[check]] lists it".to_owned(),
            Self::ArtifactUnmapped => "promises artifact, but no [[artifact]] lists it".to_owned(),
            Self::UnknownRule { rule } => format!("names unknown rule {rule}"),
            Self::Waiver(w) => w.message().to_owned(),
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
    header_gaps(policy)
        .chain(rule_gaps(policy, mechanism))
        .chain(check_gaps(policy, mechanism))
        .chain(artifact_gaps(policy, mechanism))
        .chain(waiver::gaps(policy, mechanism))
        .collect()
}

fn header_gaps(policy: &Policy) -> impl Iterator<Item = Gap> {
    let version = policy.version;
    [
        (version != SUPPORTED_VERSION)
            .then(|| gap("version", File::Rules, GapKind::UnsupportedVersion(version))),
        policy
            .rule
            .is_empty()
            .then(|| gap("rules", File::Rules, GapKind::NoRules)),
    ]
    .into_iter()
    .flatten()
}

fn rule_gaps<'a>(policy: &'a Policy, mechanism: &'a Mechanism) -> impl Iterator<Item = Gap> + 'a {
    policy
        .rule
        .iter()
        .enumerate()
        .flat_map(|(i, rule)| gaps_for_rule(&policy.rule[..i], rule, mechanism))
}

fn gaps_for_rule(earlier: &[Rule], rule: &Rule, mechanism: &Mechanism) -> Vec<Gap> {
    [
        earlier
            .iter()
            .any(|r| r.id == rule.id)
            .then_some(GapKind::DuplicateRule),
        rule.enforce.is_empty().then_some(GapKind::NoEnforcement),
        (promises(rule, Layer::Check) && !waiver::check_covered(mechanism, &rule.id))
            .then_some(GapKind::CheckUnmapped),
        (promises(rule, Layer::Artifact) && !mechanism.lists_artifact(&rule.id))
            .then_some(GapKind::ArtifactUnmapped),
    ]
    .into_iter()
    .flatten()
    .map(|kind| gap(&rule.id, File::Rules, kind))
    .collect()
}

fn promises(rule: &Rule, layer: Layer) -> bool {
    rule.enforce.contains(&layer)
}

fn check_gaps<'a>(policy: &'a Policy, mechanism: &'a Mechanism) -> impl Iterator<Item = Gap> + 'a {
    mechanism.check.iter().enumerate().flat_map(|(i, check)| {
        duplicate_check(&mechanism.check[..i], check)
            .into_iter()
            .chain(unknown_rules(policy, &check.id, &check.rules))
    })
}

fn duplicate_check(earlier: &[Check], check: &Check) -> Option<Gap> {
    earlier
        .iter()
        .any(|c| c.id == check.id)
        .then(|| gap(&check.id, File::Checks, GapKind::DuplicateCheck))
}

fn artifact_gaps<'a>(
    policy: &'a Policy,
    mechanism: &'a Mechanism,
) -> impl Iterator<Item = Gap> + 'a {
    mechanism
        .artifact
        .iter()
        .flat_map(|a: &Artifact| unknown_rules(policy, &a.heading, &a.rules))
}

fn unknown_rules<'a>(
    policy: &'a Policy,
    owner: &'a str,
    named: &'a [String],
) -> impl Iterator<Item = Gap> + 'a {
    named
        .iter()
        .filter(|id| !policy.rule.iter().any(|r| &r.id == *id))
        .map(move |id| {
            gap(
                owner,
                File::Checks,
                GapKind::UnknownRule { rule: id.clone() },
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ParseError, parse_mechanism, parse_policy};

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

    fn expect(rules: &str, checks: &str, want: &[(&str, GapKind)]) -> Result<(), ParseError> {
        let (p, m) = (parse_policy(rules)?, parse_mechanism(checks)?);
        let gaps = diagnose(&p, &m);
        let got: Vec<_> = gaps
            .iter()
            .map(|g| (g.id.as_str(), g.kind.clone()))
            .collect();
        assert_eq!(got, want);
        Ok(())
    }

    #[test]
    fn clean_policy_has_no_gaps() -> Result<(), ParseError> {
        expect(RULES, CHECKS, &[])
    }

    #[test]
    fn flags_unsupported_version() -> Result<(), ParseError> {
        let rules = RULES.replace("version = 1", "version = 2");
        expect(
            &rules,
            CHECKS,
            &[("version", GapKind::UnsupportedVersion(2))],
        )
    }

    #[test]
    fn flags_a_policy_with_no_rules() -> Result<(), ParseError> {
        expect("version = 1\n", "", &[("rules", GapKind::NoRules)])
    }

    #[test]
    fn flags_duplicate_rule_id() -> Result<(), ParseError> {
        let rules = format!(
            "{RULES}[[rule]]\nid = \"A\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"review\"]\n"
        );
        expect(&rules, CHECKS, &[("A", GapKind::DuplicateRule)])
    }

    #[test]
    fn flags_duplicate_check_id() -> Result<(), ParseError> {
        let checks = format!(
            "{CHECKS}[[check]]\nid = \"lint\"\nrules = []\nrun = \"true\"\noutput = \"exit\"\n"
        );
        expect(RULES, &checks, &[("lint", GapKind::DuplicateCheck)])
    }

    #[test]
    fn flags_rule_with_empty_enforce() -> Result<(), ParseError> {
        let rules = RULES.replace("[\"artifact\"]", "[]");
        expect(&rules, CHECKS, &[("B", GapKind::NoEnforcement)])
    }

    #[test]
    fn flags_check_layer_with_no_check() -> Result<(), ParseError> {
        let checks = CHECKS.replace("rules = [\"A\"]", "rules = []");
        expect(RULES, &checks, &[("A", GapKind::CheckUnmapped)])
    }

    #[test]
    fn flags_artifact_layer_with_no_artifact() -> Result<(), ParseError> {
        let checks = CHECKS.replace("rules = [\"B\"]", "rules = []");
        expect(RULES, &checks, &[("B", GapKind::ArtifactUnmapped)])
    }

    #[test]
    fn flags_mapping_that_names_an_unknown_rule() -> Result<(), ParseError> {
        let checks = CHECKS.replace("[\"A\"]", "[\"A\", \"Z\"]");
        let unknown = GapKind::UnknownRule {
            rule: "Z".to_owned(),
        };
        expect(RULES, &checks, &[("lint", unknown)])
    }

    #[test]
    fn review_needs_no_mapping() -> Result<(), ParseError> {
        let rules = RULES.replace("[\"check\", \"review\"]", "[\"review\"]");
        let checks = CHECKS.replace("rules = [\"A\"]", "rules = []");
        expect(&rules, &checks, &[])
    }

    #[test]
    fn display_is_one_line_with_id_file_and_message() {
        let g = gap("C2", File::Rules, GapKind::CheckUnmapped);
        assert_eq!(
            g.to_string(),
            "C2  rules.toml  promises check, but no [[check]] lists it"
        );
    }
}
