use super::*;

#[test]
fn unfiltered_execution_preserves_native_status_and_verdict() {
    for code in [Some(0), Some(7), None] {
        let raw = vec![Outcome::Exit {
            check_id: "native".to_owned(),
            code,
        }];
        let execution = Execution {
            baseline: domain::Baseline::default(),
            raw: raw.clone(),
            filtered: None,
        };
        assert_eq!(execution.effective(), raw);
        assert_eq!(execution.lines().is_empty(), code == Some(0));
    }
}

fn sample_findings() -> Vec<domain::Finding> {
    domain::parse_sarif(include_str!(
        "../../../tests/fixtures/check-baselined/findings.sarif"
    ))
    .unwrap()
}

#[test]
fn filtered_execution_retains_raw_findings_and_baseline() {
    let findings = sample_findings();
    let baseline = domain::record(domain::Baseline::default(), "ast", &findings);
    let fresh = domain::Finding {
        fingerprint: "new".to_owned(),
        ..findings[0].clone()
    };
    let raw = vec![Outcome::Sarif {
        check_id: "ast".to_owned(),
        findings: [findings, vec![fresh]].concat(),
    }];
    let filtered = vec![Outcome::Sarif {
        check_id: "ast".to_owned(),
        findings: vec![],
    }];
    let execution = Execution {
        baseline: baseline.clone(),
        raw: raw.clone(),
        filtered: Some(filtered.clone()),
    };
    assert_eq!(execution.raw, raw);
    assert_eq!(execution.baseline, baseline);
    assert_eq!(execution.effective(), filtered);
    assert!(execution.lines().is_empty());
}
