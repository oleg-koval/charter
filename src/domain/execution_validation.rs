use super::{Check, Mechanism, Outcome, Output, select};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    DuplicateSelection,
    MismatchedOutcomes,
    ChangedOutcome,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DuplicateSelection => "execution has duplicate selected check ids",
            Self::MismatchedOutcomes => {
                "execution outcomes do not match selected check ids and types"
            }
            Self::ChangedOutcome => "effective outcomes are not an ordered filter of raw outcomes",
        })
    }
}

impl std::error::Error for ExecutionError {}

/// # Errors
/// Rejects missing, duplicate, mistyped or changed execution observations.
pub fn validate_execution(
    mechanism: &Mechanism,
    event: Option<&str>,
    raw: &[Outcome],
    effective: &[Outcome],
) -> Result<(), ExecutionError> {
    let selected = select(mechanism, event);
    if selected
        .iter()
        .enumerate()
        .any(|(i, c)| selected[..i].iter().any(|prior| prior.id == c.id))
    {
        return Err(ExecutionError::DuplicateSelection);
    }
    if !matches_selection(&selected, raw) || !matches_selection(&selected, effective) {
        return Err(ExecutionError::MismatchedOutcomes);
    }
    if effective
        .iter()
        .any(|outcome| !raw.iter().any(|original| filtered_from(original, outcome)))
    {
        return Err(ExecutionError::ChangedOutcome);
    }
    Ok(())
}

fn matches_selection(selected: &[&Check], outcomes: &[Outcome]) -> bool {
    selected.len() == outcomes.len()
        && selected.iter().all(|check| {
            outcomes
                .iter()
                .filter(|outcome| match outcome {
                    Outcome::Exit { check_id, .. } => {
                        check.id == *check_id && check.output == Output::Exit
                    }
                    Outcome::Sarif { check_id, .. } => {
                        check.id == *check_id && check.output == Output::Sarif
                    }
                })
                .count()
                == 1
        })
}

fn filtered_from(raw: &Outcome, effective: &Outcome) -> bool {
    match (raw, effective) {
        (Outcome::Exit { .. }, Outcome::Exit { .. }) => raw == effective,
        (
            Outcome::Sarif {
                check_id: raw_id,
                findings: raw_findings,
            },
            Outcome::Sarif { check_id, findings },
        ) => {
            raw_id == check_id
                && raw_findings.iter().fold(0, |matched, finding| {
                    matched + usize::from(findings.get(matched) == Some(finding))
                }) == findings.len()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
