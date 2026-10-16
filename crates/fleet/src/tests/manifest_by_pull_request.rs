//! A form's save goes out as a pull request and applies at once.
//!
//! 14 Oct 2026: `SetAutoMerge` left `auto_merge: checks-pass` uncommitted in the
//! owner's checkout, and Fleet only fast-forwards a clean one, so that file
//! blocked every catch-up. The forge is scripted; what the real commands do to
//! a repository is `adapters`' to assert.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Landing, PullRequestFor};
use config::{Manifest, Reloads};
use core_model::{AutoMerge, ReviewGate};
use ipc::{EditManifest, ManifestEdit};
use testkit::{Delivered, FakeWorkProduct};

use crate::noticing::Noticing;
use crate::repositories::{Catalogued, Located, Locating, NotLocated, SavedWorkflow, WorkflowNotSaved};
use crate::tests::main_ci::{Fixture, ONE, TWO};
use crate::tests::tmp::TempDir;

const KEPT: &str = "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\nchecks:\n  test:\n    run: cargo nextest run\n";
const PULL: &str = "https://forge.invalid/armada/pull/1";

/// The composition root's half of a reload: the handle it holds, read through
/// when Fleet says what the Manifest now is.
struct Reloading(Reloads);

impl Locating for Reloading {
    fn save_workflow(
        &self,
        _root: &Path,
        _manifest: &Manifest,
        _asked: &ipc::SaveWorkflow,
    ) -> Result<SavedWorkflow, WorkflowNotSaved> {
        unreachable!("nothing here saves a workflow")
    }

    fn workflows(&self, _root: &Path, _manifest: &Manifest, _workspaces: &[Manifest]) -> Catalogued {
        Catalogued::default()
    }

    fn located(&self, folder: &Path) -> Result<Located, NotLocated> {
        Err(NotLocated::NotARepository {
            folder: folder.display().to_string(),
            why: String::from("nothing here locates"),
        })
    }

    fn serving(&self, _root: &str) {}

    fn reload_from(&self, _root: &str, text: Option<&str>) {
        self.0.hold(text.map(str::to_string));
        let _ = self.0.reread();
    }
}

fn a_fleet_over(home: &TempDir, manifest: &str) -> (Fixture, std::path::PathBuf) {
    let file = home.path().join("armada.yml");
    std::fs::write(&file, manifest).expect("a Manifest to start from");
    let (parsed, reloads) = Manifest::reloadable(&file).expect("the fixture loads");
    let mut fittings = crate::tests::daemon::fittings(home, FakeWorkProduct::changed(&[]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.starting().manifest = parsed;
    fittings.locating = Arc::new(Reloading(reloads));
    (crate::daemon::Fleet::assembled(fittings), file)
}

fn tracked(home: &TempDir) -> (Fixture, std::path::PathBuf) {
    let (fleet, file) = a_fleet_over(home, KEPT);
    fleet.vcs().now_manifest_goes_by_pull_request(true);
    fleet.vcs().now_base_holds_manifest(ONE, KEPT);
    (fleet, file)
}

fn auto_merge(word: &str) -> ManifestEdit {
    ManifestEdit::SetAutoMerge {
        auto_merge: Some(word.to_string()),
    }
}

async fn save(fleet: &Fixture, read: String, edit: ManifestEdit) -> ipc::ManifestEdited {
    fleet
        .edit_manifest_file(EditManifest { read, edits: vec![edit] }, &fleet.first())
        .await
        .expect("the edit is accepted")
}

fn what_a_form_reads(fleet: &Fixture) -> String {
    fleet.read_manifest_file(&fleet.first()).expect("reads").text
}

fn commit_of(number: usize) -> String {
    format!("{number:040x}")
}

#[tokio::test]
async fn a_save_on_a_tracked_manifest_opens_one_pull_request_and_applies_at_once() {
    let home = TempDir::new();
    let (fleet, file) = tracked(&home);
    assert_eq!(fleet.first().manifest().auto_merge(), AutoMerge::Never);

    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    let sent = fleet.vcs().manifest_edits_sent();
    let [only] = sent.as_slice() else {
        panic!("one edit sent, got {sent:?}")
    };
    assert_eq!(only.to, PullRequestFor::Open { base: String::from("main") });
    assert_eq!(only.parent, ONE, "made on the base's tip");
    assert_eq!(only.message, "armada.yml: auto_merge checks-pass");
    assert!(only.branch.starts_with("armada/manifest-"), "{}", only.branch);
    assert_eq!(only.text, edited.text);
    assert!(edited.text.contains("auto_merge: checks-pass"));
    assert!(fleet
        .vcs()
        .delivered()
        .contains(&Delivered::AutoMerge { pull_request: PULL.to_string() }));
    assert_eq!(
        std::fs::read_to_string(&file).expect("reads"),
        KEPT,
        "the file Fleet serves is not written"
    );
    assert_eq!(
        fleet.first().manifest().auto_merge(),
        AutoMerge::ChecksPass,
        "and the live key reads the pending text"
    );
    assert_eq!(what_a_form_reads(&fleet), edited.text);
}

#[tokio::test]
async fn a_second_save_stacks_on_the_same_branch_and_opens_nothing_more() {
    let home = TempDir::new();
    let (fleet, file) = tracked(&home);
    let first = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    let gate = ManifestEdit::SetReviewGate {
        review_gate: Some(String::from("auto_if_judge_passes")),
    };
    let second = save(&fleet, first.text.clone(), gate).await;

    let sent = fleet.vcs().manifest_edits_sent();
    let [one, two] = sent.as_slice() else {
        panic!("two edits sent, got {sent:?}")
    };
    assert_eq!(
        two.to,
        PullRequestFor::Update {
            branch: one.branch.clone(),
            pull_request: PULL.to_string(),
        }
    );
    assert_eq!(two.branch, one.branch);
    assert_eq!(two.parent, commit_of(1), "on top of the first edit's commit");
    assert_eq!(two.message, "armada.yml: review_gate auto_if_judge_passes");
    assert!(second.text.contains("auto_merge: checks-pass"), "{}", second.text);
    assert!(second.text.contains("review_gate: auto_if_judge_passes"));
    assert_eq!(
        sent.iter()
            .filter(|each| matches!(each.to, PullRequestFor::Open { .. }))
            .count(),
        1
    );
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), KEPT);
    assert_eq!(fleet.first().manifest().auto_merge(), AutoMerge::ChecksPass);
    assert_eq!(fleet.first().manifest().review_gate(), ReviewGate::AutoIfJudgePasses);
}

/// The form was drawn before the edit, so what it read is not what the
/// Manifest is now.
#[tokio::test]
async fn a_form_drawn_before_the_pending_edit_is_refused_with_the_pending_text() {
    let home = TempDir::new();
    let (fleet, _) = tracked(&home);
    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    let refused = fleet
        .edit_manifest_file(
            EditManifest {
                read: KEPT.to_string(),
                edits: vec![auto_merge("always")],
            },
            &fleet.first(),
        )
        .await
        .expect_err("the form read the file under the edit");
    assert_eq!(refused.status(), 409);
    assert!(matches!(
        refused.error().fields.get("on_disk"),
        Some(ipc::WireValue::Str(text)) if *text == edited.text
    ));
}

/// What the base holds is not always what the form read: the edit is placed by
/// key in the text the branch will carry.
#[tokio::test]
async fn an_edit_is_placed_in_the_text_the_base_holds_when_it_is_not_the_text_the_form_read() {
    let home = TempDir::new();
    let (fleet, _) = tracked(&home);
    let merged_since = KEPT.replace("cargo nextest run", "cargo nextest run --all");
    fleet.vcs().now_base_holds_manifest(TWO, &merged_since);

    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    let sent = fleet.vcs().manifest_edits_sent();
    assert_eq!(sent[0].parent, TWO);
    assert!(sent[0].text.contains("--all"), "{}", sent[0].text);
    assert!(sent[0].text.contains("auto_merge: checks-pass"));
    assert_eq!(edited.text, sent[0].text);
}

#[tokio::test]
async fn once_the_checkout_has_the_commit_the_pending_edit_is_dropped() {
    let home = TempDir::new();
    let (fleet, file) = tracked(&home);
    fleet.vcs().main_ci.head_is(Some(TWO));
    fleet.vcs().checkout_on(ONE);
    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    fleet.turn().await.expect("a turn");
    assert_eq!(what_a_form_reads(&fleet), edited.text, "the checkout lacks the commit");

    // The catch-up wrote the merged file, and the checkout has the commit.
    std::fs::write(&file, &edited.text).expect("the merged file");
    fleet.vcs().main_ci.head_is(Some(&commit_of(1)));
    fleet.vcs().checkout_on(&commit_of(1));
    fleet.turn().await.expect("a turn");

    assert_eq!(what_a_form_reads(&fleet), edited.text, "now read from the file");
    assert_eq!(fleet.first().manifest().auto_merge(), AutoMerge::ChecksPass);
    assert_eq!(fleet.first().repository().pending_edit().standing().map(|it| it.commit), None);
}

#[tokio::test]
async fn a_pull_request_closed_unmerged_drops_it_and_the_live_value_goes_back() {
    let home = TempDir::new();
    let (fleet, file) = tracked(&home);
    fleet.vcs().main_ci.head_is(Some(TWO));
    fleet.vcs().checkout_on(ONE);
    save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;
    assert_eq!(fleet.first().manifest().auto_merge(), AutoMerge::ChecksPass);

    fleet.vcs().now_landed(Landing::ClosedUnmerged { url: PULL.to_string() });
    fleet.turn().await.expect("a turn");

    assert_eq!(fleet.first().manifest().auto_merge(), AutoMerge::Never);
    assert_eq!(what_a_form_reads(&fleet), KEPT);
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), KEPT);
}

#[tokio::test]
async fn an_ignored_manifest_is_written_to_disk_as_before() {
    let home = TempDir::new();
    let (fleet, file) = a_fleet_over(&home, KEPT);
    fleet.vcs().now_manifest_goes_by_pull_request(false);

    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    assert!(fleet.vcs().manifest_edits_sent().is_empty());
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), edited.text);
    assert!(edited.text.contains("auto_merge: checks-pass"));
}

#[tokio::test]
async fn a_manifest_that_names_no_base_is_written_to_disk_as_before() {
    let home = TempDir::new();
    let no_base = KEPT.replace("base: main\n", "");
    let (fleet, file) = a_fleet_over(&home, &no_base);
    fleet.vcs().now_manifest_goes_by_pull_request(true);

    let edited = save(&fleet, what_a_form_reads(&fleet), auto_merge("checks-pass")).await;

    assert!(fleet.vcs().manifest_edits_sent().is_empty());
    assert_eq!(std::fs::read_to_string(&file).expect("reads"), edited.text);
}
