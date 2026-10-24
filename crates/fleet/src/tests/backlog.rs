//! A repository whose tracker Fleet writes to: `claim_issue` claims there as sync, claim, sync,
//! read back, two Fleets on one remote; and a look syncs a copy once something wrote to it, after
//! which the issue list is read again. A fake backlog stands in for `bd`.

use std::sync::Arc;

use adapter_traits::{Backlog, Tracker};
use api::{Issues, Sessions};
use ipc::{ClaimIssue, ManifestId, SessionId};
use testkit::{FakeBeads, FakeBeadsRemote, FakeTrackers};

use crate::tests::session_host::{rig_fitted, Rig};

pub(super) const BEADS: &str = "version: 1\nid: 01FIXTUREMANIFEST\ntracker:\n  provider: beads\n";

/// A Fleet named `name` whose repository's tracker is `copy`.
pub(super) async fn machine(name: &str, copy: &Arc<FakeBeads>) -> Rig {
    machine_reading(name, copy, BEADS).await
}

/// [`machine`] with a Manifest of the test's own.
pub(super) async fn machine_reading(name: &str, copy: &Arc<FakeBeads>, manifest: &'static str) -> Rig {
    let planted = Arc::clone(copy);
    let rig = rig_fitted(move |fittings| {
        fittings.trackers = Arc::new(FakeTrackers::empty().beads_named("beads", planted));
        fittings.starting().manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), manifest).expect("loads");
    });
    let changes = [("machine.name".to_string(), Some(ipc::SettingValue::Text(name.to_string())))].into_iter().collect();
    api::Settings::save_settings(&*rig.fleet, ipc::SaveSettings { changes }).await.expect("saved");
    rig
}

pub(super) fn manifest(rig: &Rig) -> ManifestId {
    ManifestId::carried(rig.fleet.repositories().first().expect("served").manifest().id().as_str())
}

pub(super) async fn claims(rig: &Rig, id: &SessionId, issue: &str) -> Result<ipc::IssueClaimed, api::Refusal> {
    rig.fleet
        .claim_issue(None, ClaimIssue { issue: issue.into(), session_id: Some(id.clone()), pull_requests: Vec::new() })
        .await
}

pub(super) fn actor(name: &str, id: &SessionId) -> String {
    format!("{name}/{}", id.as_str().chars().take(8).collect::<String>())
}

#[tokio::test]
async fn a_claim_is_synced_and_read_back_and_one_another_machine_holds_is_refused_naming_it() {
    let remote = FakeBeadsRemote::new();
    let (copy_a, copy_b) = (Arc::new(FakeBeads::on(&remote)), Arc::new(FakeBeads::on(&remote)));
    let (a, b) = (machine("mac-a", &copy_a).await, machine("mac-b", &copy_b).await);
    let reference = copy_a.create("Claim through Fleet");
    copy_a.sync("/").expect("pushed");
    copy_b.sync("/").expect("pulled");
    let (job_a, job_b) = (a.start().await, b.start().await);

    let claimed = claims(&a, &job_a, &reference).await.expect("A holds it");
    assert_eq!(claimed.issue, reference);
    assert_eq!(remote.holder(&reference), Some(actor("mac-a", &job_a)), "the claim is on the remote");

    match claims(&b, &job_b, &reference).await {
        Err(api::Refusal::IllegalMove(why)) => {
            assert_eq!(why.code, "fleet.issue_held");
            assert!(why.message.contains(&actor("mac-a", &job_a)), "{}", why.message);
        }
        other => panic!("B's claim is a 409 naming A: {other:?}"),
    }
    assert_eq!(remote.holder(&reference), Some(actor("mac-a", &job_a)), "the loser changed nothing");
    let held_on_b = b.fleet.list_sessions(None, Some(job_b.as_str().into()), None).await.expect("listed");
    assert!(held_on_b.sessions[0].attachments.iter().all(|one| one.kind != "issue"), "B's Job holds no issue row");
}

#[tokio::test]
async fn a_look_syncs_a_copy_once_it_was_written_and_the_issues_are_read_again_after() {
    let remote = FakeBeadsRemote::new();
    let (copy_a, copy_b) = (Arc::new(FakeBeads::on(&remote)), Arc::new(FakeBeads::on(&remote)));
    let a = machine("mac-a", &copy_a).await;
    assert!(a.fleet.list_issues(manifest(&a), false).await.expect("listed").issues.is_empty());

    let _ = a.fleet.backlogs_looked().await;
    let _ = a.fleet.backlogs_looked().await;
    assert_eq!(copy_a.syncs(), 1, "the first look syncs, and one with nothing written and the timer not due does not");

    let theirs = copy_b.create("Filed on the other machine");
    copy_b.sync("/").expect("pushed");
    let _ = a.fleet.backlogs_looked().await;
    assert_eq!(copy_a.syncs(), 1, "another machine's write is heard on the timer, not at once");

    let ours = copy_a.create("Filed by an agent here with bd");
    let _ = a.fleet.backlogs_looked().await;
    assert_eq!(copy_a.syncs(), 2, "a write here is synced at the next look");
    assert_eq!(remote.holder(&ours), Some(String::new()), "and pushed");

    let listed = a.fleet.list_issues(manifest(&a), false).await.expect("listed");
    let mut references: Vec<String> = listed.issues.iter().map(|one| one.reference.clone()).collect();
    references.sort();
    let mut both = vec![theirs, ours];
    both.sort();
    assert_eq!(references, both, "the list held from before the sync is read again");
    assert!(copy_a.open_issues("/").is_ok());
}
