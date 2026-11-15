use std::fmt;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkingStatusError {
    MalformedRecord { index: usize },
}

impl fmt::Display for WorkingStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedRecord { index } => {
                write!(f, "malformed Git status record at index {index}")
            }
        }
    }
}

/// Parses porcelain-v1 `-z --no-renames` records after strict UTF-8 decoding.
///
/// # Errors
/// Rejects malformed headers, empty paths, embedded NULs and rename/copy records.
pub fn parse_working_status(records: &[String]) -> Result<Vec<String>, WorkingStatusError> {
    records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            status_path(record)
                .map(str::to_owned)
                .ok_or(WorkingStatusError::MalformedRecord { index })
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()
        .map(|paths| paths.into_iter().collect())
}

fn status_path(record: &str) -> Option<&str> {
    let valid = match record.as_bytes().get(..3)? {
        [b'?', b'?', b' '] => true,
        [x, y, b' '] => {
            b" MTADU".contains(x) && b" MTADU".contains(y) && (*x != b' ' || *y != b' ')
        }
        _ => false,
    };
    record
        .get(3..)
        .filter(|path| valid && !path.is_empty() && !path.contains('\0'))
}

#[must_use]
pub fn working_status_failure(error: &WorkingStatusError) -> String {
    serde_json::json!({"event":"charter.working_status.failed", "metric":"charter_working_status_failures", "count":1, "error":error}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_records_preserve_paths_and_return_sorted_unique_paths() {
        let records = [
            " M z.rs",
            "M  z.rs",
            "??   odd'\t\nλ.rs",
            " D deleted.rs",
            "UU conflict.rs",
            " T type.rs",
        ]
        .map(str::to_owned);
        assert_eq!(
            parse_working_status(&records),
            Ok(vec![
                "  odd'\t\nλ.rs",
                "conflict.rs",
                "deleted.rs",
                "type.rs",
                "z.rs"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect())
        );
        assert_eq!(parse_working_status(&[]), Ok(vec![]));
    }

    #[test]
    fn malformed_records_are_errors_instead_of_empty_selection() {
        for record in [
            "",
            "M",
            "M ",
            " M ",
            "??",
            "?? ",
            "?? \0bad",
            "X  unknown",
            "??missing separator",
            "   unchanged",
            "R  rename",
        ] {
            assert_eq!(
                parse_working_status(&[" M valid.rs".to_owned(), record.to_owned()]),
                Err(WorkingStatusError::MalformedRecord { index: 1 })
            );
        }
    }

    #[test]
    fn failure_projection_is_a_local_metric_sample() -> Result<(), serde_json::Error> {
        let error = WorkingStatusError::MalformedRecord { index: 2 };
        let value: serde_json::Value = serde_json::from_str(&working_status_failure(&error))?;
        assert_eq!(value["event"], "charter.working_status.failed");
        assert_eq!(value["metric"], "charter_working_status_failures");
        assert_eq!(value["count"], 1);
        assert_eq!(value["error"]["kind"], "malformed_record");
        assert_eq!(value["error"]["index"], 2);
        assert_eq!(error.to_string(), "malformed Git status record at index 2");
        Ok(())
    }
}
