//! The claim, written before the code: **a draft the person turns into a Document is a page in the Job's
//! worktree, the draft is withdrawn so it waits no more, the page is kept as the Job's own write is, and the
//! Job is told to revise it there.** A draft that is settled, unknown or a proposed change, and a Job with no
//! worktree, are refused with nothing written. `crates/ipc/operations/convert_draft_to_document.toml`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{AnswerWaiting, ConvertDraftToDocument, DraftDocument, DraftHow, ProposeDraft, SessionId, SessionRow, SessionVoice, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::draft_document::told;
use crate::session_host::draft_page::{page_of, slug_of};

const TITLE: &str = "Why slot leases end early";
const SLOT: u32 = 3;

/// The body of a page, between its `<h1>` and the end.
fn body(html: &str) -> &str {
    let (_, body) = html.split_once("</h1>\n").expect("a page with a heading");
    body.strip_suffix("</body>\n</html>\n").expect("a page that ends as one")
}

fn converted(text: &str) -> String {
    body(&page_of("T", text)).to_string()
}

#[test]
fn headings_paragraphs_and_rules_become_the_pages_blocks() {
    let html = converted("The conclusion, first.\nStill the first.\n\n# One\n## Two\n### Three\n###### Six\n\n---\n\n#NotAHeading");
    assert_eq!(
        html,
        "<p>The conclusion, first.\nStill the first.</p>\n<h2>One</h2>\n<h2>Two</h2>\n<h3>Three</h3>\n<h3>Six</h3>\n<hr>\n<p>#NotAHeading</p>\n"
    );
    assert_eq!(converted("# C#"), "<h2>C#</h2>\n");
    assert_eq!(converted("## Closed ##"), "<h2>Closed</h2>\n");
}

#[test]
fn lists_quotes_and_fenced_code_become_their_elements() {
    assert_eq!(converted("- one\n* two\n  more of two\n+ three"), "<ul>\n<li>one</li>\n<li>two\nmore of two</li>\n<li>three</li>\n</ul>\n");
    assert_eq!(converted("1. first\n2) second\n\n3. third"), "<ol>\n<li>first</li>\n<li>second</li>\n<li>third</li>\n</ol>\n");
    assert_eq!(converted("- a\n\nafter"), "<ul>\n<li>a</li>\n</ul>\n<p>after</p>\n");
    assert_eq!(converted("- a\n1. b"), "<ul>\n<li>a</li>\n</ul>\n<ol>\n<li>b</li>\n</ol>\n");
    assert_eq!(converted("> quoted\n> still\n>\n> - item"), "<blockquote>\n<p>quoted\nstill</p>\n<ul>\n<li>item</li>\n</ul>\n</blockquote>\n");
    assert_eq!(
        converted("```rust\nlet a = 1 < 2;\n\n# not a heading\n```\nafter"),
        "<pre><code>let a = 1 &lt; 2;\n\n# not a heading</code></pre>\n<p>after</p>\n"
    );
    // A fence that never closes holds the rest of the text.
    assert_eq!(converted("~~~\n- kept"), "<pre><code>- kept</code></pre>\n");
}

#[test]
fn code_emphasis_and_links_become_their_elements() {
    assert_eq!(converted("a `x < y` and ``tick ` inside`` b"), "<p>a <code>x &lt; y</code> and <code>tick ` inside</code> b</p>\n");
    assert_eq!(
        converted("**bold**, *em*, _em_ and __strong__ with **a *nested* one**"),
        "<p><strong>bold</strong>, <em>em</em>, <em>em</em> and <strong>strong</strong> with <strong>a <em>nested</em> one</strong></p>\n"
    );
    assert_eq!(
        converted("[the docs](https://example.com/a?b=1&c=2) and [here](#top) and [rel](../x.html) and [mail](mailto:a@b.c)"),
        "<p><a href=\"https://example.com/a?b=1&amp;c=2\">the docs</a> and <a href=\"#top\">here</a> and <a href=\"../x.html\">rel</a> and <a href=\"mailto:a@b.c\">mail</a></p>\n"
    );
    // Words that only look like markup stay words.
    assert_eq!(
        converted("snake_case_name, 2 * 3 * 4, *.rs and *.toml, a [bracket"),
        "<p>snake_case_name, 2 * 3 * 4, *.rs and *.toml, a [bracket</p>\n"
    );
    assert_eq!(converted(r"\*not em\* and \[not a link\]"), "<p>*not em* and [not a link]</p>\n");
}

#[test]
fn a_simple_table_becomes_a_table_and_a_pipe_in_a_paragraph_stays_a_paragraph() {
    assert_eq!(
        converted("| Name | Held |\n| :--- | ---: |\n| `a` | **yes** |\n| b |\n\nafter"),
        "<table>\n<thead><tr><th>Name</th><th>Held</th></tr></thead>\n<tbody>\n<tr><td><code>a</code></td><td><strong>yes</strong></td></tr>\n<tr><td>b</td></tr>\n</tbody>\n</table>\n<p>after</p>\n"
    );
    assert_eq!(converted("left | right\nnot a table"), "<p>left | right\nnot a table</p>\n");
}

#[test]
fn text_that_is_not_markdown_becomes_paragraphs() {
    assert_eq!(converted("It rained.\nWe stayed in.\n\n\n\nThen it stopped."), "<p>It rained.\nWe stayed in.</p>\n<p>Then it stopped.</p>\n");
    assert_eq!(converted(""), "");
    assert_eq!(converted("  \n\n"), "");
}

#[test]
fn everything_is_escaped_and_no_script_style_or_image_is_let_through() {
    let html = page_of(
        "<script>alert(\"t\")</script> & 'co'",
        "<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n![alt <b>](http://x/y.png)\n\n[run](javascript:alert(1)) [data](data:text/html;base64,AA==) [odd](ftp://h/f) [empty]()\n\n<style>p{}</style>\n\n## <i>head</i>\n\n- <b>item</b>\n\n| <u> |\n| --- |\n| <s> |",
    );
    for raw in ["<script", "<img", "<style", "<b>", "<i>", "<u>", "<s>", "javascript:", "data:", "ftp:", "onerror=alert(1)>"] {
        assert!(!html.contains(raw), "{raw} stays out of the page: {html}");
    }
    assert!(html.contains("<title>&lt;script&gt;alert(&quot;t&quot;)&lt;/script&gt; &amp; &#39;co&#39;</title>"), "{html}");
    assert!(html.contains("<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>"), "{html}");
    assert!(html.contains("<p>alt &lt;b&gt;</p>"), "an image is its alt text: {html}");
    assert!(html.contains("<p>run data odd empty</p>"), "{html}");
    assert!(html.contains("<h2>&lt;i&gt;head&lt;/i&gt;</h2>"), "{html}");
    assert_eq!(html.matches("<h1>").count(), 1);
}

#[test]
fn a_page_has_the_documents_skeleton_and_a_leading_heading_that_repeats_the_title_is_left_out() {
    let html = page_of("  Why slot   leases\tend ", "# why slot leases end\n\nThe conclusion.\n\n## Measured\n\nText.");
    assert_eq!(
        html,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Why slot leases end</title>\n<link rel=\"stylesheet\" href=\"armada-kit.css\">\n</head>\n<body>\n<h1>Why slot leases end</h1>\n<p>The conclusion.</p>\n<h2>Measured</h2>\n<p>Text.</p>\n</body>\n</html>\n"
    );
    // Fleet reads the title and the heading back as the draft's.
    assert_eq!(crate::session_host::document_page::title_of(&page_of("A & B", "x"), "/r/.armada/documents/a.html"), "A & B");
    // A different first heading is a section.
    assert!(body(&page_of("T", "# Other\n\ntext")).starts_with("<h2>Other</h2>"));
}

#[test]
fn a_title_makes_a_slug_of_letters_and_digits() {
    assert_eq!(slug_of(TITLE), "why-slot-leases-end-early");
    assert_eq!(slug_of("  PR #41: the *new* description!! "), "pr-41-the-new-description");
    assert_eq!(slug_of("---"), "draft");
    assert_eq!(slug_of("Ünïcode only: éé"), "n-code-only");
    let long = slug_of(&"word ".repeat(30));
    assert!(long.len() <= 60 && !long.ends_with('-'), "{long}");
}

fn proposed(id: &SessionId, title: &str, text: &str) -> ProposeDraft {
    ProposeDraft { title: title.to_string(), text: text.to_string(), before: None, pull_request: None, session_id: Some(id.clone()), ..Default::default() }
}

/// A Job holding slot `SLOT`, whose tree is made.
async fn in_a_worktree(rig: &Rig) -> (SessionId, PathBuf) {
    let id = rig.start().await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = Some(SLOT);
    hosting.lease_branch = Some("armada/a-draft".to_string());
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let tree = PathBuf::from(adapter_traits::slot_path(&rig.root, SLOT));
    std::fs::create_dir_all(&tree).expect("a tree");
    (id, tree)
}

/// The item id of the draft just proposed, the newest of the Job's own drafts.
async fn draft_in(rig: &Rig, id: &SessionId, title: &str, text: &str) -> String {
    rig.fleet.propose_draft(None, proposed(id, title, text)).await.unwrap();
    let row = rig
        .rows(id)
        .await
        .into_iter()
        .rev()
        .find_map(|row| match row {
            SessionRow::Draft { id, change: None, .. } => Some(id),
            _ => None,
        })
        .expect("the draft's row");
    format!("draft:{row}")
}

fn convert(id: &SessionId, draft: &str) -> ConvertDraftToDocument {
    ConvertDraftToDocument { session_id: id.clone(), draft_id: draft.to_string() }
}

async fn made(rig: &Rig, id: &SessionId, draft: &str) -> DraftDocument {
    Arc::clone(&rig.fleet).convert_draft_to_document(convert(id, draft)).await.expect("converted")
}

fn settled_how(rows: &[SessionRow]) -> Option<DraftHow> {
    rows.iter().find_map(|row| match row {
        SessionRow::Draft { settled, .. } => settled.as_ref().map(|one| one.how),
        _ => None,
    })
}

fn fleet_said(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::Session { id, .. }, text, .. } if id == "fleet" => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn documents_of(tree: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(tree.join(".armada/documents"))
        .map(|read| read.flatten().map(|one| one.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

#[tokio::test]
async fn converting_a_draft_writes_its_page_withdraws_it_registers_the_document_and_tells_the_job() {
    let rig = rig_placing(None);
    let (id, tree) = in_a_worktree(&rig).await;
    let text = "The leases end because the clock starts at hand-over.\n\n## Measured\n\n- one\n- two\n";
    let item = draft_in(&rig, &id, TITLE, text).await;

    let page = made(&rig, &id, &item).await;

    // The page is where the name and the path say, and is the draft's text as HTML.
    let file = tree.join(".armada/documents/why-slot-leases-end-early.html");
    assert_eq!((page.name.as_str(), page.path.as_str(), page.told), ("why-slot-leases-end-early", file.to_str().unwrap(), true));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), page_of(TITLE, text));

    // The draft is withdrawn as `withdraw_draft` withdraws it, and leaves Needs you.
    let rows = rig.rows(&id).await;
    assert_eq!(settled_how(&rows), Some(DraftHow::Withdrawn));
    let record = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.into_iter().find(|one| one.id == id).expect("listed");
    assert!(record.waiting_for.iter().all(|one| one.source != WaitingSource::Draft), "{:?}", record.waiting_for);

    // The page is kept as the Job's own write is: one artifact, one revision, titled as the draft is.
    let path = page.path.clone();
    eventually(|| async {
        let record = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.into_iter().find(|one| one.id == id).expect("listed");
        record.attachments.iter().any(|one| {
            one.kind == "artifact" && one.target == path && one.detail.get("form").map(String::as_str) == Some("document")
                && one.detail.get("title").map(String::as_str) == Some(TITLE)
        })
    })
    .await;

    // The Job is told, in Fleet's words, where the page is and not to propose it again.
    assert_eq!(
        fleet_said(&rows).pop().unwrap(),
        format!(
            "The owner turned your draft \"{TITLE}\" into the Document .armada/documents/why-slot-leases-end-early.html. \
             Revise it there as a Document from now on, and don't propose it as a draft again."
        )
    );
    assert_eq!(told(TITLE, "why-slot-leases-end-early"), fleet_said(&rows).pop().unwrap());
}

#[tokio::test]
async fn a_name_that_is_taken_takes_the_next_number_and_no_page_is_overwritten() {
    let rig = rig_placing(None);
    let (id, tree) = in_a_worktree(&rig).await;
    let folder = tree.join(".armada/documents");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("why-slot-leases-end-early.html"), "the agent's own page").unwrap();

    let first = draft_in(&rig, &id, TITLE, "First text.").await;
    let one = made(&rig, &id, first.strip_prefix("draft:").unwrap()).await;
    let second = draft_in(&rig, &id, TITLE, "Second text.").await;
    let two = made(&rig, &id, &second).await;
    let third = draft_in(&rig, &id, TITLE, "Third text.").await;
    let three = made(&rig, &id, &third).await;

    assert_eq!((one.name.as_str(), two.name.as_str(), three.name.as_str()), ("why-slot-leases-end-early-2", "why-slot-leases-end-early-3", "why-slot-leases-end-early-4"));
    assert_eq!(std::fs::read_to_string(folder.join("why-slot-leases-end-early.html")).unwrap(), "the agent's own page");
    assert!(std::fs::read_to_string(&two.path).unwrap().contains("Second text."));
    assert_eq!(documents_of(&tree).len(), 4);
    assert!(fleet_said(&rig.rows(&id).await).last().unwrap().contains("why-slot-leases-end-early-4.html"));
}

#[tokio::test]
async fn an_unknown_a_settled_or_a_proposed_draft_is_refused_and_nothing_is_written_or_told() {
    let rig = rig_placing(None);
    let (id, tree) = in_a_worktree(&rig).await;
    let (other, _) = in_a_worktree(&rig).await;
    let item = draft_in(&rig, &id, TITLE, "Text.").await;
    let told_before = fleet_said(&rig.rows(&id).await).len();

    for (who, draft) in [(&id, "draft:nothing"), (&other, item.as_str())] {
        let Err(Refusal::NotFound(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(who, draft)).await else {
            panic!("a draft the job has no row for is a 404: {draft}")
        };
        assert_eq!(why.code, "fleet.draft_convert_unknown");
    }

    // A job Fleet does not host.
    let Err(Refusal::Unacceptable(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&SessionId::carried("nobody"), &item)).await else {
        panic!("an unknown job is a 422")
    };
    assert_eq!(why.code, "fleet.no_such_session");

    // A settled draft, whichever way it settled.
    let approve = AnswerWaiting { session_id: id.clone(), item_id: item.clone(), choice: Some(0), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(approve).await.unwrap();
    let Err(Refusal::IllegalMove(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&id, &item)).await else {
        panic!("an approved draft is a 409")
    };
    assert_eq!(why.code, "fleet.draft_convert_settled");
    let withdrawn = draft_in(&rig, &id, "Another", "Text.").await;
    rig.fleet.withdraw_draft(None, ipc::WithdrawDraft { draft: withdrawn.clone(), session_id: Some(id.clone()) }).await.unwrap();
    let Err(Refusal::IllegalMove(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&id, &withdrawn)).await else {
        panic!("a withdrawn draft is a 409")
    };
    assert_eq!(why.code, "fleet.draft_convert_settled");

    // A draft that is the say on a change has no page to be.
    rig.fleet.proposal_waits(id.as_str(), 7, "ship-it", "steps: []", None).await;
    let proposal = rig
        .rows(&id)
        .await
        .into_iter()
        .find_map(|row| match row {
            SessionRow::Draft { id, change: Some(_), .. } => Some(id),
            _ => None,
        })
        .expect("the proposal's row");
    let Err(Refusal::Unacceptable(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&id, &proposal)).await else {
        panic!("a proposal's draft is a 422")
    };
    assert_eq!(why.code, "fleet.draft_convert_proposal");

    assert!(documents_of(&tree).is_empty(), "nothing was written");
    assert_eq!(fleet_said(&rig.rows(&id).await).len(), told_before, "the job was told nothing");
}

#[tokio::test]
async fn a_job_with_no_worktree_is_refused_and_its_draft_stays_open() {
    let rig = rig_placing(None);
    // A Job that has leased no slot.
    let id = rig.start().await;
    let item = draft_in(&rig, &id, TITLE, "Text.").await;
    let Err(Refusal::IllegalMove(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&id, &item)).await else {
        panic!("no slot is a 409")
    };
    assert_eq!(why.code, "fleet.draft_convert_no_worktree");
    assert!(why.message.contains(id.as_str()), "{}", why.message);

    // A slot whose tree is gone is the same, and the folder is not made in its place.
    let (gone, tree) = in_a_worktree(&rig).await;
    std::fs::remove_dir_all(&tree).unwrap();
    let kept = draft_in(&rig, &gone, TITLE, "Text.").await;
    let Err(Refusal::IllegalMove(why)) = Arc::clone(&rig.fleet).convert_draft_to_document(convert(&gone, &kept)).await else {
        panic!("a missing tree is a 409")
    };
    assert_eq!(why.code, "fleet.draft_convert_no_worktree");
    assert!(!tree.exists());

    for job in [&id, &gone] {
        assert!(settled_how(&rig.rows(job).await).is_none(), "the draft is still open");
        assert!(fleet_said(&rig.rows(job).await).is_empty());
    }
    // The open draft still waits.
    let record = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions.into_iter().find(|one| one.id == id).expect("listed");
    assert!(record.waiting_for.iter().any(|one| one.source == WaitingSource::Draft));
}

/// A pause hands the slot back, so a Job that holds a worktree and is paused is one a pause landed on
/// as the press did, or one whose marker is set by hand here.
#[tokio::test]
async fn a_paused_job_is_converted_and_its_turn_is_kept_and_delivered_on_resume() {
    let rig = rig_placing(None);
    let (id, tree) = in_a_worktree(&rig).await;
    let item = draft_in(&rig, &id, TITLE, "Text.").await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.paused_at = Some(rig.fleet.now().as_str().to_string());
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let starts = rig.stand_in.starts().len();

    let page = made(&rig, &id, &item).await;

    // Converted as for any Job, and not told yet: the word is on the thread and kept for the Resume.
    assert!(!page.told);
    assert!(tree.join(".armada/documents/why-slot-leases-end-early.html").is_file());
    let rows = rig.rows(&id).await;
    assert_eq!(settled_how(&rows), Some(DraftHow::Withdrawn));
    let sentence = told(TITLE, &page.name);
    assert_eq!(fleet_said(&rows), vec![sentence.clone()], "on the thread as Fleet's");
    assert_eq!(rig.fleet.store().lock().await.held_later_turns(id.as_str()).unwrap().len(), 1, "kept for the resume");
    assert_eq!(rig.stand_in.starts().len(), starts, "nothing woke the job");

    // The Job holds a slot again: the kept turn goes as its next turn, and is not written to the thread twice.
    hosting.paused_at = None;
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("resumed");
    rig.fleet.pause_lifted(id.as_str()).await;
    eventually(|| async { rig.stand_in.starts().len() == starts + 1 }).await;
    // The process is written JSON, so the quotes of the title are escaped there: the rest is read.
    let revise = "into the Document .armada/documents/why-slot-leases-end-early.html. Revise it there as a Document from now on";
    eventually(|| async { rig.stand_in.process(starts).sent.lock().unwrap().join("\n").contains(revise) }).await;
    assert_eq!(rig.fleet.store().lock().await.held_later_turns(id.as_str()).unwrap().len(), 0, "taken");
    assert_eq!(fleet_said(&rig.rows(&id).await), vec![sentence]);
}
