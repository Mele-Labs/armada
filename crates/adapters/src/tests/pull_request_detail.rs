//! What jq prints for a pull request's page, read as the page. The forge is
//! never called: what is under test is what each record means.

use adapter_traits::{
    CheckStanding, FileChange, Mergeability, PullRequestWrite, ReviewSaid, ReviewStanding, Reviewer,
    ReviewerKind, Said,
};

use crate::pull_request_detail::{
    folded, profile_from, with_profiles, with_teams, Profile, MOST_COMMITS, MOST_FILES, MOST_SAID,
};
use crate::pull_request_writes::arguments;

fn lines(records: &[&str]) -> Vec<String> {
    records.iter().map(|it| it.to_string()).collect()
}

fn head(mergeable: &str, decision: &str) -> String {
    format!("head\tmain\tNickMele\t2026-10-13T15:13:44Z\t1032\t70\t{mergeable}\t{decision}")
}

#[test]
fn a_check_run_and_a_status_context_are_both_rows() {
    let page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "check\tplan\tCOMPLETED\tSUCCESS\t118\thttps://forge.invalid/run/1",
        "check\tfoundations\tIN_PROGRESS\t\t\thttps://forge.invalid/run/2",
        "check\tlint\tCOMPLETED\tSKIPPED\t0\t",
        "check\tbuild\tCOMPLETED\tTIMED_OUT\t900\t",
        "check\tdeploy\tCOMPLETED\tSOMETHING_NEW\t\t",
        "check\tlegacy/ci\t\tPENDING\t\thttps://forge.invalid/ctx",
        "check\tlegacy/lint\t\tFAILURE\t\t",
    ]))
    .expect("a page");
    let standings: Vec<_> = page.checks.iter().map(|it| it.standing).collect();
    assert_eq!(
        standings,
        [
            CheckStanding::Passed,
            CheckStanding::Running,
            CheckStanding::Skipped,
            CheckStanding::Failed,
            CheckStanding::Failed,
            CheckStanding::Running,
            CheckStanding::Failed,
        ]
    );
    assert_eq!(page.checks[0].seconds, Some(118));
    assert_eq!(
        page.checks[0].address.as_deref(),
        Some("https://forge.invalid/run/1")
    );
    assert_eq!(page.checks[1].seconds, None, "still running");
    assert_eq!(page.checks[6].address, None);
}

#[test]
fn each_review_decision_has_its_own_standing() {
    for (said, standing) in [
        ("APPROVED", ReviewStanding::Approved),
        ("CHANGES_REQUESTED", ReviewStanding::ChangesRequested),
        ("REVIEW_REQUIRED", ReviewStanding::Required),
        ("", ReviewStanding::None),
    ] {
        let page = folded(&lines(&[&head("MERGEABLE", said)])).expect("a page");
        assert_eq!(page.review, standing, "{said:?}");
    }
    for (said, standing) in [
        ("MERGEABLE", Mergeability::Mergeable),
        ("CONFLICTING", Mergeability::Conflicting),
        ("UNKNOWN", Mergeability::Unknown),
    ] {
        assert_eq!(folded(&lines(&[&head(said, "")])).unwrap().mergeable, standing);
    }
}

#[test]
fn an_approval_is_counted_once_per_reviewer() {
    let page = folded(&lines(&[
        &head("MERGEABLE", "APPROVED"),
        "approver\tada",
        "approver\tada",
        "approver\tgrace",
    ]))
    .expect("a page");
    assert_eq!(page.approvals, 2);
}

#[test]
fn comments_and_reviews_are_one_conversation_in_time_order() {
    let page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "comment\tgrace\t2026-10-13T16:00:00Z\tsecond\\nline",
        "review\tada\t2026-10-13T15:30:00Z\tCHANGES_REQUESTED\tfix this",
        // A verdict with nothing written is still said, with an empty body.
        "review\tada\t2026-10-13T17:00:00Z\tAPPROVED\t",
        "review\tada\t2026-10-13T18:00:00Z\tDISMISSED\tnote",
    ]))
    .expect("a page");
    let at: Vec<_> = page.conversation.iter().map(Said::at).collect();
    assert_eq!(
        at,
        [
            "2026-10-13T15:30:00Z",
            "2026-10-13T16:00:00Z",
            "2026-10-13T17:00:00Z",
            "2026-10-13T18:00:00Z"
        ]
    );
    assert!(matches!(
        &page.conversation[1],
        Said::Comment { body, .. } if body == "second\nline"
    ));
    assert!(matches!(
        &page.conversation[2],
        Said::Review { verdict: ReviewSaid::Approved, body, .. } if body.is_empty()
    ));
    assert!(matches!(
        &page.conversation[3],
        Said::Review { verdict: ReviewSaid::Commented, .. }
    ));
}

#[test]
fn files_and_commits_read_in_the_pages_words() {
    let page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "file\ta.rs\t3\t1\tADDED",
        "file\tb.rs\t0\t9\tDELETED",
        "file\tc.rs\t2\t2\tRENAMED",
        "file\td.rs\t2\t2\tCOPIED",
        "file\te.rs\t2\t2\tCHANGED",
        "commit\tb42a10898e95f1a21bdd361832111539a8ac2e35\tAdd a thing\tNickMele\t2026-10-13T14:03:10Z",
        "commit\t1234567890\tNo author\t\t",
    ]))
    .expect("a page");
    let changes: Vec<_> = page.files.iter().map(|it| it.change).collect();
    assert_eq!(
        changes,
        [
            FileChange::Added,
            FileChange::Removed,
            FileChange::Renamed,
            FileChange::Modified,
            FileChange::Modified
        ]
    );
    assert_eq!(page.commits[0].sha, "b42a108");
    assert_eq!(page.commits[1].author, None);
    assert_eq!(page.commits[1].at, None);
}

#[test]
fn a_capped_list_keeps_its_first_or_latest_and_counts_what_it_left() {
    let mut records = vec![head("MERGEABLE", "")];
    for n in 0..MOST_FILES + 5 {
        records.push(format!("file\tf{n}.rs\t1\t0\tADDED"));
    }
    for n in 0..MOST_COMMITS + 2 {
        records.push(format!("commit\t{n:040}\tc{n}\t\t"));
    }
    for n in 0..MOST_SAID + 3 {
        records.push(format!("comment\tada\t2026-10-13T15:{:02}:{:02}Z\tn{n}", n / 60, n % 60));
    }
    let page = folded(&records).expect("a page");
    assert_eq!(page.files.len(), MOST_FILES);
    assert_eq!(page.files[0].path, "f0.rs", "the first files");
    assert_eq!(page.commits.len(), MOST_COMMITS);
    assert_eq!(page.conversation.len(), MOST_SAID);
    assert!(
        matches!(&page.conversation[0], Said::Comment { body, .. } if body == "n3"),
        "the latest entries"
    );
    assert_eq!(
        (page.omitted.files, page.omitted.commits, page.omitted.conversation),
        (5, 2, 3)
    );
}

#[test]
fn an_answer_with_no_head_is_the_forges_silence() {
    assert!(folded(&lines(&["a notice gh printed", "file\ta.rs\t1\t1\tADDED"])).is_none());
}

#[test]
fn a_blank_comment_or_request_for_changes_is_refused() {
    for write in [
        PullRequestWrite::Comment { body: "  \n".into() },
        PullRequestWrite::RequestChanges { body: String::new() },
    ] {
        assert!(arguments("7", &write).is_err(), "{write:?}");
    }
}

#[test]
fn the_body_goes_on_standard_input_and_never_in_the_arguments() {
    let comment = PullRequestWrite::Comment { body: "--approve me".into() };
    let (args, body) = arguments("7", &comment).unwrap();
    assert_eq!(args, ["comment", "7", "--body-file", "-"]);
    assert_eq!(body, Some("--approve me"));

    let approve = PullRequestWrite::Approve { body: None };
    assert_eq!(arguments("7", &approve).unwrap(), (vec!["review", "7", "--approve"], None));
    let blank = PullRequestWrite::Approve { body: Some(" ".into()) };
    assert_eq!(arguments("7", &blank).unwrap().1, None);
    let said = PullRequestWrite::Approve { body: Some("looks right".into()) };
    assert_eq!(
        arguments("7", &said).unwrap().0,
        ["review", "7", "--approve", "--body-file", "-"]
    );

    let changes = PullRequestWrite::RequestChanges { body: "no".into() };
    assert_eq!(
        arguments("7", &changes).unwrap().0,
        ["review", "7", "--request-changes", "--body-file", "-"]
    );
}

#[test]
fn the_head_commit_and_the_description_ride_the_head_record() {
    let page = folded(&lines(&[&format!(
        "{}\t3f9c1d2e\tTakes a pull request\\nout of draft",
        head("MERGEABLE", "")
    )]))
    .expect("a page");
    assert_eq!(page.head.as_deref(), Some("3f9c1d2e"));
    assert_eq!(page.body.as_deref(), Some("Takes a pull request\nout of draft"));
    assert!(page.threads.is_empty(), "the threads are a second call");

    let bare = folded(&lines(&[&head("MERGEABLE", "")])).expect("a page");
    assert_eq!((bare.head, bare.body), (None, None));
    let blank = folded(&lines(&[&format!("{}\tabc\t  ", head("MERGEABLE", ""))])).expect("a page");
    assert_eq!(blank.body, None, "a description of spaces is none");
}

#[test]
fn a_person_asked_to_review_is_a_reviewer_with_no_verdict() {
    let page = folded(&lines(&[&head("MERGEABLE", ""), "requested\tperson\tada"])).expect("a page");
    assert_eq!(
        page.reviewers,
        [Reviewer { login: "ada".into(), kind: ReviewerKind::Person, verdict: None, name: None, url: None, team: None }]
    );
}

#[test]
fn a_team_asked_to_review_never_takes_a_persons_verdict() {
    let page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "requested\tteam\tplatform",
        "verdict\tplatform\tAPPROVED",
    ]))
    .expect("a page");
    assert_eq!(
        page.reviewers,
        [
            Reviewer { login: "platform".into(), kind: ReviewerKind::Team, verdict: None, name: None, url: None, team: None },
            Reviewer {
                login: "platform".into(),
                kind: ReviewerKind::Person,
                verdict: Some(ReviewSaid::Approved),
                name: None,
                url: None,
                team: None,
            },
        ]
    );
}

#[test]
fn a_requested_reviewer_who_reviewed_carries_the_verdict_and_an_unasked_one_is_listed() {
    let page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "requested\tperson\tada",
        "requested\tperson\tada",
        "verdict\tada\tCHANGES_REQUESTED",
        "verdict\tgrace\tAPPROVED",
        "verdict\tlin\tDISMISSED",
    ]))
    .expect("a page");
    let seen: Vec<_> = page.reviewers.iter().map(|it| (it.login.as_str(), it.verdict)).collect();
    assert_eq!(
        seen,
        [
            ("ada", Some(ReviewSaid::ChangesRequested)),
            ("grace", Some(ReviewSaid::Approved)),
            ("lin", Some(ReviewSaid::Commented)),
        ]
    );
    assert!(folded(&lines(&[&head("MERGEABLE", "")])).unwrap().reviewers.is_empty());
}

#[test]
fn a_looked_up_profile_is_set_on_each_person_once_and_never_on_a_team() {
    let mut page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "requested\tperson\tada",
        "requested\tteam\tada",
        "requested\tperson\tbob",
        "verdict\tada\tAPPROVED",
        "verdict\tcyd\tCOMMENTED",
    ]))
    .expect("a page");
    let mut asked = Vec::new();
    with_profiles(&mut page.reviewers, |login| {
        asked.push(login.to_string());
        match login {
            "ada" => Some(Profile { name: Some("Ada L".into()), url: Some("https://x/ada".into()) }),
            "bob" => Some(Profile { name: None, url: Some("https://x/bob".into()) }),
            _ => None,
        }
    });
    assert_eq!(asked, ["ada", "bob", "cyd"]);
    let seen: Vec<_> = page
        .reviewers
        .iter()
        .map(|it| (it.login.as_str(), it.kind, it.name.as_deref(), it.url.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            ("ada", ReviewerKind::Person, Some("Ada L"), Some("https://x/ada")),
            ("ada", ReviewerKind::Team, None, None),
            ("bob", ReviewerKind::Person, None, Some("https://x/bob")),
            ("cyd", ReviewerKind::Person, None, None),
        ]
    );
}

#[test]
fn a_person_is_set_under_the_requested_team_they_belong_to_and_a_team_is_read_once() {
    let mut page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "requested\tteam\tplatform",
        "requested\tperson\tada",
        "requested\tperson\tbob",
        "verdict\tcyd\tAPPROVED",
    ]))
    .expect("a page");
    let mut asked = Vec::new();
    with_teams(&mut page.reviewers, |slug| {
        asked.push(slug.to_string());
        Some(vec!["ada".to_string(), "cyd".to_string()])
    });
    assert_eq!(asked, ["platform"]);
    let seen: Vec<_> = page.reviewers.iter().map(|it| (it.login.as_str(), it.kind, it.team.as_deref())).collect();
    assert_eq!(
        seen,
        [
            ("platform", ReviewerKind::Team, None),
            ("ada", ReviewerKind::Person, Some("platform")),
            ("bob", ReviewerKind::Person, None),
            ("cyd", ReviewerKind::Person, Some("platform")),
        ]
    );
}

#[test]
fn a_person_in_two_requested_teams_takes_the_first_and_a_silent_team_groups_nobody() {
    let mut page = folded(&lines(&[
        &head("MERGEABLE", ""),
        "requested\tteam\tgone",
        "requested\tteam\tfirst",
        "requested\tteam\tsecond",
        "requested\tperson\tada",
    ]))
    .expect("a page");
    with_teams(&mut page.reviewers, |slug| match slug {
        "gone" => None,
        _ => Some(vec!["ada".to_string()]),
    });
    let ada = page.reviewers.iter().find(|it| it.login == "ada").unwrap();
    assert_eq!(ada.team.as_deref(), Some("first"));
}

#[test]
fn a_profile_line_with_no_name_keeps_the_address_and_an_empty_one_is_nothing() {
    let full = profile_from("William Martin\thttps://github.com/williammartin\n").unwrap();
    assert_eq!(full.name.as_deref(), Some("William Martin"));
    assert_eq!(full.url.as_deref(), Some("https://github.com/williammartin"));
    let bare = profile_from("\thttps://github.com/ghost\n").unwrap();
    assert_eq!((bare.name, bare.url.as_deref()), (None, Some("https://github.com/ghost")));
    assert!(profile_from("").is_none());
    assert!(profile_from("\t\n").is_none());
}
