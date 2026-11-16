//! The claim, written before the code: **a vote a page declares with `<armada-vote>` takes his pick, his rank or
//! his keep and drop on each named part, read off the kept revision, and sends the Job the result with every
//! note. A result that does not fit the vote's takes is refused, the newest cast per vote is kept on the page, and
//! a vote that will not count is named to the Job when the page is kept.**
//! `crates/ipc/operations/cast_page_vote.toml`.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{HostedSessions, Refusal};
use ipc::{CastPageVote, SessionId, SessionRow, VoteCast, VoteForm, VoteMark, VoteResult};

use super::page_actions::{a_job, page_holding, sent};
use super::session_host::{eventually, rig_placing, Rig};

/// A page whose one vote of form `form` holds `takes`.
fn layouts(form: &str, takes: &str) -> String {
    format!(
        "<html><head><title>Layouts</title></head><body><h1>Layouts</h1>\
         <armada-vote id=\"layout\" form=\"{form}\" question=\"Which layout?\" parts=\"header,list,footer\">{takes}</armada-vote></body></html>"
    )
}

const THREE: &str = "<armada-take id=\"dense\" title=\"Dense\"><p>Tight rows</p></armada-take>\
     <armada-take id=\"roomy\" title=\"Roomy\"><p>Wide rows</p></armada-take>\
     <armada-take id=\"split\" title=\"Split\"><p>Two panes</p></armada-take>";

fn notes(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(take, words)| (take.to_string(), words.to_string())).collect()
}

fn casting(id: &SessionId, path: &str, result: VoteResult) -> CastPageVote {
    CastPageVote { session_id: id.clone(), artifact: path.into(), revision: 1, vote: "layout".into(), result }
}

async fn cast(rig: &Rig, vote: CastPageVote) -> Result<VoteCast, Refusal> {
    Arc::clone(&rig.fleet).cast_page_vote(vote).await
}

async fn votes(rig: &Rig, id: &SessionId, path: &str) -> Vec<VoteCast> {
    rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap().votes
}

#[tokio::test]
async fn a_pick_with_notes_sends_the_job_the_pick_and_every_note_and_is_kept() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &layouts("pick", THREE)).await;

    let result = VoteResult::Pick { take: "roomy".into(), notes: notes(&[("dense", "Too cramped"), ("split", "Keep the second pane")]) };
    let voted = cast(&rig, casting(&id, &path, result.clone())).await.expect("cast");

    assert_eq!((voted.form, &voted.result), (VoteForm::Pick, &result));
    let told = sent(&rig.rows(&id).await).pop();
    assert_eq!(
        told.as_deref(),
        Some("He voted on \"Which layout?\" in \"Layouts\".\nPicked: Roomy.\nDense: Too cramped\nSplit: Keep the second pane")
    );
    assert_eq!(votes(&rig, &id, &path).await, [voted]);
}

#[tokio::test]
async fn a_rank_that_skips_a_take_is_refused_and_keeps_nothing() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &layouts("rank", THREE)).await;
    let before = sent(&rig.rows(&id).await).len();

    let result = VoteResult::Rank { order: vec!["split".into(), "dense".into()], notes: BTreeMap::new() };
    let refused = cast(&rig, casting(&id, &path, result)).await.expect_err("refused");

    assert_eq!(refused.error().code, "fleet.page_vote_takes");
    assert!(refused.error().message.contains("skips roomy"), "{}", refused.error().message);
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "nothing sent");
    assert!(votes(&rig, &id, &path).await.is_empty());
}

#[tokio::test]
async fn a_score_lists_what_each_take_keeps_and_drops_and_leaves_a_bare_part_out() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &layouts("score", THREE)).await;
    let mark = |take: &str, part: &str, keep: bool| VoteMark { take: take.into(), part: part.into(), keep };

    // Dense's footer and all of Split's list are bare: no vote.
    let marks = vec![mark("dense", "header", true), mark("dense", "list", false), mark("roomy", "list", true), mark("roomy", "footer", true), mark("split", "header", false)];
    let result = VoteResult::Score { marks, notes: notes(&[("split", "Header too tall")]) };
    cast(&rig, casting(&id, &path, result)).await.expect("cast");

    let told = sent(&rig.rows(&id).await).pop();
    assert_eq!(
        told.as_deref(),
        Some(
            "He voted on \"Which layout?\" in \"Layouts\".\nDense: keeps header; drops list\nRoomy: keeps list, footer\nSplit: drops header\nSplit: Header too tall"
        )
    );
}

#[tokio::test]
async fn a_recast_replaces_the_earlier_result_on_the_page() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &layouts("pick", THREE)).await;

    cast(&rig, casting(&id, &path, VoteResult::Pick { take: "dense".into(), notes: BTreeMap::new() })).await.expect("first cast");
    let again = cast(&rig, casting(&id, &path, VoteResult::Pick { take: "split".into(), notes: BTreeMap::new() })).await.expect("recast");

    assert_eq!(votes(&rig, &id, &path).await, [again]);
    assert_eq!(sent(&rig.rows(&id).await).pop().as_deref(), Some("He voted on \"Which layout?\" in \"Layouts\".\nPicked: Split."));
}

#[tokio::test]
async fn a_vote_with_one_take_is_named_when_the_page_is_kept() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &layouts("pick", "<armada-take id=\"dense\" title=\"Dense\"></armada-take>")).await;

    let told = || async {
        rig.rows(&id).await.into_iter().find_map(|row| match row {
            SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" => Some(text),
            _ => None,
        })
    };
    eventually(|| async { told().await.is_some() }).await;
    let told = told().await.unwrap();
    let named = format!("In the page {path}, vote \"Which layout?\" will not count: it holds 1 take, and a vote needs two or more.");
    assert!(told.starts_with(&named), "{told}");
}
