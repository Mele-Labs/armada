//! Who is on what, written before the code: **the open pull requests of the Reviews lists are kept with their authors and
//! files, read from the forge once a head, mapped to features, and read beside the owner's running Jobs; a Job handed a
//! feature is told of a teammate's open pull request on it.** Each test reads the lists through a real Fleet and a
//! scripted forge, which records every read of a pull request's files. `docs/concepts/feature-catalog.md`, *Who's on what*.

use adapter_traits::{ListedPull, PatchedFile, ReviewList};
use api::Features;
use ipc::{AreaSeed, FeatureSeed, UpsertFeatures};
use store::KeptOpenPull;
use testkit::Delivered;

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, hand, held_slot, seeded, wrote, VOTING};
use super::pull_review_lists::{pull, SLUG};

const RANKS: &str = "crates/pages/voting/ranks.rs";
const ACTIONS: &str = "crates/pages/actions/menu.rs";

fn listed(number: u64, author: &str, head: &str) -> ListedPull {
    ListedPull { author: author.into(), head: head.into(), title: format!("{author}'s change {number}"), ..pull(number, SLUG) }
}

/// The forge answers the four lists, the requested one with `rows`; `nick` is signed in.
fn forge_lists(rig: &Rig, rows: Vec<ListedPull>) {
    rig.fleet.vcs().main_ci.login_is(Some("nick"));
    for list in [ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        rig.fleet.vcs().now_review_list(list, Vec::new());
    }
    rig.fleet.vcs().now_review_list(ReviewList::Requested, rows);
}

fn forge_files(rig: &Rig, paths: &[&str]) {
    let files = paths.iter().map(|path| PatchedFile { path: path.to_string(), patch: None, blob: None }).collect();
    rig.fleet.vcs().now_pull_request_patches(Some(files));
}

/// The lists as the forge says them now, and the files of the heads new to Fleet kept, as a refresh does.
async fn refreshed(rig: &Rig) -> ipc::PullReviewLists {
    let lists = rig.fleet.review_lists_read(None).await.expect("read");
    rig.fleet.open_pulls_filed(None, &lists).await;
    lists
}

fn files_read(rig: &Rig) -> usize {
    rig.fleet.vcs().delivered().iter().filter(|one| matches!(one, Delivered::ReadPullRequestPatches { .. })).count()
}

async fn kept(rig: &Rig) -> Vec<KeptOpenPull> {
    rig.fleet.store().lock().await.open_pulls_kept().expect("read")
}

#[tokio::test]
async fn the_files_of_an_open_pull_request_are_kept_with_its_author_and_mapped_to_the_feature_that_owns_them() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    forge_lists(&rig, vec![listed(412, "priya", "aaaa")]);
    forge_files(&rig, &[RANKS, "README.md"]);
    refreshed(&rig).await;

    let rows = kept(&rig).await;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!((row.number, row.repository.as_str(), row.author.as_str(), row.head_sha.as_str(), row.draft), (412, SLUG, "priya", "aaaa", false));
    assert_eq!((row.files.clone(), row.files_capped), (vec![RANKS.to_string(), "README.md".to_string()], false));

    let who = rig.fleet.get_whos_on_what(None, Some(project)).await.expect("read");
    assert_eq!(who.features.len(), 1, "only the feature a file belongs to: {who:?}");
    let feature = &who.features[0];
    assert_eq!(feature.slug, VOTING);
    assert_eq!(feature.cells.len(), 1);
    assert_eq!(feature.cells[0].login, "priya");
    assert_eq!(feature.cells[0].pulls.iter().map(|one| (one.number, one.title.as_str(), one.draft)).collect::<Vec<_>>(), [(412, "priya's change 412", false)]);
    assert!(feature.cells[0].jobs.is_empty());
}

#[tokio::test]
async fn a_pull_request_with_more_than_a_hundred_files_keeps_a_hundred_and_says_there_are_more() {
    let rig = rig(None);
    forge_lists(&rig, vec![listed(5, "priya", "aaaa")]);
    let names: Vec<String> = (0..130).map(|n| format!("crates/pages/voting/f{n}.rs")).collect();
    forge_files(&rig, &names.iter().map(String::as_str).collect::<Vec<_>>());
    refreshed(&rig).await;

    let row = kept(&rig).await.remove(0);
    assert_eq!((row.files.len(), row.files_capped), (100, true));
}

#[tokio::test]
async fn a_head_already_read_is_not_read_again_and_a_new_head_is() {
    let rig = rig(None);
    forge_lists(&rig, vec![listed(7, "priya", "aaaa"), listed(8, "sam", "bbbb")]);
    forge_files(&rig, &[RANKS]);
    refreshed(&rig).await;
    assert_eq!(files_read(&rig), 2, "one read for each pull request");

    refreshed(&rig).await;
    assert_eq!(files_read(&rig), 2, "the same heads cost nothing");

    // A retitle and a draft mark are read off the row, with no forge read, and the files stay.
    let mut retitled = listed(7, "priya", "aaaa");
    retitled.title = "A better title".into();
    retitled.draft = true;
    forge_lists(&rig, vec![retitled, listed(8, "sam", "bbbb")]);
    refreshed(&rig).await;
    assert_eq!(files_read(&rig), 2);
    let row = kept(&rig).await.into_iter().find(|one| one.number == 7).expect("kept");
    assert_eq!((row.title.as_str(), row.draft, row.files.clone()), ("A better title", true, vec![RANKS.to_string()]));

    forge_lists(&rig, vec![listed(7, "priya", "cccc"), listed(8, "sam", "bbbb")]);
    forge_files(&rig, &[ACTIONS]);
    refreshed(&rig).await;
    assert_eq!(files_read(&rig), 3, "only the pull request whose head moved");
    let row = kept(&rig).await.into_iter().find(|one| one.number == 7).expect("kept");
    assert_eq!((row.head_sha.as_str(), row.files.clone()), ("cccc", vec![ACTIONS.to_string()]));
}

#[tokio::test]
async fn a_pull_request_no_list_names_any_more_is_dropped_but_not_while_a_list_is_silent() {
    let rig = rig(None);
    forge_lists(&rig, vec![listed(7, "priya", "aaaa"), listed(8, "sam", "bbbb")]);
    forge_files(&rig, &[RANKS]);
    refreshed(&rig).await;
    assert_eq!(kept(&rig).await.iter().map(|one| one.number).collect::<Vec<_>>(), [7, 8]);

    // One list did not answer, so what it would have named is not known to be closed.
    forge_lists(&rig, vec![listed(8, "sam", "bbbb")]);
    rig.fleet.vcs().review_list_silent(ReviewList::Team);
    refreshed(&rig).await;
    assert_eq!(kept(&rig).await.iter().map(|one| one.number).collect::<Vec<_>>(), [7, 8], "a silent list drops nothing");

    forge_lists(&rig, vec![listed(8, "sam", "bbbb")]);
    refreshed(&rig).await;
    assert_eq!(kept(&rig).await.iter().map(|one| one.number).collect::<Vec<_>>(), [8], "seven closed");
}

#[tokio::test]
async fn the_owners_running_job_is_in_his_own_cell_beside_a_teammates_pull_request_and_his_own() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let job = an_agent_for(&rig, &project).await;
    wrote(&rig, &job, RANKS).await;
    held_slot(&rig, &job, "4").await;
    forge_lists(&rig, vec![listed(10, "Priya", "aaaa"), listed(11, "NICK", "bbbb")]);
    forge_files(&rig, &[RANKS]);
    refreshed(&rig).await;

    let who = rig.fleet.get_whos_on_what(None, Some(project)).await.expect("read");
    assert_eq!(who.people.iter().map(|one| (one.login.as_str(), one.me)).collect::<Vec<_>>(), [("nick", true), ("Priya", false)], "the owner first, by the login he is signed in as");
    let feature = who.features.iter().find(|one| one.slug == VOTING).expect("the feature");
    let cells: Vec<(&str, Vec<u64>, Vec<ipc::SessionId>)> = feature.cells.iter().map(|cell| (cell.login.as_str(), cell.pulls.iter().map(|one| one.number).collect(), cell.jobs.clone())).collect();
    assert_eq!(cells, [("nick", vec![11], vec![job]), ("Priya", vec![10], vec![])], "his Job and his pull request in his cell, hers in hers");
}

#[tokio::test]
async fn features_are_listed_most_people_first_then_by_title_and_only_where_somebody_is() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let seed = |slug: &str, title: &str, path: &str| FeatureSeed {
        slug: slug.into(),
        area: "takes".into(),
        title: title.into(),
        summary: "One line.".into(),
        state: None,
        paths: vec![path.into()],
        entry_points: Vec::new(),
        manifest_id: None,
    };
    let features = vec![seed("zeta", "Zeta cards", "crates/zeta"), seed("alpha", "Alpha cards", "crates/alpha"), seed("busy", "Busy page", "crates/busy"), seed("quiet", "Quiet page", "crates/quiet")];
    let seeded = UpsertFeatures { project: project.clone(), areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }], features };
    rig.fleet.upsert_features(None, seeded).await.expect("seeded");
    let pull_on = |number: u64, author: &str, file: &str| KeptOpenPull {
        manifest_id: rig.manifest.as_str().to_string(),
        number,
        repository: SLUG.into(),
        title: format!("Pull request {number}"),
        author: author.into(),
        draft: false,
        head_sha: "h".into(),
        files: vec![file.into()],
        files_capped: false,
        read_at: String::new(),
    };
    {
        let mut store = rig.fleet.store().lock().await;
        for one in [
            pull_on(1, "priya", "crates/zeta/a.rs"),
            pull_on(2, "priya", "crates/alpha/a.rs"),
            pull_on(3, "sam", "crates/busy/a.rs"),
            pull_on(4, "ada", "crates/busy/b.rs"),
            pull_on(5, "priya", "crates/busy/c.rs"),
            pull_on(6, "sam", "crates/elsewhere/a.rs"),
        ] {
            store.keep_open_pull(&one).expect("kept");
        }
    }
    let who = rig.fleet.get_whos_on_what(None, Some(project)).await.expect("read");
    let order: Vec<(&str, usize)> = who.features.iter().map(|one| (one.slug.as_str(), one.cells.len())).collect();
    assert_eq!(order, [("busy", 3), ("alpha", 1), ("zeta", 1)], "three people first, ties by title, the quiet feature and the file no feature owns left out");
    let busy = &who.features[0];
    assert_eq!(busy.cells.iter().map(|cell| (cell.login.as_str(), cell.pulls.len())).collect::<Vec<_>>(), [("ada", 1), ("priya", 1), ("sam", 1)], "cells by login where none is the owner's");
    assert_eq!(who.people.iter().map(|one| one.login.as_str()).collect::<Vec<_>>(), ["ada", "priya", "sam"], "nobody is signed in, so nobody is the owner");
}

#[tokio::test]
async fn a_job_handed_a_feature_is_told_of_a_teammates_open_pull_request_on_it_and_not_of_its_owners_own() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let job = an_agent_for(&rig, &project).await;
    forge_lists(&rig, vec![listed(412, "priya", "aaaa"), listed(413, "nick", "bbbb")]);
    forge_files(&rig, &[RANKS]);
    refreshed(&rig).await;

    let brief = format!("Rework the ranks in `{VOTING}`.");
    let handed = rig.fleet.hand_over_features(hand(&job, None, &[], Some(&brief))).await.expect("handed");
    assert_eq!(handed.features, [VOTING]);
    assert!(handed.text.contains("Open elsewhere: #412 \"priya's change 412\" by priya touches this feature."), "{}", handed.text);
    assert!(!handed.text.contains("#413"), "his own pull request is no news to him: {}", handed.text);
    assert_eq!(handed.text.matches("Open elsewhere").count(), 1);

    // A feature no open pull request touches says nothing of the kind.
    let other = rig.fleet.hand_over_features(hand(&job, None, &["crates/pages/actions/menu.rs"], None)).await.expect("handed");
    assert_eq!(other.features, ["page-actions"]);
    assert!(!other.text.contains("Open elsewhere"), "{}", other.text);
}
