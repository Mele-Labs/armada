//! What Fleet files and tells of the comments on a pull request a Job holds, against a scripted
//! forge: a reviewer's new or edited comment is told once, a bot's and the owner's are filed
//! without a word, and a pull request taken on with comments is told in one line.

use adapter_traits::{FromOutside, OnFile, PullComment};

use super::pull_notices::{a_rig, pull, Rig};

const OWNER: &str = "harborowner";
const REVIEWER: &str = "quinnreviewer";

fn said(id: &str, author: &str, at: &str) -> PullComment {
    PullComment {
        id: id.to_string(),
        author: author.to_string(),
        bot: false,
        at: at.to_string(),
        edited_at: None,
        url: format!("https://forge.invalid/armada/pull/7#{id}"),
        body: format!("{id} says this name is unclear."),
        file: None,
    }
}

fn by_a_bot(id: &str, at: &str) -> PullComment {
    PullComment { bot: true, ..said(id, "lintbot[bot]", at) }
}

/// A Job on #7, which the owner opened, read once with nothing said on it.
async fn a_rig_on_seven() -> (Rig, String) {
    let rig = a_rig();
    let id = rig.a_hosted_session_on(7).await;
    let mut seven = pull(7, vec![]);
    seven.author = Some(FromOutside::verbatim(OWNER));
    rig.fleet.fake_vcs().main_ci.followed_are(Some(vec![seven]));
    (rig, id)
}

fn comments_are(rig: &Rig, comments: Vec<PullComment>) {
    rig.fleet.fake_vcs().main_ci.comments_on(7, comments);
}

fn the_file(rig: &Rig, id: &str) -> String {
    std::fs::read_to_string(rig.fleet.scratch_of(id).join("comments-7.md")).unwrap_or_default()
}

fn told_of_comments(rig: &Rig) -> Vec<String> {
    rig.runs_sent_to_the_session().into_iter().filter(|line| line.contains("#7")).collect()
}

#[tokio::test]
async fn a_new_comment_from_a_reviewer_is_told_once() {
    let (rig, id) = a_rig_on_seven().await;
    comments_are(&rig, vec![]);
    rig.reads().await;
    assert!(told_of_comments(&rig).is_empty(), "nothing said yet");

    comments_are(&rig, vec![said("IC_1", REVIEWER, "2026-11-03T10:00:00Z")]);
    rig.reads().await;
    rig.reads().await;

    let told = told_of_comments(&rig);
    let [line] = told.as_slice() else { panic!("told once: {told:?}") };
    let file = rig.fleet.scratch_of(&id).join("comments-7.md");
    assert!(
        line.contains(&format!("#7: 1 new comment from {REVIEWER}, in {}", file.display())),
        "{line}"
    );
    assert!(the_file(&rig, &id).contains("## IC_1 · quinnreviewer · written 2026-11-03T10:00:00Z\n"));
}

#[tokio::test]
async fn an_unchanged_comment_is_never_told_again() {
    let (rig, _) = a_rig_on_seven().await;
    comments_are(&rig, vec![]);
    rig.reads().await;
    comments_are(&rig, vec![said("IC_1", REVIEWER, "2026-11-03T10:00:00Z")]);
    rig.reads().await;

    // Fleet restarted: what was told is read back from the store.
    *rig.fleet.sweeping().lock().await = Default::default();
    rig.reads().await;
    rig.reads().await;

    assert_eq!(told_of_comments(&rig).len(), 1, "{:?}", told_of_comments(&rig));
}

#[tokio::test]
async fn an_edited_comment_is_told_again() {
    let (rig, id) = a_rig_on_seven().await;
    comments_are(&rig, vec![]);
    rig.reads().await;
    let mut comment = said("IC_1", REVIEWER, "2026-11-03T10:00:00Z");
    comments_are(&rig, vec![comment.clone()]);
    rig.reads().await;

    comment.edited_at = Some("2026-11-03T11:00:00Z".into());
    comment.body = "Renamed since, never mind.".into();
    comments_are(&rig, vec![comment]);
    rig.reads().await;
    rig.reads().await;

    let told = told_of_comments(&rig);
    let [_, edited] = told.as_slice() else { panic!("told twice: {told:?}") };
    assert!(edited.contains(&format!("#7: {REVIEWER} edited a comment, in ")), "{edited}");
    let file = the_file(&rig, &id);
    assert!(file.contains("written 2026-11-03T10:00:00Z · edited 2026-11-03T11:00:00Z\n"), "{file}");
    assert!(file.contains("> Renamed since, never mind.\n"), "the file is rewritten: {file}");
}

#[tokio::test]
async fn a_bots_comment_is_filed_and_says_nothing() {
    let (rig, id) = a_rig_on_seven().await;
    comments_are(&rig, vec![]);
    rig.reads().await;
    let bot = by_a_bot("PRRC_1", "2026-11-03T10:00:00Z");
    comments_are(&rig, vec![bot.clone()]);
    rig.reads().await;

    assert!(told_of_comments(&rig).is_empty(), "{:?}", told_of_comments(&rig));
    assert!(the_file(&rig, &id).contains("## PRRC_1 · lintbot[bot] · bot · written"));

    comments_are(&rig, vec![bot, said("IC_2", REVIEWER, "2026-11-03T11:00:00Z")]);
    rig.reads().await;
    let told = told_of_comments(&rig);
    let [line] = told.as_slice() else { panic!("told once: {told:?}") };
    assert!(line.contains(&format!("#7: 1 new comment from {REVIEWER}, in ")), "the bot's stays kept: {line}");
}

#[tokio::test]
async fn the_owners_own_comment_is_filed_and_says_nothing() {
    let (rig, id) = a_rig_on_seven().await;
    comments_are(&rig, vec![]);
    rig.reads().await;
    comments_are(&rig, vec![said("IC_1", OWNER, "2026-11-03T10:00:00Z")]);
    rig.reads().await;
    rig.reads().await;

    assert!(told_of_comments(&rig).is_empty(), "{:?}", told_of_comments(&rig));
    assert!(the_file(&rig, &id).contains("## IC_1 · harborowner · written"));
}

#[tokio::test]
async fn a_pull_request_taken_on_with_comments_is_told_once_for_all() {
    let (rig, id) = a_rig_on_seven().await;
    let mut on_a_line = said("PRRC_3", REVIEWER, "2026-11-03T09:00:00Z");
    on_a_line.file = Some(OnFile { path: "src/lib.rs".into(), line: Some(12), resolved: true });
    comments_are(
        &rig,
        vec![
            said("IC_2", REVIEWER, "2026-11-03T10:00:00Z"),
            by_a_bot("PRRC_1", "2026-11-03T08:00:00Z"),
            on_a_line,
        ],
    );

    rig.reads().await;
    rig.reads().await;

    let told = told_of_comments(&rig);
    let [line] = told.as_slice() else { panic!("told once: {told:?}") };
    assert!(line.contains("#7 already has 3 comments, in "), "{line}");
    let file = the_file(&rig, &id);
    let headings: Vec<&str> = file.lines().filter(|line| line.starts_with("## ")).collect();
    assert_eq!(
        headings,
        [
            "## PRRC_1 · lintbot[bot] · bot · written 2026-11-03T08:00:00Z",
            "## PRRC_3 · quinnreviewer · src/lib.rs:12 · resolved · written 2026-11-03T09:00:00Z",
            "## IC_2 · quinnreviewer · written 2026-11-03T10:00:00Z",
        ],
        "oldest first: {file}"
    );
}
