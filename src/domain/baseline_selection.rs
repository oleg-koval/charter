use super::{Check, Mechanism, Output, terminal_text};
use std::fmt;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum BaselineSelectionError {
    Unknown(String),
    Duplicate(String),
    NotSarif(String),
}

impl fmt::Display for BaselineSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "unknown baseline check: {}", terminal_text(id)),
            Self::Duplicate(id) => write!(f, "duplicate baseline check: {}", terminal_text(id)),
            Self::NotSarif(id) => {
                write!(f, "baseline check {} must produce SARIF", terminal_text(id))
            }
        }
    }
}

/// # Errors
/// A selected id must identify exactly one SARIF check.
pub fn baseline_checks<'a>(
    mechanism: &'a Mechanism,
    selected: Option<&str>,
) -> Result<Vec<&'a Check>, BaselineSelectionError> {
    let checks = mechanism
        .check
        .iter()
        .filter(|check| selected.map_or(check.output == Output::Sarif, |id| check.id == id))
        .collect::<Vec<_>>();
    if let Some(id) = selected {
        match checks.as_slice() {
            [check] if check.output == Output::Sarif => {}
            [_] => return Err(BaselineSelectionError::NotSarif(id.to_owned())),
            [] => return Err(BaselineSelectionError::Unknown(id.to_owned())),
            _ => return Err(BaselineSelectionError::Duplicate(id.to_owned())),
        }
    }
    Ok(checks)
}

#[must_use]
pub fn baseline_selection_failure(error: &BaselineSelectionError) -> String {
    serde_json::json!({"event":"charter.baseline_selection.failed", "metric":"charter_baseline_selection_failures", "count":1, "error":error}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Check, Mechanism, Output};

    fn mechanism(entries: &[(&str, Output)]) -> Mechanism {
        Mechanism {
            check: entries
                .iter()
                .map(|(id, output)| Check {
                    id: (*id).to_owned(),
                    rules: vec![],
                    run: "never run".to_owned(),
                    output: *output,
                    on: Some("optional".to_owned()),
                })
                .collect(),
            ..Mechanism::default()
        }
    }
    #[test]
    fn default_preserves_sarif_order_and_selection_ignores_events() {
        let mechanism = mechanism(&[
            ("first", Output::Sarif),
            ("exit", Output::Exit),
            ("last", Output::Sarif),
        ]);
        assert_eq!(
            baseline_checks(&mechanism, None),
            Ok(vec![&mechanism.check[0], &mechanism.check[2]])
        );
        assert_eq!(
            baseline_checks(&mechanism, Some("last")),
            Ok(vec![&mechanism.check[2]])
        );
        assert_eq!(baseline_checks(&Mechanism::default(), None), Ok(vec![]));
    }
    #[test]
    fn selected_id_must_name_exactly_one_sarif_check() {
        for (entries, expected) in [
            (vec![], BaselineSelectionError::Unknown("chosen".to_owned())),
            (
                vec![("chosen", Output::Exit)],
                BaselineSelectionError::NotSarif("chosen".to_owned()),
            ),
            (
                vec![("chosen", Output::Sarif), ("chosen", Output::Exit)],
                BaselineSelectionError::Duplicate("chosen".to_owned()),
            ),
            (
                vec![("chosen", Output::Sarif), ("chosen", Output::Sarif)],
                BaselineSelectionError::Duplicate("chosen".to_owned()),
            ),
        ] {
            assert_eq!(
                baseline_checks(&mechanism(&entries), Some("chosen")),
                Err(expected)
            );
        }
    }
    #[test]
    fn errors_keep_raw_data_but_escape_human_terminal_output() -> Result<(), serde_json::Error> {
        let error = BaselineSelectionError::Unknown("id\n\u{1b}".to_owned());
        let value: serde_json::Value = serde_json::from_str(&baseline_selection_failure(&error))?;
        assert_eq!(value["event"], "charter.baseline_selection.failed");
        assert_eq!(value["metric"], "charter_baseline_selection_failures");
        assert_eq!(value["count"], 1);
        assert_eq!(value["error"]["kind"], "unknown");
        assert_eq!(value["error"]["id"], "id\n\u{1b}");
        assert!(!error.to_string().contains(['\n', '\u{1b}']));
        Ok(())
    }
}
