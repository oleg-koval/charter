use super::Mechanism;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrGap {
    pub heading: String,
    pub message: String,
}

#[must_use]
pub fn verify_pr(mechanism: &Mechanism, body: &str) -> Vec<PrGap> {
    mechanism
        .artifact
        .iter()
        .filter_map(|artifact| verify_section(body, &artifact.heading, artifact.first))
        .collect()
}

fn verify_section(body: &str, heading: &str, first: bool) -> Option<PrGap> {
    let title = format!("## {heading}");
    let lines = body.lines().collect::<Vec<_>>();
    let Some(section) = lines.iter().position(|line| line.trim() == title) else {
        return Some(PrGap {
            heading: heading.to_owned(),
            message: "artifact heading is missing".to_owned(),
        });
    };
    if first && lines.iter().position(is_level_two_heading) != Some(section) {
        return Some(PrGap {
            heading: heading.to_owned(),
            message: "artifact heading is not first".to_owned(),
        });
    }
    if section_text(&lines, section).trim().is_empty() {
        Some(PrGap {
            heading: heading.to_owned(),
            message: "artifact section is empty".to_owned(),
        })
    } else {
        None
    }
}

fn section_text(lines: &[&str], section: usize) -> String {
    let content = lines
        .iter()
        .enumerate()
        .skip(section + 1)
        .take_while(|(index, line)| !is_section_boundary(lines, section, *index, line))
        .map(|(_, line)| *line)
        .collect::<Vec<_>>()
        .join("\n");
    strip_comments(&content)
}

fn is_section_boundary(lines: &[&str], section: usize, index: usize, line: &&str) -> bool {
    let fenced = lines[section + 1..index]
        .iter()
        .filter(|line| line.trim_start().starts_with("```"))
        .count()
        % 2
        == 1;
    !fenced && (line.trim_start().starts_with("# ") || is_level_two_heading(line))
}

fn is_level_two_heading(line: &&str) -> bool {
    line.trim_start().starts_with("## ")
}

fn strip_comments(text: &str) -> String {
    text.split("<!--")
        .enumerate()
        .filter_map(|(index, part)| {
            if index == 0 {
                Some(part)
            } else {
                part.split_once("-->").map(|(_, after)| after)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Artifact, Check};

    fn mechanism() -> Mechanism {
        Mechanism {
            check: Vec::<Check>::new(),
            artifact: vec![Artifact {
                heading: "Problem".to_owned(),
                rules: vec!["C1".to_owned()],
                first: false,
            }],
        }
    }

    #[test]
    fn accepts_nonempty_artifact_section() {
        assert!(verify_pr(&mechanism(), "## Problem\nDetails\n## Other\nText").is_empty());
    }

    #[test]
    fn requires_first_artifact_heading_to_be_the_first_level_two_heading() {
        let mechanism = Mechanism {
            check: Vec::<Check>::new(),
            artifact: vec![Artifact {
                heading: "Problem".to_owned(),
                rules: vec!["C1".to_owned()],
                first: true,
            }],
        };
        assert_eq!(
            verify_pr(&mechanism, "## Earlier\nText\n## Problem\nDetails")
                .first()
                .map(|gap| gap.message.as_str()),
            Some("artifact heading is not first")
        );
    }

    #[test]
    fn section_ends_at_level_one_or_two_headings_outside_fences() {
        assert!(verify_pr(&mechanism(), "## Problem\n### Nested\nDetails").is_empty());
        assert!(
            verify_pr(
                &mechanism(),
                "## Problem\n```md\n# Still text\n```\nDetails"
            )
            .is_empty()
        );
        assert_eq!(
            verify_pr(&mechanism(), "## Problem\n## Next")
                .first()
                .map(|gap| gap.message.as_str()),
            Some("artifact section is empty")
        );
    }

    #[test]
    fn reports_missing_and_empty_sections() {
        assert_eq!(
            verify_pr(&mechanism(), "## Other\nText")
                .first()
                .map(|gap| gap.message.as_str()),
            Some("artifact heading is missing")
        );
        let gaps = verify_pr(&mechanism(), "## Problem\n<!-- template -->\n   ");
        assert_eq!(
            gaps.first().map(|gap| gap.message.as_str()),
            Some("artifact section is empty")
        );
    }

    #[test]
    fn ignores_multiline_html_comments() {
        let gaps = verify_pr(&mechanism(), "## Problem\n<!-- start\ncomment -->\n ");
        assert_eq!(
            gaps.first().map(|gap| gap.heading.as_str()),
            Some("Problem")
        );
    }
}
