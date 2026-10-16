//! The steps a review with comments takes, driven through a runner that records them and answers
//! as scripted. The forge is never called.

use adapter_traits::{DiffSide, ReviewComment, ReviewSaid};

use crate::pull_request_review::{post_with, Step};

fn comments() -> Vec<ReviewComment> {
    vec![
        ReviewComment::Line {
            path: "src/a.rs".into(),
            line: 42,
            side: DiffSide::Left,
            body: "--why this".into(),
        },
        ReviewComment::Reply {
            thread: "PRRT_9".into(),
            body: "agreed".into(),
        },
        ReviewComment::File {
            path: "src/b.rs".into(),
            body: "rename this file".into(),
        },
    ]
}

/// Takes every step, answering `id` for each; the one at `refusing` answers with `sentence`.
fn forge<'a>(
    taken: &'a mut Vec<Step>,
    refusing: Option<(usize, &'a str)>,
) -> impl FnMut(&Step) -> Result<String, String> + 'a {
    move |step| {
        let at = taken.len();
        taken.push(step.clone());
        match refusing {
            Some((index, sentence)) if index == at => Err(sentence.to_string()),
            _ => Ok(format!("ID_{at}")),
        }
    }
}

fn has(step: &Step, name: &str, value: &str) -> bool {
    step.strings.iter().any(|(it, was)| *it == name && was == value)
}

#[test]
fn a_review_opens_pending_fills_each_comment_in_order_and_submits() {
    let mut taken = Vec::new();
    post_with(
        &mut forge(&mut taken, None),
        ("PR_node", "headhead"),
        ReviewSaid::ChangesRequested,
        Some("needs work"),
        &comments(),
        None,
    )
    .expect("posted");
    let names: Vec<&str> = taken
        .iter()
        .map(|step| {
            ["addPullRequestReview(", "addPullRequestReviewThread(", "addPullRequestReviewThreadReply(", "submitPullRequestReview("]
                .into_iter()
                .filter(|name| step.query.contains(name))
                .last()
                .expect("a known mutation")
        })
        .collect();
    assert_eq!(
        names,
        [
            "addPullRequestReview(",
            "addPullRequestReviewThread(",
            "addPullRequestReviewThreadReply(",
            "addPullRequestReviewThread(",
            "submitPullRequestReview("
        ]
    );
    assert!(has(&taken[0], "pr", "PR_node") && has(&taken[0], "commit", "headhead"), "the branch as it stands");
    assert!(has(&taken[1], "review", "ID_0") && has(&taken[1], "side", "LEFT"));
    assert_eq!(taken[1].line, Some(42));
    assert!(has(&taken[2], "thread", "PRRT_9"));
    assert!(taken[3].query.contains("subjectType:FILE") && taken[3].line.is_none());
    assert!(has(&taken[4], "event", "REQUEST_CHANGES") && has(&taken[4], "review", "ID_0"));
    assert_eq!(taken[4].text.as_deref(), Some("needs work"));
}

#[test]
fn the_commit_the_person_read_is_the_one_the_review_opens_at() {
    let mut taken = Vec::new();
    post_with(
        &mut forge(&mut taken, None),
        ("PR_node", "moved_on"),
        ReviewSaid::Commented,
        None,
        &comments()[..1],
        Some("what_was_read"),
    )
    .expect("posted");
    assert!(has(&taken[0], "commit", "what_was_read"));
}

#[test]
fn text_travels_on_standard_input_and_never_in_the_arguments() {
    let mut taken = Vec::new();
    post_with(
        &mut forge(&mut taken, None),
        ("PR_node", "h"),
        ReviewSaid::Approved,
        Some("ship it"),
        &comments(),
        None,
    )
    .expect("posted");
    for step in &taken {
        let args = step.arguments();
        for said in ["--why this", "agreed", "rename this file", "ship it"] {
            assert!(!args.iter().any(|arg| arg.contains(said)), "{said} is in an argument");
        }
        assert_eq!(args.iter().any(|arg| arg == "body=@-"), step.text.is_some());
    }
    assert_eq!(taken[1].text.as_deref(), Some("--why this"));
    let line = taken[1].arguments();
    assert!(line.windows(2).any(|pair| pair == ["-F", "line=42"]));
    assert!(line.windows(2).any(|pair| pair == ["-f", "side=LEFT"]));
}

#[test]
fn a_blank_review_body_sends_no_body() {
    let mut taken = Vec::new();
    post_with(
        &mut forge(&mut taken, None),
        ("PR_node", "h"),
        ReviewSaid::Commented,
        Some("  \n"),
        &comments()[..1],
        None,
    )
    .expect("posted");
    let submit = taken.last().expect("a submit");
    assert_eq!(submit.text, None);
    assert!(has(submit, "event", "COMMENT"));
    assert!(!submit.arguments().iter().any(|arg| arg == "body=@-"));
}

#[test]
fn a_comment_the_forge_refuses_deletes_the_pending_review_and_says_why() {
    let mut taken = Vec::new();
    let refused = post_with(
        &mut forge(&mut taken, Some((2, "Line must be part of the diff"))),
        ("PR_node", "h"),
        ReviewSaid::Approved,
        None,
        &comments(),
        None,
    )
    .unwrap_err();
    assert_eq!(refused, "Line must be part of the diff");
    assert_eq!(taken.len(), 4, "open, the line, the refused reply, then the clean-up");
    let last = taken.last().expect("a step");
    assert!(last.query.contains("deletePullRequestReview") && has(last, "review", "ID_0"));
    assert!(
        !taken.iter().any(|step| step.query.contains("submitPullRequestReview")),
        "nothing was submitted"
    );
}

#[test]
fn a_refused_submit_deletes_the_pending_review_too() {
    let mut taken = Vec::new();
    let refused = post_with(
        &mut forge(&mut taken, Some((2, "Can not approve your own pull request"))),
        ("PR_node", "h"),
        ReviewSaid::Approved,
        None,
        &comments()[..1],
        None,
    )
    .unwrap_err();
    assert_eq!(refused, "Can not approve your own pull request");
    assert!(taken.last().expect("a step").query.contains("deletePullRequestReview"));
}

#[test]
fn a_review_the_forge_will_not_open_has_nothing_to_delete() {
    let mut taken = Vec::new();
    let refused = post_with(
        &mut forge(&mut taken, Some((0, "Could not resolve to a node"))),
        ("PR_node", "h"),
        ReviewSaid::Commented,
        None,
        &comments(),
        None,
    )
    .unwrap_err();
    assert_eq!(refused, "Could not resolve to a node");
    assert_eq!(taken.len(), 1, "no review, no clean-up");
}

#[test]
fn a_step_that_answers_with_no_id_is_a_refusal() {
    let mut taken = 0;
    let mut quiet = |_: &Step| {
        taken += 1;
        Ok::<String, String>(if taken == 1 { "ID_0".to_string() } else { String::new() })
    };
    let refused = post_with(
        &mut quiet,
        ("PR_node", "h"),
        ReviewSaid::Commented,
        None,
        &comments()[..1],
        None,
    )
    .unwrap_err();
    assert!(refused.contains("gave nothing back"), "{refused}");
}
