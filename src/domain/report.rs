use super::{Gap, Mechanism, Policy};

#[must_use]
pub fn summary(policy: &Policy, mechanism: &Mechanism) -> String {
    let waived = mechanism
        .waive
        .iter()
        .map(|w| format!("\nwaived  {}  {}", w.rule, w.reason));
    format!(
        "ok  {} rules, {} checks, {} artifacts",
        policy.rule.len(),
        mechanism.check.len(),
        mechanism.artifact.len()
    ) + &waived.collect::<String>()
}

// Hand-written: serde_json would be a fourth dependency for one flat shape.
#[must_use]
pub fn render_json(gaps: &[Gap]) -> String {
    let items = gaps
        .iter()
        .map(|g| {
            format!(
                "{{\"id\":{},\"file\":{},\"message\":{}}}",
                quote(&g.id),
                quote(g.file.name()),
                quote(&g.kind.message())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"ok\":{},\"gaps\":[{items}]}}", gaps.is_empty())
}

#[must_use]
pub fn render_verdict_json(lines: &[String]) -> String {
    let items = lines.iter().map(|l| quote(l)).collect::<Vec<_>>().join(",");
    format!("{{\"ok\":{},\"lines\":[{items}]}}", lines.is_empty())
}

fn quote(text: &str) -> String {
    let body = text.chars().fold(String::new(), |acc, c| acc + &escape(c));
    format!("\"{body}\"")
}

fn escape(c: char) -> String {
    match c {
        '"' => "\\\"".to_owned(),
        '\\' => "\\\\".to_owned(),
        '\n' => "\\n".to_owned(),
        '\r' => "\\r".to_owned(),
        '\t' => "\\t".to_owned(),
        c if c.is_control() => control(c),
        c => c.to_string(),
    }
}

fn control(c: char) -> String {
    format!("\\u{:04x}", u32::from(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{File, GapKind};

    #[test]
    fn empty_gaps_render_ok_true() {
        assert_eq!(render_json(&[]), "{\"ok\":true,\"gaps\":[]}");
    }

    #[test]
    fn escapes_quotes_and_control_characters() {
        assert_eq!(quote("a\"b\n\u{1}"), "\"a\\\"b\\n\\u0001\"");
    }

    #[test]
    fn renders_a_gap_object() {
        let gap = Gap {
            id: "C2".to_owned(),
            file: File::Rules,
            kind: GapKind::DuplicateRule,
        };
        assert_eq!(
            render_json(&[gap]),
            "{\"ok\":false,\"gaps\":[{\"id\":\"C2\",\"file\":\"rules.toml\",\"message\":\"duplicate rule id\"}]}"
        );
    }
}
