//! What jq prints for a pull request's review threads, read as threads. The forge is never
//! called: what is under test is what each record means.

use adapter_traits::DiffSide;

use crate::pull_request_threads::threads_folded;

fn lines(records: &[&str]) -> Vec<String> {
    records.iter().map(|it| it.to_string()).collect()
}

#[test]
fn a_thread_is_its_first_comment_and_the_replies_after_it() {
    let threads = threads_folded(&lines(&[
        "thread\tPRRT_1\tfalse\tfalse\tsrc/a.rs\t42\tRIGHT",
        "tc\tPRRC_1\tada\tUser\t2026-10-15T09:00:00Z\t\twhy?\\nreally",
        "tc\tPRRC_2\tgrace\tUser\t2026-10-15T09:05:00Z\t\tbecause",
        "thread\tPRRT_2\ttrue\tfalse\tsrc/b.rs\t7\tLEFT",
        "tc\tPRRC_3\tada\tUser\t2026-10-15T10:00:00Z\t\tgone",
    ]));
    assert_eq!(threads.len(), 2);
    let first = &threads[0];
    assert_eq!((first.id.as_str(), first.path.as_str()), ("PRRT_1", "src/a.rs"));
    assert_eq!((first.line, first.side, first.resolved), (Some(42), DiffSide::Right, false));
    assert_eq!(first.comments.len(), 2);
    assert_eq!(first.comments[0].body, "why?\nreally");
    assert_eq!(first.comments[1].author, "grace");
    assert_eq!((threads[1].line, threads[1].side, threads[1].resolved), (Some(7), DiffSide::Left, true));
}

#[test]
fn an_outdated_thread_and_a_file_thread_have_no_line() {
    let threads = threads_folded(&lines(&[
        "thread\tPRRT_1\tfalse\ttrue\tsrc/a.rs\t42\tRIGHT",
        "tc\tPRRC_1\tada\tUser\t2026-10-15T09:00:00Z\t\tmoved",
        "thread\tPRRT_2\tfalse\tfalse\tsrc/b.rs\t\t",
        "tc\tPRRC_2\tada\tUser\t2026-10-15T09:00:00Z\t\twhole file",
    ]));
    assert_eq!(threads[0].line, None, "the line has changed under it");
    assert_eq!(threads[1].line, None, "the forge named none");
    assert_eq!(threads[1].side, DiffSide::Right, "no side is the head's");
}

#[test]
fn a_comment_by_an_account_that_is_gone_is_by_ghost() {
    let threads = threads_folded(&lines(&[
        "thread\tPRRT_1\tfalse\tfalse\tsrc/a.rs\t1\tRIGHT",
        "tc\tPRRC_1\t\t\t2026-10-15T09:00:00Z\t\tstill here",
    ]));
    assert_eq!(threads[0].comments[0].author, "ghost");
}

#[test]
fn a_comment_says_when_it_was_edited_and_whether_an_app_wrote_it() {
    let threads = threads_folded(&lines(&[
        "thread\tPRRT_1\tfalse\tfalse\tsrc/a.rs\t1\tRIGHT",
        "tc\tPRRC_1\tada\tUser\t2026-10-15T09:00:00Z\t2026-10-15T09:20:00Z\tsaid again",
        "tc\tPRRC_2\tlint-app\tBot\t2026-10-15T09:30:00Z\t\tfound one",
        "tc\tPRRC_3\tdeploy[bot]\t\t2026-10-15T09:40:00Z\t\tdeployed",
    ]));
    let said = &threads[0].comments;
    assert_eq!(said[0].id, "PRRC_1");
    assert_eq!(said[0].edited_at.as_deref(), Some("2026-10-15T09:20:00Z"));
    assert!(!said[0].bot, "a person");
    assert_eq!((said[1].bot, said[1].edited_at.as_deref()), (true, None), "never edited");
    assert!(said[2].bot, "the REST spelling of an app's login is one too");
}

#[test]
fn a_thread_with_nothing_said_and_a_notice_are_left_out() {
    let threads = threads_folded(&lines(&[
        "a notice the tool printed",
        "tc\tPRRC_0\tada\tUser\t2026-10-15T08:00:00Z\t\tbefore any thread",
        "thread\tPRRT_1\tfalse\tfalse\tsrc/a.rs\t1\tRIGHT",
        "thread\tPRRT_2\tfalse\tfalse\tsrc/b.rs\t1\tRIGHT",
        "tc\tPRRC_2\tada\tUser\t2026-10-15T09:00:00Z\t\tonly one",
    ]));
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].id, "PRRT_2");
    assert!(threads_folded(&[]).is_empty(), "no threads is an answer");
}
