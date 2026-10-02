//! What the repository requires of every change, in the Drone's own opening
//! brief as well as its Judge's. Driven through a whole Fleet, because what is
//! under test is that the spawn reads the file the Manifest names and puts it
//! where the Drone reads it — not only that a block renders.

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
/// holds the rules file and whose Manifest names it or does not.
async fn opened_with(standing_rules: Option<&str>) -> String {
    let home = TempDir::new();
    let file = home.path().join("docs/every-change.md");
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
    std::fs::write(&file, format!("{A_RULE}\n")).expect("the rules file");

    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let key = standing_rules
        .map(|path| format!("standing_rules: {path}\n"))
        .unwrap_or_default();
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

#[tokio::test]
async fn a_manifest_naming_a_rules_file_puts_it_in_the_drone_s_brief_in_its_own_section() {
    let brief = opened_with(Some("docs/every-change.md")).await;

    let section = brief
        .find(&format!("\n\n{HEADING}\n\n"))
        .expect("the section opens on its own heading");
    let rule = brief.find(A_RULE).expect("the file's text is in it");
    let job = brief.find("\n\nJOB BRIEF\n\n").expect("the job brief");
    assert!(section < rule && rule < job, "{brief}");
    assert!(
        brief[section..].contains("What this repository requires of every change you make"),
        "worded for the one doing the work: {brief}"
    );
}

/// Today's brief, byte for byte: the only difference a named file makes is the
/// section it adds.
#[tokio::test]
async fn a_manifest_naming_none_gives_the_drone_the_brief_it_always_did() {
    let without = opened_with(None).await;
    let with = opened_with(Some("docs/every-change.md")).await;

    assert!(!without.contains(HEADING), "{without}");
    let from = with.find(HEADING).expect("the section");
    let to = from + with[from..].find("\n\nJOB BRIEF").expect("the next block") + 2;
    assert_eq!(
        format!("{}{}", &with[..from], &with[to..]),
        without,
        "the section is the whole of the difference"
    );
}
