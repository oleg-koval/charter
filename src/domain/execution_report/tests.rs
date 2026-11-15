use super::{execution_report as report, *};
use crate::domain::{parse_mechanism, parse_sarif, record};

fn mechanism() -> Mechanism {
    parse_mechanism("[[check]]\nid='pass'\nrules=['C1']\nrun='true'\noutput='exit'\n[[check]]\nid='fail'\nrules=['C1']\nrun='false'\noutput='exit'\n[[check]]\nid='ast'\nrules=['C2']\nrun='analyzer'\noutput='sarif'\n[[check]]\nid='event'\nrules=['C3']\nrun='never'\noutput='exit'\non='ci'\n[[waive]]\nrule='C4'\nreason='not implemented'").unwrap()
}

fn exit(id: &str, code: i32) -> Outcome {
    Outcome::Exit {
        check_id: id.to_owned(),
        code: Some(code),
    }
}

fn sarif(findings: Vec<crate::domain::Finding>) -> Outcome {
    Outcome::Sarif {
        check_id: "ast".to_owned(),
        findings,
    }
}

fn sample() -> (Baseline, Vec<Outcome>, Vec<Outcome>) {
    let findings = parse_sarif(include_str!(
        "../../../tests/fixtures/check-baselined/findings.sarif"
    ))
    .unwrap();
    let baseline = record(
        record(Baseline::default(), "ast", &findings),
        "unrelated",
        &findings,
    );
    let fresh = crate::domain::Finding {
        fingerprint: "new".to_owned(),
        ..findings[0].clone()
    };
    let raw = vec![
        exit("pass", 0),
        exit("fail", 7),
        sarif([findings.clone(), vec![fresh]].concat()),
    ];
    let effective = vec![raw[0].clone(), raw[1].clone(), sarif(findings)];
    (baseline, raw, effective)
}

#[test]
fn report_distinguishes_execution_filtering_and_baseline_storage() {
    let (baseline, raw, effective) = sample();
    let report = report(
        &mechanism(),
        &baseline,
        &raw,
        &effective,
        None,
        "working_tree",
    )
    .unwrap();
    assert_eq!(
        report["counts"],
        json!({"configured":4,"selected":3,"completed":3,"skipped":1,"pass":2,"fail":1})
    );
    assert_eq!(report["ok"], false);
    assert_eq!(report["checks"][1]["result"]["exit_code"], 7);
    assert_eq!(
        report["checks"][2]["result"],
        json!({"raw":2,"effective":1,"new":0,"baselined":1,"filtered":1,"native_exit":"unavailable"})
    );
    assert_eq!(report["checks"][3]["state"], "skipped");
    assert_eq!(report["checks"][3]["result"], serde_json::Value::Null);
    assert_eq!(report["baseline"]["stored_checks"], 2);
    assert_eq!(report["baseline"]["stored_fingerprints"], 2);
}

#[test]
fn empty_selection_and_noop_do_not_establish_enforcement() {
    let mechanism =
        parse_mechanism("[[check]]\nid='event'\nrules=['C3']\nrun='never'\noutput='exit'\non='ci'")
            .unwrap();
    let baseline = Baseline::default();
    let skipped = report(&mechanism, &baseline, &[], &[], None, "all").unwrap();
    assert_eq!(skipped["ok"], true);
    assert_eq!(
        skipped["counts"],
        json!({"configured":1,"selected":0,"completed":0,"skipped":1,"pass":0,"fail":0})
    );
    let noop = [exit("event", 0)];
    let ran = report(&mechanism, &baseline, &noop, &noop, Some("ci"), "all").unwrap();
    for report in [skipped, ran] {
        assert_eq!(report["enforcement_evidence"], "unknown");
        assert_eq!(report["fixture_evidence"], "unknown");
        assert_eq!(report["review_evidence"], "not_assessed");
        assert_eq!(report["artifact_evidence"], "not_assessed");
        assert!(!report.to_string().contains("never"));
    }
}

#[test]
fn inactive_duplicate_id_has_no_execution_result() {
    let mechanism = parse_mechanism("[[check]]\nid='same'\nrules=['C1']\nrun='tool'\noutput='exit'\n[[check]]\nid='same'\nrules=['C2']\nrun='tool'\noutput='sarif'\non='ci'").unwrap();
    let observed = [exit("same", 0)];
    let baseline = Baseline::default();
    let result = report(&mechanism, &baseline, &observed, &observed, None, "all").unwrap();
    assert_eq!(result["counts"]["selected"], 1);
    assert_eq!(result["counts"]["skipped"], 1);
    assert_eq!(result["checks"][0]["state"], "pass");
    assert_eq!(result["checks"][1]["state"], "skipped");
    assert_eq!(result["checks"][1]["result"], Value::Null);
}

#[test]
fn new_findings_fail_the_check_verdict() {
    let (baseline, raw, effective) = sample();
    let unfiltered = report(&mechanism(), &baseline, &raw, &raw, Some("pr"), "all").unwrap();
    assert_eq!(unfiltered["checks"][2]["state"], "fail");
    assert_eq!(unfiltered["checks"][2]["result"]["new"], 1);
    assert_eq!(unfiltered["waivers"][0]["rule"], "C4");
    assert_eq!(unfiltered["checks"][0]["rules"], json!(["C1"]));
    assert!(report(&mechanism(), &baseline, &[], &raw, None, "all").is_err());
    assert!(report(&mechanism(), &baseline, &raw, &[], None, "all").is_err());
    assert!(report(&mechanism(), &baseline, &effective, &raw, None, "all").is_err());
}

#[test]
fn version_two_report_fails_on_nonzero_or_signalled_sarif_status() {
    let baseline = Baseline::default();
    let outcome = sarif(vec![]);
    let mechanism =
        parse_mechanism("[[check]]\nid='ast'\nrules=['C1']\nrun='analyzer'\noutput='sarif'")
            .unwrap();
    for (status, expected) in [
        (
            NativeExitStatus::Exited(7),
            json!({"kind":"exited","code":7}),
        ),
        (
            NativeExitStatus::Signalled(9),
            json!({"kind":"signalled","signal":9}),
        ),
    ] {
        assert_native_status_report(&mechanism, &baseline, &outcome, status, &expected);
    }
}

#[test]
fn version_two_report_accepts_nonzero_status_with_baselined_findings() {
    let (baseline, raw, effective) = sample();
    let evidence = [NativeExitEvidence {
        check_id: "ast".to_owned(),
        status: NativeExitStatus::Exited(1),
    }];
    let report = execution_report_v2(
        &mechanism(),
        &baseline,
        &raw,
        &effective,
        &evidence,
        None,
        "all",
    )
    .unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["counts"]["pass"], 2);
    assert_eq!(report["counts"]["fail"], 1);
    assert_eq!(report["checks"][2]["state"], "pass");
    assert_eq!(report["checks"][2]["result"]["native_exit"]["code"], 1);
}

#[test]
fn version_two_report_accepts_zero_status_and_reports_counts() {
    let (baseline, raw, effective) = sample();
    let outcome = effective[2].clone();
    let mechanism =
        parse_mechanism("[[check]]\nid='ast'\nrules=['C2']\nrun='analyzer'\noutput='sarif'")
            .unwrap();
    let evidence = [NativeExitEvidence {
        check_id: "ast".to_owned(),
        status: NativeExitStatus::Exited(0),
    }];
    let report = execution_report_v2(
        &mechanism,
        &baseline,
        &[raw[2].clone()],
        &[outcome],
        &evidence,
        None,
        "all",
    )
    .unwrap();
    assert_v2_pass(&report);
}

fn assert_v2_pass(report: &Value) {
    assert_eq!(report["version"], 2);
    assert_eq!(report["ok"], true);
    assert_eq!(report["sarif_native_exit"], "observed");
    assert_eq!(report["counts"]["pass"], 1);
    assert_eq!(report["counts"]["fail"], 0);
    assert_eq!(report["checks"][0]["state"], "pass");
    assert_eq!(
        report["checks"][0]["result"]["native_exit"],
        json!({"kind":"exited","code":0})
    );
}

#[test]
fn version_two_validates_execution_before_native_exit_evidence() {
    let (baseline, _, effective) = sample();
    let evidence = [NativeExitEvidence {
        check_id: "unknown".to_owned(),
        status: NativeExitStatus::Exited(0),
    }];
    assert_eq!(
        execution_report_v2(
            &mechanism(),
            &baseline,
            &[],
            &effective,
            &evidence,
            None,
            "all",
        )
        .unwrap_err(),
        ExecutionError::MismatchedOutcomes
    );
}

fn assert_native_status_report(
    mechanism: &Mechanism,
    baseline: &Baseline,
    outcome: &Outcome,
    status: NativeExitStatus,
    expected: &Value,
) {
    let evidence = [NativeExitEvidence {
        check_id: "ast".to_owned(),
        status,
    }];
    let report = execution_report_v2(
        mechanism,
        baseline,
        std::slice::from_ref(outcome),
        std::slice::from_ref(outcome),
        &evidence,
        None,
        "all",
    )
    .unwrap();
    assert_eq!(report["version"], 2);
    assert_eq!(report["ok"], false);
    assert_eq!(report["checks"][0]["state"], "fail");
    assert_eq!(&report["checks"][0]["result"]["native_exit"], expected);
}

#[test]
fn version_two_rejects_missing_sarif_status() {
    assert!(validate_native_exit_evidence(&mechanism(), None, &[]).is_err());
}
