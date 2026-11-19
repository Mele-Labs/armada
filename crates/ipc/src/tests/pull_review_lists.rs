//! The Reviews lists as one value: every list reachable for what Fleet does to all of them, and a list
//! from a Fleet before the fifth one, or before `refreshing`, still read.

use crate::{decode, encode, PullReviewLists};

fn row(number: u64) -> String {
    format!(
        r#"{{"manifest_id":"01MANIFEST","repository":"example-org/harbour","number":{number},"title":"t","branch":"b",
        "address":"https://example.invalid/{number}","draft":false,"checks":"passed","review":"none","head":"bbb",
        "author":"someone","updated_at":"2026-11-13T09:00:00Z"}}"#
    )
}

fn lists(reviewing: Option<&str>) -> String {
    let reviewing = reviewing.map(|rows| format!(r#","reviewing":[{rows}]"#)).unwrap_or_default();
    format!(
        r#"{{"me":"nick","fetched_at":"2026-11-13T09:00:00Z","requested":[{}],"team":[{}],"assigned":[{}],"mine":[{}]{reviewing},"silent":[]}}"#,
        row(1),
        row(2),
        row(3),
        row(4)
    )
}

#[test]
fn every_list_is_reached_for_what_is_done_to_all_of_them_the_fifth_included() {
    let mut read: PullReviewLists = decode("lists", lists(Some(&row(5))).as_bytes()).expect("decodes");
    let numbers: Vec<Vec<u64>> =
        read.lists_mut().into_iter().map(|list| list.iter().map(|it| it.number).collect()).collect();
    assert_eq!(numbers, [[1], [2], [3], [4], [5]], "requested, team, assigned, mine, reviewing");
    for list in read.lists_mut() {
        for it in list {
            it.reviewing = true;
        }
    }
    assert!(read.reviewing[0].reviewing && read.requested[0].reviewing, "a change to every list reaches the fifth");
}

#[test]
fn lists_from_a_fleet_before_the_fifth_list_and_refreshing_decode_and_say_neither_again() {
    let before: PullReviewLists = decode("lists", lists(None).as_bytes()).expect("decodes with no reviewing");
    assert!(before.reviewing.is_empty() && !before.refreshing);
    let text = encode(&before).expect("encodes");
    assert!(!text.contains("reviewing") && !text.contains("refreshing"), "both are left off when empty: {text}");
}
