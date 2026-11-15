use super::*;
use crate::domain::{Finding, parse_mechanism};

fn mechanism() -> Mechanism {
    parse_mechanism("[[check]]\nid='ast'\nrules=['C1']\nrun='tool'\noutput='sarif'\n[[check]]\nid='native'\nrules=['C2']\nrun='tool'\noutput='exit'\non='ci'").unwrap()
}

fn finding(id: &str) -> Finding {
    Finding {
        rule_id: id.to_owned(),
        file: "a.rs".to_owned(),
        line: 1,
        message: "finding".to_owned(),
        fingerprint: id.to_owned(),
    }
}

fn sarif(findings: Vec<Finding>) -> Outcome {
    Outcome::Sarif {
        check_id: "ast".to_owned(),
        findings,
    }
}

#[test]
fn rejects_missing_raw_missing_selected_and_wrong_or_duplicate_ids() {
    let valid = vec![sarif(vec![finding("old")])];
    assert!(validate_execution(&mechanism(), None, &[], &valid).is_err());
    assert!(validate_execution(&mechanism(), None, &valid, &[]).is_err());
    assert!(validate_execution(&mechanism(), Some("ci"), &valid, &valid).is_err());
    assert!(
        validate_execution(
            &mechanism(),
            None,
            &[valid[0].clone(), valid[0].clone()],
            &valid
        )
        .is_err()
    );
    let wrong = [Outcome::Exit {
        check_id: "unknown".to_owned(),
        code: Some(0),
    }];
    assert!(validate_execution(&mechanism(), None, &wrong, &wrong).is_err());
    let wrong_type = [Outcome::Exit {
        check_id: "ast".to_owned(),
        code: Some(0),
    }];
    assert!(validate_execution(&mechanism(), None, &wrong_type, &wrong_type).is_err());
}

fn accepts(raw: &[&str], effective: &[&str]) -> bool {
    let raw = [sarif(raw.iter().map(|id| finding(id)).collect())];
    let effective = [sarif(effective.iter().map(|id| finding(id)).collect())];
    validate_execution(&mechanism(), None, &raw, &effective).is_ok()
}

#[test]
fn findings_preserve_order_multiplicity_and_full_values() {
    assert!(accepts(&["old", "new"], &["old"]));
    assert!(!accepts(&["old", "new"], &["other"]));
    assert!(!accepts(&["old", "new"], &["old", "old"]));
    assert!(!accepts(&["old", "new"], &["new", "old"]));
    assert!(accepts(&["old", "old", "new"], &["old", "old"]));
    assert!(accepts(&["old", "old", "new"], &["old", "new"]));
    assert!(accepts(&["old"], &[]));
    let raw = [sarif(vec![finding("old")])];
    let changed = Finding {
        line: 2,
        ..finding("old")
    };
    assert!(validate_execution(&mechanism(), None, &raw, &[sarif(vec![changed])]).is_err());
}

fn native(code: Option<i32>) -> Outcome {
    Outcome::Exit {
        check_id: "native".to_owned(),
        code,
    }
}

#[test]
fn exit_codes_and_selected_ids_cannot_change() {
    let mechanism =
        parse_mechanism("[[check]]\nid='native'\nrules=['C1']\nrun='tool'\noutput='exit'\non='ci'")
            .unwrap();
    assert_eq!(validate_execution(&mechanism, None, &[], &[]), Ok(()));
    for code in [Some(1), None] {
        let raw = [native(code)];
        assert_eq!(
            validate_execution(&mechanism, Some("ci"), &raw, &raw),
            Ok(())
        );
        assert!(validate_execution(&mechanism, Some("ci"), &raw, &[native(Some(0))]).is_err());
    }
    let duplicate = Mechanism {
        check: [mechanism.check.clone(), mechanism.check].concat(),
        ..Mechanism::default()
    };
    let raw = [native(Some(1))];
    assert!(validate_execution(&duplicate, Some("ci"), &raw, &raw).is_err());
}
