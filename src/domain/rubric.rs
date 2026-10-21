use super::{Layer, Policy};

#[must_use]
pub fn render_rubric(policy: &Policy) -> String {
    let rules = policy
        .rule
        .iter()
        .filter(|rule| rule.enforce.contains(&Layer::Review))
        .map(|rule| format!("- {} | {} | {}", rule.id, rule.section, rule.text))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Review every rule below and return exactly one table row per rule.\n\n{rules}\n\n| rule | verdict | evidence |\n|---|---|---|\nVerdict must be PASS, FAIL, or N-A. Evidence is required for PASS and FAIL."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::parse_policy;

    #[test]
    fn includes_review_rules_in_file_order_and_skips_others() {
        let policy = parse_policy(
            "version = 1\n[[rule]]\nid = \"A\"\nsection = \"Code\"\ntext = \"first\"\nenforce = [\"review\"]\n[[rule]]\nid = \"B\"\nsection = \"Data\"\ntext = \"skip\"\nenforce = [\"check\"]\n[[rule]]\nid = \"C\"\nsection = \"Data\"\ntext = \"last\"\nenforce = [\"review\"]\n",
        );
        let rendered = policy.map(|p| render_rubric(&p));
        let text = rendered.unwrap_or_default();
        assert!(text.contains("- A | Code | first\n- C | Data | last"));
        assert!(!text.contains("skip"));
        assert!(text.contains("| rule | verdict | evidence |"));
        assert!(text.contains("PASS, FAIL, or N-A"));
    }
}
