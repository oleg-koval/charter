use super::doctor::{File, Gap, GapKind};
use super::{Layer, Mechanism, Policy, Waiver};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaiverGap {
    UnknownRule,
    BlankReason,
    Stale,
    NotCheck,
    Duplicate,
}

impl WaiverGap {
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::UnknownRule => "waiver names an unknown rule",
            Self::BlankReason => "waiver has a blank reason",
            Self::Stale => "waiver is stale, a [[check]] already lists the rule",
            Self::NotCheck => "waiver names a rule that does not promise check",
            Self::Duplicate => "duplicate waiver for the same rule",
        }
    }
}

pub fn check_covered(mechanism: &Mechanism, rule: &str) -> bool {
    mechanism.lists_check(rule) || mechanism.waive.iter().any(|w| w.rule == rule)
}

pub fn gaps(policy: &Policy, mechanism: &Mechanism) -> Vec<Gap> {
    mechanism
        .waive
        .iter()
        .enumerate()
        .filter_map(|(i, w)| {
            diagnose(policy, mechanism, i, w).map(|kind| Gap {
                id: w.rule.clone(),
                file: File::Checks,
                kind: GapKind::Waiver(kind),
            })
        })
        .collect()
}

fn diagnose(policy: &Policy, mechanism: &Mechanism, i: usize, w: &Waiver) -> Option<WaiverGap> {
    let Some(rule) = policy.rule.iter().find(|r| r.id == w.rule) else {
        return Some(WaiverGap::UnknownRule);
    };
    if mechanism.waive[..i].iter().any(|e| e.rule == w.rule) {
        Some(WaiverGap::Duplicate)
    } else if w.reason.trim().is_empty() {
        Some(WaiverGap::BlankReason)
    } else if !rule.enforce.contains(&Layer::Check) {
        Some(WaiverGap::NotCheck)
    } else if mechanism.lists_check(&rule.id) {
        Some(WaiverGap::Stale)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::WaiverGap;
    use crate::domain::{GapKind, ParseError, diagnose, parse_mechanism, parse_policy};

    const RULES: &str = "version = 1\n\
        [[rule]]\nid = \"A\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"check\"]\n\
        [[rule]]\nid = \"R\"\nsection = \"S\"\ntext = \"t\"\nenforce = [\"review\"]\n";
    const CHECK_A: &str =
        "[[check]]\nid = \"l\"\nrules = [\"A\"]\nrun = \"true\"\noutput = \"exit\"\n";

    fn waive(rule: &str, reason: &str) -> String {
        format!("[[waive]]\nrule = \"{rule}\"\nreason = \"{reason}\"\n")
    }

    fn gaps_for(checks: &str) -> Result<Vec<(String, GapKind)>, ParseError> {
        let (p, m) = (parse_policy(RULES)?, parse_mechanism(checks)?);
        Ok(diagnose(&p, &m)
            .into_iter()
            .map(|g| (g.id, g.kind))
            .collect())
    }
    fn only(id: &str, kind: WaiverGap) -> Vec<(String, GapKind)> {
        vec![(id.to_owned(), GapKind::Waiver(kind))]
    }

    #[test]
    fn a_waiver_clears_only_the_unmapped_check() -> Result<(), ParseError> {
        assert_eq!(gaps_for(&waive("A", "no tooling"))?, vec![]);
        let unwaived = vec![("A".to_owned(), GapKind::CheckUnmapped)];
        assert_eq!(gaps_for("")?, unwaived);
        Ok(())
    }

    #[test]
    fn flags_each_kind_of_bad_waiver() -> Result<(), ParseError> {
        let cases = [
            (
                format!("{}{}", waive("A", "x"), waive("Z", "x")),
                only("Z", WaiverGap::UnknownRule),
            ),
            (waive("A", "  "), only("A", WaiverGap::BlankReason)),
            (
                format!("{CHECK_A}{}", waive("A", "x")),
                only("A", WaiverGap::Stale),
            ),
            (
                format!("{CHECK_A}{}", waive("R", "x")),
                only("R", WaiverGap::NotCheck),
            ),
            (waive("A", "x").repeat(2), only("A", WaiverGap::Duplicate)),
        ];
        let got = cases
            .iter()
            .map(|(c, _)| gaps_for(c))
            .collect::<Result<Vec<_>, _>>()?;
        let want: Vec<_> = cases.iter().map(|(_, w)| w.clone()).collect();
        assert_eq!(got, want);
        Ok(())
    }
}
