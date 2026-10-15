//! A red main whose pull request a Session merged is handed to that Session by Fleet itself:
//! claimed for it, recorded on its ledger and told to it. A Session that cannot be told is
//! forked, and the fork holds the claim and is told. A red held elsewhere is left alone, and a
//! Job's red still goes to the Job.
//!
//! **Beside `red_claim`**, whose two-machine rig this borrows: the claim is read back from the
//! shared store, as there.

use std::sync::Arc;

use adapter_traits::{Holder, HolderKind, Store};
use api::{HostedSessions, Sessions};
use ipc::{
    ManifestId, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SessionVoice,
    StartSession,
};
use store::{AttachmentState, KeptAttachment};
use testkit::FakeStore;

use super::main_ci::{Fixture, ONE};
use super::main_fix::{a_finished_job, goes_red, root, takes};
use super::red_claim::{machine_hosting, Machine, SLUG};
use super::session_host::StandIn;
use crate::red_claim::keys_of;
use crate::tests::tmp::TempDir;

struct Rig {
    fleet: Arc<Fixture>,
    manifest: ManifestId,
    root: String,
    _stand_in: Arc<StandIn>,
    home: TempDir,
}

async fn rig(shared: &FakeStore) -> Rig {
    let stand_in = Arc::new(StandIn::default());
    let Machine { fleet, _home, .. } =
        machine_hosting("mac-a", shared, true, Some(Arc::clone(&stand_in))).await;
    let served = fleet.repositories().first().expect("a served repository");
    let manifest = ManifestId::carried(served.manifest().id().as_str());
    let root = root(&fleet).await;
    Rig { fleet: Arc::new(fleet), manifest, root, _stand_in: stand_in, home: _home }
}

fn claim_in(store: &FakeStore) -> Option<Holder> {
    let (repository, red) = keys_of(SLUG, ONE).expect("keys");
    store.read_claim(&repository, &red).expect("reads")
}

impl Rig {
    /// A Session Bridge hosts, in this repository.
    async fn a_hosted_session(&self) -> String {
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                root: None,
                pilot: None,
                fork: None,
            })
            .await
            .expect("started")
            .id
            .as_str()
            .to_string()
    }

    /// A Session run in a terminal that has since ended.
    async fn an_ended_session(&self, id: &str) {
        for fact in [
            SessionFact::Started {
                cwd: self.root.clone(),
                title: Some("Why the reader drops lines".into()),
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
            SessionFact::Ended { reason: "prompt_input_exit".into() },
        ] {
            self.fleet
                .report_session(SessionReport {
                    harness: adapters::HOSTED_HARNESS.into(),
                    session_id: SessionId::carried(id),
                    fact,
                })
                .await
                .unwrap();
        }
    }

    /// The Session held pull request `number`, and it has merged.
    async fn held_pull(&self, id: &str, number: u64) {
        let at = "2026-10-14T09:00:00.000Z".to_string();
        self.fleet
            .store()
            .lock()
            .await
            .attach(
                &KeptAttachment {
                    holder: store::Holder::session(id),
                    kind: crate::pull_requesting::PR.into(),
                    manifest_id: self.manifest.as_str().to_string(),
                    target: number.to_string(),
                    state: AttachmentState::Spent,
                    detail: Default::default(),
                    since: at.clone(),
                    changed_at: at,
                },
                false,
            )
            .unwrap();
    }

    /// One turn, then what the loop does with the `Arc` after it.
    async fn turned(&self) -> Vec<crate::main_ci::MainChanged> {
        let turned = self.fleet.turn().await.unwrap();
        self.fleet.red_given_to_sessions(&turned.main_changed).await;
        turned.main_changed
    }

    async fn thread(&self, id: &str) -> Vec<String> {
        Arc::clone(&self.fleet)
            .get_session(SessionId::carried(id))
            .await
            .expect("read")
            .rows
            .into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { from: SessionVoice::Session { .. }, text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    async fn handed(&self, id: &str) -> Vec<store::KeptAttachment> {
        self.fleet
            .store()
            .lock()
            .await
            .attachments_of(&store::Holder::session(id))
            .unwrap()
            .into_iter()
            .filter(|one| one.kind == crate::session_red::MAIN_RED)
            .collect()
    }

    async fn other_than(&self, id: &str) -> Vec<String> {
        Arc::clone(&self.fleet)
            .list_sessions(None, None, None)
            .await
            .unwrap()
            .sessions
            .into_iter()
            .map(|one| one.id.as_str().to_string())
            .filter(|one| one != id)
            .collect()
    }
}

#[tokio::test]
async fn the_session_that_merged_the_pull_request_is_woken_with_the_red_claimed_for_it() {
    let shared = FakeStore::new();
    let rig = rig(&shared).await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1815).await;
    goes_red(&rig.fleet, 1815);

    let changed = rig.turned().await;
    let held = claim_in(&shared).expect("Fleet claimed the red");
    assert_eq!((held.kind, held.id.as_str(), held.machine.as_str()), (HolderKind::Session, id.as_str(), "mac-a"));
    let [told] = rig.thread(&id).await.try_into().expect("told once");
    assert!(
        told.contains("Main went red at 1111111 after your pull request #1815 merged")
            && told.contains("test fails on main.")
            && told.contains("FAIL ["),
        "{told}"
    );
    let [take] = rig.handed(&id).await.try_into().expect("recorded once");
    assert_eq!(take.detail.get("how").map(String::as_str), Some("woke"));
    assert!(takes(&rig.fleet).await.is_empty(), "no Job was made for it");
    assert_eq!(rig.other_than(&id).await.len(), 0, "and nothing was forked");

    rig.fleet.red_given_to_sessions(&changed).await;
    assert_eq!(rig.thread(&id).await.len(), 1, "once per red per Session");
}

#[tokio::test]
async fn a_session_that_has_ended_is_forked_and_the_fork_holds_the_claim_and_is_told() {
    let shared = FakeStore::new();
    let rig = rig(&shared).await;
    rig.an_ended_session("aaaa-1111").await;
    rig.held_pull("aaaa-1111", 1815).await;
    goes_red(&rig.fleet, 1815);

    let changed = rig.turned().await;
    let [fork] = rig.other_than("aaaa-1111").await.try_into().expect("one fork");
    let held = claim_in(&shared).expect("claimed");
    assert_eq!((held.kind, held.id.as_str()), (HolderKind::Session, fork.as_str()), "the fork holds it");
    assert!(rig.thread("aaaa-1111").await.is_empty());
    let [told] = rig.thread(&fork).await.try_into().expect("the fork is told");
    assert!(
        told.contains("Main went red at 1111111 after pull request #1815, which session aaaa-1111 merged"),
        "{told}"
    );
    let [old] = rig.handed("aaaa-1111").await.try_into().expect("recorded on the old one");
    assert_eq!(old.detail.get("how").map(String::as_str), Some("forked"));
    assert_eq!(old.detail.get("fork"), Some(&fork));
    assert_eq!(rig.handed(&fork).await.len(), 1);

    rig.fleet.red_given_to_sessions(&changed).await;
    assert_eq!(rig.other_than("aaaa-1111").await.len(), 1, "not forked twice");
}

#[tokio::test]
async fn a_red_held_elsewhere_is_left_alone() {
    let shared = FakeStore::new();
    let rig = rig(&shared).await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1815).await;
    goes_red(&rig.fleet, 1815);
    let (repository, red) = keys_of(SLUG, ONE).expect("keys");
    let other = Holder::new(HolderKind::Job, "01JOBELSEWHERE", "mac-b", 1).expect("a holder");
    shared.claim(&repository, &red, &other).expect("claimed");

    rig.turned().await;
    assert_eq!(claim_in(&shared).expect("still held").machine, "mac-b");
    assert!(rig.thread(&id).await.is_empty(), "the Session was not told");
    assert!(rig.handed(&id).await.is_empty());
    assert!(rig.other_than(&id).await.is_empty(), "and nothing was forked");
}

#[tokio::test]
async fn a_jobs_red_still_goes_to_the_job() {
    let shared = FakeStore::new();
    let rig = rig(&shared).await;
    let culprit = a_finished_job(&rig.fleet, &rig.home, "fix the off-by-one in the log reader").await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1).await;
    goes_red(&rig.fleet, 1);

    rig.turned().await;
    let [take] = takes(&rig.fleet).await.try_into().expect("the Job's send-back");
    assert_ne!(take.job, *culprit.id());
    let held = claim_in(&shared).expect("claimed");
    assert_eq!((held.kind, held.id.as_str()), (HolderKind::Job, take.job.as_str()));
    assert!(rig.thread(&id).await.is_empty(), "the Session was not told");
    assert!(rig.handed(&id).await.is_empty());
}
