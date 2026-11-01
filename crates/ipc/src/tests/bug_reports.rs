//! A kept bug report carries `kept` only once the owner has kept it, so Bridge reads a key as present.

use crate::{decode, encode, BugFixed, BugReport, Instant};

fn report(kept: Option<Instant>) -> BugReport {
    BugReport {
        id: "b1".into(),
        session_id: "s1".into(),
        job_title: "Build the report call".into(),
        title: "Preview restart drops the lease".into(),
        what: "The slot was unleased.".into(),
        steps: vec!["Press Refresh preview".into()],
        errors: vec!["no lease".into()],
        logs: vec!["/tmp/fleet.log".into()],
        at: Instant::carried("2026-10-29T09:00:00.000Z"),
        kept,
        fixed: None,
    }
}

#[test]
fn a_bug_report_not_kept_sends_no_kept_key_and_reads_back_whole() {
    let bare = report(None);
    let written = encode(&bare).expect("a report encodes");
    assert!(!written.contains("kept"), "{written}");
    assert_eq!(decode::<BugReport>("a report", written.as_bytes()).expect("it reads"), bare);
}

#[test]
fn a_kept_bug_report_sends_its_time_and_reads_back_whole() {
    let kept = report(Some(Instant::carried("2026-10-29T10:00:00.000Z")));
    let written = encode(&kept).expect("a report encodes");
    assert!(written.contains(r#""kept":"2026-10-29T10:00:00.000Z""#), "{written}");
    assert_eq!(decode::<BugReport>("a report", written.as_bytes()).expect("it reads"), kept);
}

#[test]
fn a_bug_report_still_open_sends_no_fixed_key() {
    let written = encode(&report(None)).expect("a report encodes");
    assert!(!written.contains("fixed"), "{written}");
}

#[test]
fn a_fixed_bug_report_sends_the_pull_request_that_fixed_it_and_reads_back_whole() {
    let fixed = BugReport {
        fixed: Some(BugFixed {
            at: Instant::carried("2026-10-31T09:00:00.000Z"),
            pull_request: 41,
            title: "Keep the lease across a refresh".into(),
            url: "https://forge.invalid/o/r/pull/41".into(),
        }),
        ..report(None)
    };
    let written = encode(&fixed).expect("a report encodes");
    assert!(written.contains(r#""fixed":{"at":"2026-10-31T09:00:00.000Z","pull_request":41,"#), "{written}");
    assert_eq!(decode::<BugReport>("a report", written.as_bytes()).expect("it reads"), fixed);
}
