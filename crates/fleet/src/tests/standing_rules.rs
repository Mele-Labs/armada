//! What the repository requires of every change goes to the Judge and never
//! into the Drone's opening brief: a Drone reads the repository itself (owner,
//! 2 Oct 2026). Driven through a whole Fleet, because the block it keeps out
//! was put there by the spawn, not by a renderer a unit test would reach.

use std::path::Path;

use config::Manifest;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

const HEADING: &str = "WHAT THIS REPOSITORY REQUIRES OF EVERY CHANGE";
const A_RULE: &str = "Prose the change makes wrong is fixed in the same change.";

/// The opening brief the first Drone is given, in a repository whose checkout
/// holds the rules file and whose Manifest names it.
async fn opened_with(standing_rules: &str) -> String {
    let home = TempDir::new();
    let file = home.path().join("docs/every-change.md");
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(&file, format!("{A_RULE}\n")).expect("the rules file");

    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let key = format!("standing_rules: {standing_rules}\n");
    fittings.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{key}"),
    )
    .expect("a manifest");
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a proposal");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect("it is approved");
    let configured = fleet.harness().configured();
    assert_eq!(configured.len(), 1, "one Drone was put on the first step");
    configured[0].prompt().as_str().to_string()
}

/// Removed on 2 Oct 2026 after one day in every brief. This keeps it out.
#[tokio::test]
async fn a_drone_s_brief_carries_no_standing_rules_even_when_the_manifest_names_a_file() {
    let brief = opened_with("docs/every-change.md").await;

    assert!(!brief.contains(HEADING), "{brief}");
    assert!(
        !brief.contains(A_RULE),
        "the file's text is not quoted: {brief}"
    );
}
