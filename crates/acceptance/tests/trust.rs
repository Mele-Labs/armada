//! Trust's claim: **work that passes its Check but is wrong gets caught, and I
//! can see that it was caught rightly, before I take it.** #903.
//!
//! Written before the code it names; the compiler's errors are what #903 provides.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A delivering step asks for a review, and the Job holds there with it | That a Drone writes a good one |
//! | A review not accounting for the change is refused, every fault in one pass | That the refusal reaches the Drone as a turn |
//! | A removed test opens its section, named, with its row marked | That Bridge draws it; the Screens story does |
//! | A removed test given no reason needs the person | That pull request CI is read, which is a forge |
//! | The review survives the wire whole | View #904, What should change #907, CI #905, follow-ups #906 |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use verification::{Bucket, ReviewRefused};

use bench::trust::{
    a_review_that_removes_a_test, a_review_wrong_three_ways,
    the_diff, CHANGED, REMOVED_TEST,
};





/// Every fault by name in one pass, so a Drone told to write it again is told everything.
#[test]
fn a_review_that_does_not_account_for_the_change_is_refused() {
    let refused = a_review_wrong_three_ways()
        .against(&CHANGED, the_diff())
        .expect_err("a review leaving the change unaccounted for is refused");
    assert_eq!(
        refused,
        vec![
            ReviewRefused::FileInNoArea {
                path: "crates/fleet/src/admitting.rs".to_string()
            },
            ReviewRefused::FileInNoArea {
                path: "crates/fleet/src/tests/headroom.rs".to_string()
            },
            ReviewRefused::TestNotInDiff {
                name: "a_test_nobody_wrote".to_string()
            },
            ReviewRefused::FindingWithNoReason {
                finding: "Rename a variable".to_string()
            },
        ]
    );
}

/// A removed test the reviewer gave no reason for is not theirs to wave through.
#[test]
fn a_removed_test_with_no_reason_needs_the_person() {
    let accepted = a_review_that_removes_a_test()
        .against(&CHANGED, the_diff())
        .expect("every changed file is in an area and every named test is in the diff");
    let needs_you: Vec<&str> = accepted
        .findings()
        .iter()
        .filter(|finding| finding.bucket() == Bucket::NeedsYou)
        .map(|finding| finding.finding())
        .collect();
    assert!(
        needs_you
            .iter()
            .any(|finding| finding.contains(REMOVED_TEST)),
        "{needs_you:?}"
    );
}

