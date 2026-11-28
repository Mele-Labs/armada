//! The librarian's `gists` part, written before the code: **the model writes a gist only where none exists.** A Job's own
//! words and a doc's first sentence say it for free, so a linked doc that is a file of the repository and has neither is
//! what is sent, up to ten in a call, each call spent from the night's share. Each test lays a repository out under the
//! rig's root. `docs/concepts/feature-catalog.md`, *Librarian*.

use api::Features;
use ipc::{LibrarianPart, LibrarianStepState, SettingValue};
use store::{feature_link_kind, KeptFeatureLink};
use testkit::FakeJudge;

use super::doc_drift::{commit, git, write};
use super::librarian::{libraried, pressed, rig_judged, rig_saying, set, step, steps_listed, steps_told};
use super::session_host::Rig;

/// A repository at the rig's root holding `docs/<name>.md` for each of `names`.
fn repository_of(rig: &Rig, names: &[&str]) {
    let root = rig.root.as_str();
    git(root, &["init", "-q", "-b", "main"]);
    let paths: Vec<String> = names.iter().map(|name| format!("docs/{name}.md")).collect();
    for (path, name) in paths.iter().zip(names) {
        write(root, path, &format!("# {name}\n\nThe opening of {name}.\n"));
    }
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    commit(root, &paths, "the docs");
}

/// A read link from `feature` to the doc `target`, made by `job` at minute `minute` of the day, saying `gist` by `by`.
async fn linked(rig: &Rig, feature: &str, target: &str, job: &str, minute: u32, gist: &str, by: &str) {
    let link = KeptFeatureLink {
        feature_id: feature.to_string(),
        kind: feature_link_kind::DOC.to_string(),
        target: target.to_string(),
        job: job.to_string(),
        title: String::new(),
        gist: gist.to_string(),
        gist_by: by.to_string(),
        state: String::new(),
        via: "read".to_string(),
        at: format!("2026-11-19T{:02}:{:02}:00.000Z", minute / 60, minute % 60),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
}

async fn only_gists(rig: &Rig) {
    set(rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("gists".into())])).await;
}

/// The gist and who wrote it, as the store holds them on the doc's links, oldest first.
async fn kept(rig: &Rig, feature: &str, target: &str) -> Vec<(String, String)> {
    let links = rig.fleet.store().lock().await.feature_links(feature).expect("read");
    links.into_iter().filter(|link| link.target == target).map(|link| (link.gist, link.gist_by)).collect()
}

/// `count` numbered lines of gists, the first numbered 1, naming topics from `from` on.
fn numbered(from: usize, count: usize) -> String {
    (1..=count).map(|at| format!("{at}. Covers topic {}.", from + at)).collect::<Vec<_>>().join("\n")
}

/// A project with `count` linked docs, `d00` the oldest link, all in the repository, and only gists on.
async fn docs_linked(rig: &Rig, count: usize) -> (String, String) {
    let (project, feature) = libraried(rig).await;
    let names: Vec<String> = (0..count).map(|at| format!("d{at:02}")).collect();
    repository_of(rig, &names.iter().map(String::as_str).collect::<Vec<_>>());
    for (at, name) in names.iter().enumerate() {
        linked(rig, &feature, &format!("docs/{name}.md"), "j1", at as u32, "", "").await;
    }
    only_gists(rig).await;
    (project, feature)
}

#[tokio::test]
async fn a_linked_repo_doc_with_no_gist_gets_the_models_line_by_the_librarian_and_the_page_serves_it() {
    let (rig, judge) = rig_saying("1. Explains how takes are voted on.");
    let (project, feature) = docs_linked(&rig, 1).await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [LibrarianPart::Gists]);
    assert!(ran.skipped_for_share.is_empty());
    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("1. Path: docs/d00.md") && asked[0].contains("The opening of d00."), "{}", asked[0]);
    assert!(asked[0].contains("at most 20 words") && asked[0].contains("No dashes"), "{}", asked[0]);

    assert_eq!(kept(&rig, &feature, "docs/d00.md").await, [("Explains how takes are voted on.".to_string(), "librarian".to_string())]);
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), "voting-on-takes".to_string(), None, false).await.expect("the page");
    assert_eq!((page.docs[0].gist.as_str(), page.docs[0].gist_by_job), ("Explains how takes are voted on.", false));
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("write_gists", LibrarianStepState::Done, "Wrote 1 gist, 1 of 20 calls tonight"));
}

#[tokio::test]
async fn a_doc_with_a_job_gist_or_a_first_sentence_is_not_sent() {
    let (rig, judge) = rig_saying("1. Covers the bare one.");
    let (project, feature) = libraried(&rig).await;
    repository_of(&rig, &["said", "first", "bare"]);
    linked(&rig, &feature, "docs/said.md", "j1", 1, "A Job's words.", "job").await;
    linked(&rig, &feature, "docs/first.md", "j1", 2, "The first sentence.", "").await;
    linked(&rig, &feature, "docs/bare.md", "j1", 3, "", "").await;
    only_gists(&rig).await;

    pressed(&rig, &project).await;
    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("docs/bare.md") && !asked[0].contains("docs/said.md") && !asked[0].contains("docs/first.md"), "{}", asked[0]);
    assert_eq!(kept(&rig, &feature, "docs/said.md").await, [("A Job's words.".to_string(), "job".to_string())]);
    assert_eq!(kept(&rig, &feature, "docs/first.md").await, [("The first sentence.".to_string(), String::new())]);
}

#[tokio::test]
async fn twelve_docs_cost_two_calls_the_oldest_link_first() {
    let (rig, judge) = rig_judged(FakeJudge::answering_in_turn(&[&numbered(0, 10), &numbered(10, 2)]));
    let (project, feature) = docs_linked(&rig, 12).await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    let asked = judge.asked();
    assert_eq!(asked.len(), 2);
    assert!(asked[0].contains("10. Path: docs/d09.md") && !asked[0].contains("docs/d10.md"), "{}", asked[0]);
    assert!(asked[1].contains("1. Path: docs/d10.md") && asked[1].contains("2. Path: docs/d11.md"), "{}", asked[1]);
    assert_eq!(kept(&rig, &feature, "docs/d11.md").await, [("Covers topic 12.".to_string(), "librarian".to_string())]);
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("write_gists", LibrarianStepState::Done, "Wrote 12 gists, 2 of 20 calls tonight"));
    let record = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert_eq!(record.model_calls, 2);
}

#[tokio::test]
async fn at_most_thirty_docs_go_in_a_run() {
    let (rig, judge) = rig_judged(FakeJudge::saying(&numbered(0, 10)));
    let (project, _) = docs_linked(&rig, 35).await;
    pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 3, "thirty docs, ten a call");
}

#[tokio::test]
async fn the_share_running_out_leaves_the_rest_and_the_run_lists_gists_as_skipped() {
    let (rig, judge) = rig_judged(FakeJudge::saying(&numbered(0, 10)));
    let (project, feature) = docs_linked(&rig, 12).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;

    let ran = pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "the night's one call");
    assert_eq!(ran.skipped_for_share, [LibrarianPart::Gists]);
    assert_eq!(kept(&rig, &feature, "docs/d09.md").await[0].1, "librarian");
    assert_eq!(kept(&rig, &feature, "docs/d10.md").await, [(String::new(), String::new())], "left as it was");
    let record = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert_eq!(record.skipped, ["gists"]);

    let again = pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "the share is spent, so nothing more is asked");
    assert_eq!(again.skipped_for_share, [LibrarianPart::Gists]);
}

#[tokio::test]
async fn a_share_of_none_skips_the_step_and_says_so() {
    let (rig, judge) = rig_saying("1. Covers it.");
    let (project, _) = docs_linked(&rig, 1).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(0)).await;
    let mut watching = rig.fleet.events().subscribe();
    pressed(&rig, &project).await;
    assert!(judge.asked().is_empty());
    assert_eq!(steps_told(&mut watching, &project).await[3], step("write_gists", LibrarianStepState::Skipped, "The share allows no calls"));
}

#[tokio::test]
async fn a_number_missing_from_the_reply_leaves_that_doc_as_it_was() {
    let (rig, _) = rig_saying("1. Covers the first.\n3. Covers the third.\nnot a line\n9. Past the end.");
    let (project, feature) = docs_linked(&rig, 3).await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    assert_eq!(kept(&rig, &feature, "docs/d00.md").await[0].0, "Covers the first.");
    assert_eq!(kept(&rig, &feature, "docs/d01.md").await, [(String::new(), String::new())]);
    assert_eq!(kept(&rig, &feature, "docs/d02.md").await[0].0, "Covers the third.");
    assert_eq!(steps_told(&mut watching, &project).await[3], step("write_gists", LibrarianStepState::Done, "Wrote 2 gists, 1 of 20 calls tonight"));
}

#[tokio::test]
async fn a_reply_with_no_readable_line_fails_the_step_and_keeps_nothing() {
    let (rig, _) = rig_saying("I cannot do that.");
    let (project, feature) = docs_linked(&rig, 1).await;
    let mut watching = rig.fleet.events().subscribe();
    let ran = pressed(&rig, &project).await;
    assert!(ran.ran.is_empty());
    assert_eq!(kept(&rig, &feature, "docs/d00.md").await, [(String::new(), String::new())]);
    assert_eq!(steps_told(&mut watching, &project).await[3], step("write_gists", LibrarianStepState::Failed, "The model wrote no gists, 1 of 20 calls tonight"));
}

#[tokio::test]
async fn the_line_is_kept_on_one_line_cut_to_two_hundred_characters_with_no_dash() {
    let long = "word ".repeat(80);
    let (rig, _) = rig_saying(&format!("1. \"Covers voting - and tallies\u{2014}both\"\n2. {long}"));
    let (project, feature) = docs_linked(&rig, 2).await;
    pressed(&rig, &project).await;
    assert_eq!(kept(&rig, &feature, "docs/d00.md").await[0].0, "Covers voting, and tallies,both");
    let cut = kept(&rig, &feature, "docs/d01.md").await[0].0.clone();
    assert!(cut.ends_with('…') && (199..=201).contains(&cut.chars().count()), "200 characters and the ellipsis: {cut}");
}

#[tokio::test]
async fn no_doc_lacking_a_gist_means_no_call_and_a_line_that_says_so() {
    let (rig, judge) = rig_saying("1. Never asked.");
    let (project, feature) = libraried(&rig).await;
    repository_of(&rig, &["said"]);
    linked(&rig, &feature, "docs/said.md", "j1", 1, "Has one.", "").await;
    linked(&rig, &feature, "https://example.com/spec", "j1", 2, "", "").await;
    only_gists(&rig).await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert!(judge.asked().is_empty());
    assert_eq!(ran.ran, [LibrarianPart::Gists], "the part ran and found nothing to do");
    assert_eq!(steps_told(&mut watching, &project).await[3], step("write_gists", LibrarianStepState::Done, "No linked doc lacks a gist"));
    assert_eq!(steps_listed(&rig, &project).await[1], ("write_gists".to_string(), true, "No linked doc lacks a gist".to_string()));
}

#[tokio::test]
async fn a_doc_that_is_not_a_file_of_the_repository_at_head_is_not_sent() {
    let (rig, judge) = rig_saying("1. Never asked.");
    let (project, feature) = libraried(&rig).await;
    repository_of(&rig, &["there"]);
    write(rig.root.as_str(), "docs/untracked.md", "Not committed.\n");
    linked(&rig, &feature, "docs/untracked.md", "j1", 1, "", "").await;
    linked(&rig, &feature, "docs/missing.md", "j1", 2, "", "").await;
    linked(&rig, &feature, "/nowhere/else.md", "j1", 3, "", "").await;
    only_gists(&rig).await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    assert!(judge.asked().is_empty());
    assert_eq!(steps_told(&mut watching, &project).await[3], step("write_gists", LibrarianStepState::Done, "No doc without a gist could be read"));
}

#[tokio::test]
async fn a_jobs_gist_written_later_replaces_the_librarians_and_a_later_first_sentence_does_not() {
    let (rig, judge) = rig_saying("1. The librarian's line.");
    let (project, feature) = docs_linked(&rig, 1).await;
    pressed(&rig, &project).await;

    // A first sentence heard in another Job, later, does not replace it.
    linked(&rig, &feature, "docs/d00.md", "j2", 600, "A first sentence.", "").await;
    let served = rig.fleet.get_feature_page(None, Some(project.clone()), "voting-on-takes".to_string(), None, false).await.expect("the page").docs;
    assert_eq!((served[0].gist.as_str(), served[0].gist_by_job), ("The librarian's line.", false));

    // A Job's own gist replaces it.
    assert_eq!(rig.fleet.store().lock().await.keep_feature_job_gist("j1", "docs/d00.md", "The Job's words.").expect("kept"), 1);
    let served = rig.fleet.get_feature_page(None, Some(project.clone()), "voting-on-takes".to_string(), None, false).await.expect("the page").docs;
    assert_eq!((served[0].gist.as_str(), served[0].gist_by_job), ("The Job's words.", true));

    // And the librarian does not write over it, nor ask about the doc again.
    pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1);
    assert_eq!(kept(&rig, &feature, "docs/d00.md").await[0], ("The Job's words.".to_string(), "job".to_string()));
}

#[tokio::test]
async fn the_step_appears_only_where_gists_is_on_after_every_other_part_and_says_how_many_calls() {
    let (rig, _) = rig_saying("1. Covers it.");
    let (project, _) = docs_linked(&rig, 12).await;

    let listed = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian.steps;
    let ids: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["read_merges", "write_gists", "read_through"]);
    assert_eq!((listed[1].title.as_str(), listed[1].detail.as_str(), listed[1].model), ("Write missing gists", "12 linked docs have no gist; about 2 model calls", true));

    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("floor_plans".into())])).await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["read_merges", "read_through"], "gists is off");

    let every = LibrarianPart::ALL.iter().rev().map(|part| SettingValue::Text(part.name().into())).collect();
    set(&rig, "features.librarianParts", SettingValue::List(every)).await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["read_merges", "check_upkeep", "check_drift", "since_you_looked", "take_pictures", "write_gists", "read_through"]);
}
