//! A merged Job's pull request still carries its title and comment count
//! (23.5). Absent is unknown, so neither is ever written as `null` or `0`.

use crate::{decode, encode, JobDelivery, Settled};

/// What `get_job` answers for a Job whose pull request merged, field for field
/// as `fleet::serving::detail` builds it: the live reading gone, the two kept.
fn fleets_answer_for_a_merged_job() -> JobDelivery {
    JobDelivery {
        commit: Some("fdc4cf46".to_string()),
        pushed: Some("origin/armada/retire-guides".to_string()),
        pull_request: Some("https://forge.invalid/armada/pull/1750".to_string()),
        pull_request_detail: None,
        landed: Some(Settled::Merged),
        unpushed: None,
        pull_request_title: Some("Retire guides 8 and 20".to_string()),
        pull_request_comments: Some(5),
        merged_at: None,
        pull_request_opened_at: None,
    }
}

#[test]
fn a_merged_jobs_title_and_comment_count_cross_byte_for_byte() {
    let sent = fleets_answer_for_a_merged_job();
    let written = encode(&sent).expect("a delivery encodes");
    assert_eq!(
        written,
        r#"{"commit":"fdc4cf46","pushed":"origin/armada/retire-guides","pull_request":"https://forge.invalid/armada/pull/1750","landed":"merged","pull_request_title":"Retire guides 8 and 20","pull_request_comments":5}"#
    );
    let read: JobDelivery = decode("a delivery", written.as_bytes()).expect("it reads");
    assert_eq!(read, sent);
    assert_eq!(encode(&read).expect("it encodes again"), written);
}

/// A count nobody read is no key, and a 23.4 Fleet's answer still reads.
#[test]
fn an_unread_count_carries_no_key_and_a_23_4_answer_still_reads() {
    let unread = JobDelivery {
        pull_request_comments: None,
        merged_at: None,
        ..fleets_answer_for_a_merged_job()
    };
    let written = encode(&unread).expect("a delivery encodes");
    assert!(
        !written.contains("pull_request_comments") && !written.contains("null"),
        "{written}"
    );
    let older: JobDelivery = decode(
        "a 23.4 delivery",
        br#"{"pull_request":"https://forge.invalid/armada/pull/1750","landed":"merged"}"#,
    )
    .expect("it reads");
    assert_eq!(older.pull_request_title, None);
    assert_eq!(older.pull_request_comments, None);
}

/// When Fleet opened the pull request crosses with it, and is no key where unkept.
#[test]
fn the_instant_fleet_opened_it_crosses_and_is_no_key_where_unkept() {
    let sent = JobDelivery {
        pull_request_opened_at: Some(crate::Instant::carried("2026-10-13T09:10:00.000Z")),
        ..fleets_answer_for_a_merged_job()
    };
    let written = encode(&sent).expect("a delivery encodes");
    assert!(
        written.contains(r#""pull_request_opened_at":"2026-10-13T09:10:00.000Z""#),
        "{written}"
    );
    assert_eq!(decode::<JobDelivery>("a delivery", written.as_bytes()).unwrap(), sent);
    let unkept = encode(&fleets_answer_for_a_merged_job()).expect("a delivery encodes");
    assert!(!unkept.contains("opened_at"), "{unkept}");
}
