//! Asking the librarian, written before the code: **one model call, at most one act, already done, and Undo takes it
//! back.** Asks spend from a night of their own. `docs/concepts/feature-catalog.md`, *Librarian*.

use std::sync::Arc;

use api::{Features, Projects};
use ipc::{
    AreaSeed, AskLibrarian, AskedBlockKind, AskedBoard, BoardBlock, BoardLayout, FeatureSeed, LibrarianAct, RankWithLibrarian, RefileLibrarianAsk, SaveProjectBoard, SettingValue,
    UndoLibrarianAsk, UpsertFeatures,
};
use store::feature_link_kind;
use testkit::{FakeJudge, FakeTracker, FakeTrackers};

use super::librarian::{libraried, rig_judged, set};
use super::peer::Placing;
use super::session_host::{rig_fitted, Rig};

fn block(kind: &str, x: u8, y: u8, w: u8, h: u8, view: &str) -> BoardBlock {
    BoardBlock { kind: kind.to_string(), x, y, w, h, view: view.to_string() }
}

/// A three-column board holding one `needs` block, that draws `needs` as a list or cards and `checks` as a table.
fn board(preset: bool) -> AskedBoard {
    AskedBoard {
        layout: BoardLayout { blocks: vec![block("needs", 0, 0, 2, 1, "list")] },
        preset,
        kinds: vec![
            AskedBlockKind { kind: "needs".into(), views: vec!["list".into(), "cards".into()] },
            AskedBlockKind { kind: "checks".into(), views: vec!["table".into()] },
        ],
        columns: 3,
    }
}

fn ask(project: &str, context: &str, words: &str, files: &[&str], board: Option<AskedBoard>) -> AskLibrarian {
    AskLibrarian { project: project.to_string(), context: context.to_string(), words: words.to_string(), files: files.iter().map(|one| one.to_string()).collect(), board }
}

async fn kept_board(rig: &Rig, project: &str) -> Option<BoardLayout> {
    rig.fleet.get_project_board(project.to_string()).await.expect("read").layout
}

const CHECKS_ADDED: &str = "ACT board\nREPLY Added a checks block.\nBLOCK needs 0 0 2 1 list\nBLOCK checks 2 0 1 1 table";

#[tokio::test]
async fn a_board_ask_keeps_the_board_and_undo_puts_back_the_one_kept_before() {
    let (rig, judge) = rig_judged(FakeJudge::saying(CHECKS_ADDED));
    let (project, _) = libraried(&rig).await;
    let before = board(false).layout;
    rig.fleet.save_project_board(project.clone(), SaveProjectBoard { layout: Some(before.clone()) }).await.expect("kept");

    let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Add the checks", &[], Some(board(false)))).await.expect("answered");
    let wanted = BoardLayout { blocks: vec![block("needs", 0, 0, 2, 1, "list"), block("checks", 2, 0, 1, 1, "table")] };
    assert_eq!(answered.act, Some(LibrarianAct::Board { layout: wanted.clone() }));
    assert_eq!(answered.reply, "Added a checks block.");
    assert_eq!((answered.calls, answered.limit), (1, 40), "one call of the night's forty");
    assert_eq!(judge.asked().len(), 1, "one model call");
    assert_eq!(kept_board(&rig, &project).await, Some(wanted));

    let undone = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project: project.clone(), asked_at: answered.asked_at.clone() }).await.expect("undone");
    assert_eq!(undone.reply, "The board is back as it was.");
    assert_eq!(kept_board(&rig, &project).await, Some(before));
    let again = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project, asked_at: answered.asked_at }).await.expect_err("undone once");
    assert_eq!(again.error().code, "fleet.librarian_ask_unknown");
}

#[tokio::test]
async fn undo_of_a_board_ask_on_the_preset_forgets_the_kept_board() {
    let (rig, _) = rig_judged(FakeJudge::saying(CHECKS_ADDED));
    let (project, _) = libraried(&rig).await;
    assert_eq!(kept_board(&rig, &project).await, None, "the preset is drawn");
    let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Add the checks", &[], Some(board(true)))).await.expect("answered");
    assert!(kept_board(&rig, &project).await.is_some(), "the ask kept a board");
    let undone = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project: project.clone(), asked_at: answered.asked_at }).await.expect("undone");
    assert_eq!(undone.reply, "The board is the preset again.");
    assert_eq!(kept_board(&rig, &project).await, None, "forgotten, so the preset draws again");
}

#[tokio::test]
async fn a_layout_that_does_not_fit_the_board_is_not_kept_and_the_ask_does_nothing() {
    let wrong = [
        "ACT board\nREPLY Done.\nBLOCK needs 0 0 2 1 list\nBLOCK checks 1 0 2 1 table",
        "ACT board\nREPLY Done.\nBLOCK needs 0 0 2 1 list\nBLOCK checks 2 0 2 1 table",
        "ACT board\nREPLY Done.\nBLOCK needs 0 0 2 1 list\nBLOCK reviews 2 0 1 1 list",
        "ACT board\nREPLY Done.\nBLOCK needs 0 0 2 1 graph",
        "ACT board\nREPLY Done.\nBLOCK needs 0 0 0 1 list",
        "ACT board\nREPLY Done.\nBLOCK needs zero 0 1 1 list",
        "ACT board\nREPLY Done.",
    ];
    let (rig, judge) = rig_judged(FakeJudge::answering_in_turn(&wrong));
    let (project, _) = libraried(&rig).await;
    for (at, reply) in wrong.iter().enumerate() {
        let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Change the board", &[], Some(board(true)))).await.expect("answered");
        assert_eq!(answered.act, None, "{reply}");
        assert_eq!(answered.reply, "The board was not changed. The librarian's layout did not fit the board.");
        assert_eq!(answered.calls, u32::try_from(at + 1).unwrap(), "a call is spent all the same");
        let refused = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project: project.clone(), asked_at: answered.asked_at }).await.expect_err("nothing to undo");
        assert_eq!(refused.error().code, "fleet.librarian_ask_unknown");
    }
    assert_eq!(judge.asked().len(), wrong.len());
    assert_eq!(kept_board(&rig, &project).await, None, "nothing was kept");
}

/// The project's second feature, `ballots`, and its id.
async fn ballots(rig: &Rig, project: &str) -> String {
    let upsert = UpsertFeatures {
        project: project.to_string(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![FeatureSeed {
            slug: "ballots".into(),
            area: "takes".into(),
            title: "Ballots".into(),
            summary: "One line.".into(),
            state: None,
            paths: vec!["crates/pages/ballots".into()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    rig.fleet.store().lock().await.feature_named(project, "ballots").expect("read").expect("held").id
}

async fn docs_of(rig: &Rig, feature: &str) -> Vec<String> {
    let links = rig.fleet.store().lock().await.feature_links(feature).expect("read");
    links.into_iter().filter(|link| link.kind == feature_link_kind::DOC).map(|link| link.target).collect()
}

#[tokio::test]
async fn a_file_ask_files_the_doc_under_a_feature_move_refiles_it_and_undo_drops_it() {
    let (rig, _) = rig_judged(FakeJudge::saying("ACT file\nREPLY Filed the guide under Voting on takes.\nFILE docs/guide.md | The guide | voting-on-takes"));
    let (project, voting) = libraried(&rig).await;
    let ballots = ballots(&rig, &project).await;

    let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "File this", &["docs/guide.md"], Some(board(false)))).await.expect("answered");
    let filed = LibrarianAct::FiledDoc { target: "docs/guide.md".into(), title: "The guide".into(), feature: "voting-on-takes".into(), feature_title: "Voting on takes".into() };
    assert_eq!(answered.act, Some(filed));
    assert_eq!(docs_of(&rig, &voting).await, ["docs/guide.md"], "on the feature's page at once");

    let refused = rig.fleet.refile_librarian_ask(None, RefileLibrarianAsk { project: project.clone(), asked_at: answered.asked_at.clone(), feature: "nowhere".into() }).await.expect_err("no such feature");
    assert_eq!(refused.error().code, "fleet.feature_unknown");
    let moved = rig.fleet.refile_librarian_ask(None, RefileLibrarianAsk { project: project.clone(), asked_at: answered.asked_at.clone(), feature: "ballots".into() }).await.expect("moved");
    assert_eq!(moved.reply, "Filed The guide under Ballots.");
    assert_eq!(moved.calls, 1, "a move spends no call");
    assert!(docs_of(&rig, &voting).await.is_empty());
    assert_eq!(docs_of(&rig, &ballots).await, ["docs/guide.md"]);

    let undone = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project: project.clone(), asked_at: answered.asked_at.clone() }).await.expect("undone");
    assert_eq!(undone.reply, "The guide is no longer filed under Ballots.");
    assert!(docs_of(&rig, &ballots).await.is_empty(), "dropped from where Move put it");
    let refused = rig.fleet.refile_librarian_ask(None, RefileLibrarianAsk { project, asked_at: answered.asked_at, feature: "ballots".into() }).await.expect_err("undone");
    assert_eq!(refused.error().code, "fleet.librarian_ask_unknown");
}

#[tokio::test]
async fn on_a_features_page_a_file_goes_under_that_feature_and_a_file_he_did_not_give_is_not_filed() {
    let (rig, _) = rig_judged(FakeJudge::answering_in_turn(&["ACT file\nREPLY Filed.\nFILE docs/guide.md | Guide | ", "ACT file\nREPLY Filed.\nFILE docs/other.md | Other | ballots"]));
    let (project, voting) = libraried(&rig).await;
    let ballots = ballots(&rig, &project).await;
    let answered = rig.fleet.ask_librarian(None, ask(&project, "feature:voting-on-takes", "Keep this here", &["docs/guide.md"], None)).await.expect("answered");
    assert!(matches!(answered.act, Some(LibrarianAct::FiledDoc { ref feature, .. }) if feature == "voting-on-takes"));
    assert_eq!(docs_of(&rig, &voting).await, ["docs/guide.md"]);
    let invented = rig.fleet.ask_librarian(None, ask(&project, "feature:voting-on-takes", "Keep this here", &["docs/guide.md"], None)).await.expect("answered");
    assert_eq!(invented.act, None, "docs/other.md is none of his");
    assert!(docs_of(&rig, &ballots).await.is_empty());
}

/// A rig whose tracker lists three issues and whose small model answers `replies` in turn, and a project of it.
async fn ranking_rig(replies: &[&str]) -> (Rig, Arc<FakeJudge>, String) {
    let judge = Arc::new(FakeJudge::answering_in_turn(replies));
    let planted = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| {
        fittings.peers = Placing::nothing();
        fittings.judge = planted;
        let issues = vec![FakeTracker::issue(1, "Nothing alike"), FakeTracker::issue(2, "Search is slow"), FakeTracker::issue(3, "Cart totals")];
        fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::new(FakeTracker::listing(issues))));
    });
    let (project, _) = libraried(&rig).await;
    (rig, judge, project)
}

fn order(ranked: &ipc::RankedIssues) -> Vec<(&str, Option<&str>)> {
    ranked.might_pick_up.iter().map(|row| (row.issue.reference.as_str(), row.why.as_deref())).collect()
}

#[tokio::test]
async fn rank_with_the_librarian_puts_its_rows_first_with_a_why_and_reset_goes_back_to_features() {
    let (rig, judge, project) = ranking_rig(&["I3 Cart totals are what your Jobs touched\nI2: Search is slow for everyone"]).await;
    let by_features = rig.fleet.list_ranked_issues(project.clone()).await.expect("ranked");
    assert_eq!(by_features.by_librarian, None);
    let keys: Vec<&str> = by_features.might_pick_up.iter().map(|row| row.issue.reference.as_str()).collect();
    assert_eq!(keys.len(), 3);

    let ranked = rig.fleet.rank_with_librarian(None, RankWithLibrarian { project: project.clone() }).await.expect("ranked");
    let third = by_features.might_pick_up[2].issue.reference.as_str();
    let second = by_features.might_pick_up[1].issue.reference.as_str();
    let first = by_features.might_pick_up[0].issue.reference.as_str();
    assert_eq!(
        order(&ranked),
        [(third, Some("Cart totals are what your Jobs touched")), (second, Some("Search is slow for everyone")), (first, None)],
        "the rows it named first, the rest after in feature order"
    );
    assert!(ranked.by_librarian.is_some());
    assert_eq!(judge.asked().len(), 1);
    assert_eq!(rig.fleet.list_ranked_issues(project.clone()).await.expect("ranked"), ranked, "held for the listing");

    let reset = rig.fleet.forget_librarian_ranking(None, RankWithLibrarian { project: project.clone() }).await.expect("forgotten");
    assert_eq!(reset, by_features, "back to features, no why");
}

#[tokio::test]
async fn an_ask_that_ranks_spends_its_call_and_the_rank_call_and_undo_forgets_the_ranking() {
    let (rig, judge, project) = ranking_rig(&["ACT rank\nREPLY Ranked by your recent work.", "I2 Most recent"]).await;
    let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Rank what I might pick up", &[], Some(board(true)))).await.expect("answered");
    assert_eq!(answered.act, Some(LibrarianAct::Ranked));
    assert_eq!(answered.calls, 2, "the ask and the ranking");
    assert_eq!(judge.asked().len(), 2);
    assert!(rig.fleet.list_ranked_issues(project.clone()).await.expect("ranked").by_librarian.is_some());
    let undone = rig.fleet.undo_librarian_ask(None, UndoLibrarianAsk { project: project.clone(), asked_at: answered.asked_at }).await.expect("undone");
    assert_eq!(undone.reply, "The lists are back in feature order.");
    let after = rig.fleet.list_ranked_issues(project).await.expect("ranked");
    assert_eq!(after.by_librarian, None);
    assert!(after.might_pick_up.iter().all(|row| row.why.is_none()));
}

#[tokio::test]
async fn an_answer_does_nothing_and_a_screen_that_answers_only_reads_any_act_as_an_answer() {
    let (rig, _) = rig_judged(FakeJudge::answering_in_turn(&["ACT answer\nREPLY Voting on takes is the one feature.", "ACT board\nREPLY Added a block.\nBLOCK needs 0 0 1 1 list"]));
    let (project, _) = libraried(&rig).await;
    let answered = rig.fleet.ask_librarian(None, ask(&project, "somewhere new", "What is here?", &[], None)).await.expect("answered");
    assert_eq!((answered.act, answered.reply.as_str()), (None, "Voting on takes is the one feature."));
    let empty = rig.fleet.ask_librarian(None, ask(&project, "features_empty", "Add a block", &[], Some(board(true)))).await.expect("answered");
    assert_eq!((empty.act, empty.reply.as_str()), (None, "Added a block."), "the empty Features page answers only");
    assert_eq!(kept_board(&rig, &project).await, None);
}

#[tokio::test]
async fn a_model_that_fails_still_spends_its_call() {
    let (rig, _) = rig_judged(FakeJudge::that_fails("the network"));
    let (project, _) = libraried(&rig).await;
    let answered = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Anything?", &[], None)).await.expect("answered");
    assert_eq!((answered.act, answered.reply.as_str(), answered.calls), (None, "The librarian did not answer.", 1));
}

#[tokio::test]
async fn a_blank_ask_a_job_and_a_project_fleet_lacks_are_refused_and_nothing_is_asked() {
    let (rig, judge) = rig_judged(FakeJudge::saying("ACT answer\nREPLY Yes."));
    let (project, _) = libraried(&rig).await;
    let refused = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "  \n", &[" "], None)).await.expect_err("blank");
    assert_eq!(refused.error().code, "fleet.librarian_ask_blank");
    let refused = rig.fleet.ask_librarian(None, ask("nobody", "dashboard", "Hello", &[], None)).await.expect_err("no such project");
    assert_eq!(refused.error().code, "fleet.feature_project");
    let job = Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()));
    let refused = rig.fleet.ask_librarian(job, ask(&project, "dashboard", "Hello", &[], None)).await.expect_err("a Job's agent does not ask");
    assert_eq!(refused.error().code, "fleet.not_the_operator");
    assert!(judge.asked().is_empty());
    let files_only = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "", &["docs/guide.md"], None)).await;
    assert!(files_only.is_ok(), "a file alone is an ask");
}

#[tokio::test]
async fn the_nights_forty_asks_spent_the_next_is_refused_until_midnight() {
    let (rig, judge) = rig_judged(FakeJudge::saying("ACT answer\nREPLY Yes."));
    let (project, _) = libraried(&rig).await;
    assert_eq!(rig.fleet.librarian_set().ask_share, 40, "as it ships");
    let night = rig.fleet.librarian_night().await;
    {
        let mut store = rig.fleet.store().lock().await;
        for _ in 0..39 {
            store.spend_librarian_ask_call(&project, &night).expect("spent");
        }
    }
    let fortieth = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "One more", &[], None)).await.expect("the fortieth is asked");
    assert_eq!((fortieth.calls, fortieth.limit), (40, 40));
    let refused = rig.fleet.ask_librarian(None, ask(&project, "dashboard", "And another", &[], None)).await.expect_err("spent");
    assert_eq!(refused.error().code, "fleet.librarian_asks_spent");
    assert!(refused.error().message.contains("40") && refused.error().message.contains("midnight"), "{}", refused.error().message);
    let refused = rig.fleet.rank_with_librarian(None, RankWithLibrarian { project: project.clone() }).await.expect_err("a rank spends from the same night");
    assert_eq!(refused.error().code, "fleet.librarian_asks_spent");
    assert_eq!(judge.asked().len(), 1);
    assert_eq!(rig.fleet.store().lock().await.librarian_model_calls(&project, &night).expect("read"), 0, "Catch up's share is untouched");

    set(&rig, "features.librarianAskShare", SettingValue::Integer(41)).await;
    assert!(rig.fleet.ask_librarian(None, ask(&project, "dashboard", "Again", &[], None)).await.is_ok(), "a higher limit is read at once");
}
