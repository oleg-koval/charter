use super::{Layer, ParseError, Policy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub rule: String,
    pub verdict: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGap {
    pub rule: String,
    pub message: String,
}

/// # Errors
/// The text contains no table or a table row with other than three columns.
pub fn parse_review(text: &str) -> Result<Vec<Row>, ParseError> {
    let rows = text
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .filter_map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(ParseError("review has no markdown table".to_owned()));
    }
    Ok(rows)
}

fn parse_row(line: &str) -> Option<Result<Row, ParseError>> {
    let cells = cells(line);
    if is_separator(&cells)
        || cells
            .first()
            .is_some_and(|cell| cell.eq_ignore_ascii_case("rule"))
    {
        return None;
    }
    Some(match cells.as_slice() {
        [rule, verdict, evidence] => Ok(Row {
            rule: (*rule).to_owned(),
            verdict: (*verdict).to_owned(),
            evidence: (*evidence).to_owned(),
        }),
        _ => Err(ParseError(
            "review table rows must have three columns".to_owned(),
        )),
    })
}

fn cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

fn is_separator(cells: &[&str]) -> bool {
    cells
        .iter()
        .all(|cell| cell.trim_matches(['-', ':']).is_empty())
}

#[must_use]
pub fn verify_review(policy: &Policy, rows: &[Row]) -> Vec<ReviewGap> {
    let expected = policy
        .rule
        .iter()
        .filter(|rule| rule.enforce.contains(&Layer::Review))
        .map(|rule| rule.id.as_str())
        .collect::<Vec<_>>();
    expected
        .iter()
        .filter_map(
            |id| match rows.iter().filter(|row| row.rule == **id).count() {
                0 => Some(gap(id, "missing review row")),
                1 => None,
                _ => Some(gap(id, "duplicate review row")),
            },
        )
        .chain(rows.iter().filter_map(|row| verify_row(row, &expected)))
        .collect()
}

fn verify_row(row: &Row, expected: &[&str]) -> Option<ReviewGap> {
    if !expected.contains(&row.rule.as_str()) {
        return Some(gap(&row.rule, "unknown review rule"));
    }
    match row.verdict.as_str() {
        "PASS" | "FAIL" if row.evidence.trim().is_empty() => {
            Some(gap(&row.rule, "evidence required for PASS and FAIL"))
        }
        "PASS" | "N-A" => None,
        "FAIL" => Some(gap(&row.rule, "FAIL")),
        _ => Some(gap(&row.rule, "verdict must be PASS, FAIL, or N-A")),
    }
}

fn gap(rule: &str, message: &str) -> ReviewGap {
    ReviewGap {
        rule: rule.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Rule;

    fn policy() -> Policy {
        Policy {
            version: 1,
            rule: vec![Rule {
                id: "A".to_owned(),
                section: "Code".to_owned(),
                text: "a".to_owned(),
                enforce: vec![Layer::Review],
            }],
        }
    }

    fn row(rule: &str, verdict: &str, evidence: &str) -> Row {
        Row {
            rule: rule.to_owned(),
            verdict: verdict.to_owned(),
            evidence: evidence.to_owned(),
        }
    }

    #[test]
    fn parses_verdict_table_rows() {
        assert_eq!(
            parse_review(
                "| rule | verdict | evidence |\n|---|---|---|\n| A | PASS | src/lib.rs:1 |"
            ),
            Ok(vec![row("A", "PASS", "src/lib.rs:1")])
        );
    }

    #[test]
    fn rejects_malformed_table_rows() {
        assert!(
            parse_review("| rule | verdict | evidence |\n|---|---|---|\n| A | PASS |").is_err()
        );
    }

    #[test]
    fn flags_missing_unknown_and_duplicate_rows() {
        let gaps = verify_review(
            &policy(),
            &[
                row("A", "PASS", "line 1"),
                row("A", "PASS", "line 2"),
                row("Z", "N-A", ""),
            ],
        );
        assert_eq!(
            gaps.first().map(|gap| gap.message.as_str()),
            Some("duplicate review row")
        );
        assert_eq!(
            gaps.last().map(|gap| gap.message.as_str()),
            Some("unknown review rule")
        );
        assert_eq!(
            verify_review(&policy(), &[])
                .first()
                .map(|gap| gap.message.as_str()),
            Some("missing review row")
        );
    }

    #[test]
    fn flags_invalid_verdict_missing_evidence_and_fail() {
        let rows = [
            row("A", "MAYBE", "proof"),
            row("A", "PASS", " "),
            row("A", "FAIL", "proof"),
        ];
        let messages = verify_review(&policy(), &rows)
            .into_iter()
            .map(|gap| gap.message)
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec![
                "duplicate review row",
                "verdict must be PASS, FAIL, or N-A",
                "evidence required for PASS and FAIL",
                "FAIL"
            ]
        );
    }
}
