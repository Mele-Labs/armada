//! What jq prints for a pull request's comments, read as comments. The forge is never called:
//! what is under test is what each record means.

use adapter_traits::OnFile;

use crate::pull_request_comments::comments_folded;

fn lines(records: &[&str]) -> Vec<String> {
    records.iter().map(|it| it.to_string()).collect()
}

#[test]
fn a_comment_on_the_diff_names_its_file_line_and_whether_it_is_resolved() {
    let comments = comments_folded(&lines(&[
        "c\tIC_1\tgrace\tUser\t2026-10-15T10:00:00Z\t2026-10-15T10:30:00Z\thttps://forge.invalid/1\tCan this\\nwait?",
        "thread\ttrue\tfalse\tcrates/a.rs\t12",
        "tc\tPRRC_1\tada\tUser\t2026-10-15T10:04:00Z\t\thttps://forge.invalid/2\tWhy here?",
        "thread\tfalse\ttrue\tdocs/c.md\t4",
        "tc\tPRRC_2\tlint-app\tBot\t2026-10-15T10:07:00Z\t\thttps://forge.invalid/3\tOld.",
    ]));
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[0].file, None);
    assert_eq!(comments[0].body, "Can this\nwait?");
    assert_eq!(comments[0].edited_at.as_deref(), Some("2026-10-15T10:30:00Z"));
    assert_eq!(comments[1].file, Some(OnFile { path: "crates/a.rs".into(), line: Some(12), resolved: true }));
    assert_eq!(comments[1].url, "https://forge.invalid/2");
    assert_eq!(comments[2].file.as_ref().and_then(|it| it.line), None, "the line has changed under it");
    assert!(comments[2].bot);
}

#[test]
fn a_review_with_no_body_says_nothing_and_a_gone_author_is_ghost() {
    let comments = comments_folded(&lines(&[
        "r\tPRR_1\tada\tUser\t2026-10-15T10:04:00Z\t\thttps://forge.invalid/1\tOne thing.",
        "r\tPRR_2\tada\tUser\t2026-10-15T10:08:00Z\t\thttps://forge.invalid/2\t",
        "c\tIC_1\t\t\t2026-10-15T10:09:00Z\t\thttps://forge.invalid/3\tStill here.",
    ]));
    let ids: Vec<&str> = comments.iter().map(|it| it.id.as_str()).collect();
    assert_eq!(ids, ["PRR_1", "IC_1"]);
    assert_eq!(comments[1].author, "ghost");
    assert!(!comments[1].bot);
}
