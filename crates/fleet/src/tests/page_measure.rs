//! The claim, written before the code: **a page that asks and buries its ask is kept and is not put to him.**
//! More than 120 words of body text before the first `<armada-vote>`, `<armada-action>` or `<h2>`, or no sentence
//! at all before it, and the Job is told what to move. A page that opens short and puts the ask up front passes.

use ipc::SessionId;

use super::session_host::{eventually, rig_placing};
use crate::session_host::page_measure::{buried_ask, MOST_WORDS_BEFORE_THE_ASK};

const PATH: &str = "/work/.armada/documents/design.html";

fn words(count: usize) -> String {
    vec!["word"; count].join(" ")
}

fn page(body: &str) -> String {
    format!(r#"<html><head><title>T</title><meta name="armada-review" content="approve"></head><body>{body}</body></html>"#)
}

#[test]
fn a_short_page_that_opens_with_a_sentence_and_puts_the_ask_next_passes() {
    let page = page(
        r#"<h1>Which look</h1><p>Pick the look for the card.</p>
        <armada-vote id="look" form="pick" question="Which look?"><armada-take id="a" title="A"></armada-take><armada-take id="b" title="B"></armada-take></armada-vote>
        <h2>Detail</h2><p>A long account follows that is never read before the vote.</p>"#,
    );
    assert_eq!(buried_ask(&page, PATH), None);
}

#[test]
fn a_page_with_two_hundred_words_before_its_first_block_is_named_with_the_count_and_where_it_stops() {
    let overloaded = page(&format!("<h1>Findings</h1><p>{}</p><h2>Decision</h2><p>Approve it.</p>", words(200)));
    let told = buried_ask(&overloaded, PATH).expect("200 words is past the limit");
    assert!(told.starts_with(&format!("In the page {PATH}, 200 words come before the first <h2>")), "{told}");
    assert!(told.contains("fold detail below them") && told.contains("not put to him"), "{told}");
    assert!(!told.contains('\n') && !told.contains('\u{2014}'), "{told}");
}

#[test]
fn the_limit_is_a_hundred_and_twenty_words_exactly_and_headings_are_not_counted() {
    let at = |count: usize| page(&format!("<h1>{}</h1><p>{}</p><armada-action id=\"go\" act=\"message\" label=\"Go\"></armada-action>", words(50), words(count)));
    assert_eq!(buried_ask(&at(MOST_WORDS_BEFORE_THE_ASK), PATH), None);
    let told = buried_ask(&at(MOST_WORDS_BEFORE_THE_ASK + 1), PATH).expect("121 words");
    assert!(told.contains("121 words come before the first <armada-action>"), "{told}");
}

#[test]
fn the_head_scripts_styles_and_comments_are_not_body_text() {
    let page = format!(
        "<html><head><title>{w}</title><style>{w}</style></head><body><script>{w}</script><!-- {w} --><p>One sentence.</p><h2>Ask</h2></body></html>",
        w = words(300)
    );
    assert_eq!(buried_ask(&page, PATH), None);
}

#[test]
fn a_page_with_no_sentence_before_its_ask_is_named() {
    let told = buried_ask(&page("<h1>Title</h1><h2>Decision</h2><p>Approve it.</p>"), PATH).expect("nothing opens it");
    assert!(told.starts_with(&format!("In the page {PATH}, no sentence comes before the first <h2>.")), "{told}");
    let vote = buried_ask(&page(r#"<h1>Title</h1><armada-vote id="v" form="pick"></armada-vote>"#), PATH).expect("nothing opens it");
    assert!(vote.contains("before the first <armada-vote>"), "{vote}");
}

#[test]
fn a_page_with_no_block_that_holds_the_ask_is_measured_to_its_end() {
    assert_eq!(buried_ask(&page("<p>Findings.</p>"), PATH), None);
    let told = buried_ask(&page(&format!("<p>{}</p>", words(130))), PATH).expect("130 words and no ask block");
    assert!(told.contains("130 words come before the end of the page"), "{told}");
}

/// **The Job is told, and the page does not ask.** The revision is kept as one for reading, so nothing waits on him.
#[tokio::test]
async fn an_overloaded_page_that_asks_is_kept_but_not_put_to_him_and_the_job_is_told() {
    let rig = rig_placing(None);
    let (id, at) = super::documents::a_job(&rig).await;
    let (file, path) = super::documents::page_at_root(&rig, "design.html");
    std::fs::write(&file, page(&format!("<h1>Findings</h1><p>{}</p><h2>Decision</h2>", words(300)))).unwrap();
    super::documents::writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { super::documents::revisions_of(&record_of(&rig, &id).await, &path).is_some() }).await;
    eventually(|| async { rig.rows(&id).await.iter().any(is_told_about_the_ask) }).await;

    let record = record_of(&rig, &id).await;
    assert_eq!(super::documents::asked(&record, &path).as_deref(), Some("false"));
    assert!(!super::documents::waits_on(&record, &path), "{:?}", record.waiting_for);

    // Shortened, the next revision asks.
    std::fs::write(&file, page("<h1>Findings</h1><p>Approve the design.</p><h2>Decision</h2>")).unwrap();
    super::documents::writes(&rig, at, &path, "toolu_2", "Write", " +10").await;
    eventually(|| async { super::documents::waits_on(&record_of(&rig, &id).await, &path) }).await;
    assert_eq!(super::documents::asked(&record_of(&rig, &id).await, &path).as_deref(), Some("true"));
}

/// **A vote asks him without an Approve mark**, so a page that buries one is named to its Job as well.
#[tokio::test]
async fn an_overloaded_page_with_a_vote_and_no_approve_mark_is_named_to_the_job() {
    let rig = rig_placing(None);
    let (id, at) = super::documents::a_job(&rig).await;
    let (file, path) = super::documents::page_at_root(&rig, "takes.html");
    std::fs::write(&file, page(&format!(r#"<h1>Takes</h1><p>{}</p><armada-vote id="v" form="pick"></armada-vote>"#, words(300)))).unwrap();
    super::documents::writes(&rig, at, &path, "toolu_1", "Write", " +10").await;
    eventually(|| async { rig.rows(&id).await.iter().any(|row| matches!(row, ipc::SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" && text.contains("300 words come before the first <armada-vote>"))) }).await;
}

fn is_told_about_the_ask(row: &ipc::SessionRow) -> bool {
    matches!(row, ipc::SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" && text.contains("300 words come before the first <h2>"))
}

async fn record_of(rig: &super::session_host::Rig, id: &SessionId) -> ipc::SessionRecord {
    super::documents::record_of(rig, id).await
}
