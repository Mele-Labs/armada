//! A red main whose pull request a Session merged is handed to that Session by Fleet itself:
//! claimed for it, recorded as its take and told to it. A Session that cannot be told is
//! forked, and the fork holds the claim and is told. A red held elsewhere is left alone, and a
//! Job's red still goes to the Job.
//!
//! **A Session's take is a take**: the hub's Fixing row shows it with no store configured, and
//! every guard that a Job's take raises, `fix_main` and `take_main_red` included, a Session's
//! raises too. Green ends it.
//!
//! **Beside `red_claim`**, whose two-machine rig this borrows: the claim is read back from the
//! shared store, as there.

use std::sync::Arc;

use adapter_traits::{CiState, Holder, HolderKind, Store};
use api::{HostedSessions, Sessions};
use ipc::{
    ManifestId, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SessionVoice,
    StartSession,
};
use store::{AttachmentState, KeptAttachment, TakenBy, TakenHow};
use testkit::FakeStore;

use super::main_ci::{kept, run, the_hub, Fixture, ONE, TWO};
use crate::main_ci::{MainChange, MainChanged};
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
    rig_with(shared, true).await
}

/// No central store applies: the repository's *Pull request owners* switch is off, as on every
/// machine today.
async fn local_rig() -> Rig {
    rig_with(&FakeStore::new(), false).await
}

async fn rig_with(shared: &FakeStore, switch: bool) -> Rig {
    let stand_in = Arc::new(StandIn::default());
    let Machine { fleet, _home, .. } =
        machine_hosting("mac-a", shared, switch, Some(Arc::clone(&stand_in))).await;
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
                request: None,
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

    /// The takes of this red that a Session made.
    async fn session_takes(&self) -> Vec<store::MainFix> {
        takes(&self.fleet).await.into_iter().filter(|take| take.by.session().is_some()).collect()
    }

    async fn jobs_made(&self) -> usize {
        self.fleet.every_job().await.unwrap().0.jobs.len()
    }

    /// A terminal Session in this repository, which then asks for the red itself.
    async fn a_terminal_session(&self, id: &str) {
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(id),
                fact: SessionFact::Started {
                    cwd: self.root.clone(),
                    title: Some("Why the reader drops lines".into()),
                    origin: SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .unwrap();
    }

    async fn takes_the_red(&self, id: &str) -> Result<ipc::MainRedTaken, api::Refusal> {
        self.fleet.take_main_red(None, ipc::TakeMainRed { session_id: Some(SessionId::carried(id)), take_over: false }).await
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
    let [take] = takes(&rig.fleet).await.try_into().expect("recorded once");
    assert_eq!((&take.by, take.how, &take.forked_from), (&TakenBy::Session(id.clone()), TakenHow::Took, &None));
    assert_eq!(rig.jobs_made().await, 0, "no Job was made for it");
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
    let [take] = takes(&rig.fleet).await.try_into().expect("the fork's take, and none for the old one");
    assert_eq!((&take.by, take.how), (&TakenBy::Session(fork.clone()), TakenHow::Dispatched));
    assert_eq!(take.forked_from.as_deref(), Some("aaaa-1111"));

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
    assert!(takes(&rig.fleet).await.is_empty());
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
    let job = take.by.job().expect("a Job's take");
    assert_ne!(job, culprit.id());
    let held = claim_in(&shared).expect("claimed");
    assert_eq!((held.kind, held.id.as_str()), (HolderKind::Job, job.as_str()));
    assert!(rig.thread(&id).await.is_empty(), "the Session was not told");
    assert!(rig.session_takes().await.is_empty());
}

#[tokio::test]
async fn a_sessions_take_shows_in_the_hubs_fixing_row_with_no_store_configured() {
    let rig = local_rig().await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1815).await;
    goes_red(&rig.fleet, 1815);

    rig.turned().await;
    let hub = the_hub(&rig.fleet).await;
    let fixing = hub.fixing_session.expect("the Session is in the Fixing row");
    assert_eq!(fixing.id, id);
    assert!(hub.fixing.is_none(), "and no Job is");
    assert!(hub.fixing_elsewhere.is_none());
}

#[tokio::test]
async fn the_fork_is_the_one_the_hub_names() {
    let rig = local_rig().await;
    rig.an_ended_session("aaaa-1111").await;
    rig.held_pull("aaaa-1111", 1815).await;
    goes_red(&rig.fleet, 1815);

    let changed = rig.turned().await;
    let [fork] = rig.other_than("aaaa-1111").await.try_into().expect("one fork");
    let fixing = the_hub(&rig.fleet).await.fixing_session.expect("the fork holds it");
    assert_eq!((fixing.id.as_str(), fixing.title.as_deref()), (fork.as_str(), Some("Fix main after #1815")));

    // The fork ends: it holds nothing now, and the old Session is not forked again for this red.
    rig.fleet.close_session(ipc::CloseSession { session_id: SessionId::carried(&fork) }).await.unwrap();
    assert!(the_hub(&rig.fleet).await.fixing_session.is_none());
    rig.fleet.red_given_to_sessions(&changed).await;
    assert_eq!(rig.other_than("aaaa-1111").await.len(), 1, "once per red per Session, fork or not");
}

#[tokio::test]
async fn a_session_that_took_the_red_itself_shows_in_the_hub_and_its_end_frees_it() {
    let rig = local_rig().await;
    goes_red(&rig.fleet, 1815);
    rig.fleet.turn().await.unwrap();
    rig.a_terminal_session("s-a").await;

    let taken = rig.takes_the_red("s-a").await.expect("taken");
    assert!(!taken.claimed, "no store applies");
    let [take] = rig.session_takes().await.try_into().expect("recorded");
    assert_eq!((&take.by, take.how), (&TakenBy::Session("s-a".to_string()), TakenHow::Took));
    assert_eq!(the_hub(&rig.fleet).await.fixing_session.map(|one| one.id), Some("s-a".to_string()));

    rig.an_ended_session("s-a").await;
    assert!(the_hub(&rig.fleet).await.fixing_session.is_none(), "an ended Session holds nothing");
    rig.a_terminal_session("s-b").await;
    rig.takes_the_red("s-b").await.expect("free again");
}

#[tokio::test]
async fn nobody_fixes_a_red_a_session_holds() {
    let rig = local_rig().await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1815).await;
    goes_red(&rig.fleet, 1815);
    rig.turned().await;
    let culprit = a_finished_job(&rig.fleet, &rig.home, "fix the off-by-one in the log reader").await;
    let jobs = rig.jobs_made().await;
    let root = rig.root.clone();

    for send_to in [None, Some(culprit.id())] {
        for take_over in [false, true] {
            let refused = rig.fleet.fix_main(&root, send_to, None, take_over).await.unwrap_err();
            let wire = rig.fleet.refusal(refused);
            assert_eq!(wire.error().code, "fleet.main_already_taken");
            assert!(wire.error().message.contains(&id), "{}", wire.error().message);
        }
    }
    assert_eq!(rig.jobs_made().await, jobs, "no Job was proposed");

    rig.a_terminal_session("s-b").await;
    let refused = rig.takes_the_red("s-b").await.unwrap_err();
    assert_eq!(refused.error().code, "fleet.main_already_taken");
    assert!(refused.error().message.contains(&id), "{}", refused.error().message);
    assert_eq!(rig.session_takes().await.len(), 1, "and no second take was kept");
}

#[tokio::test]
async fn the_jobs_pickup_leaves_a_red_a_session_holds() {
    let rig = local_rig().await;
    let culprit = a_finished_job(&rig.fleet, &rig.home, "fix the off-by-one in the log reader").await;
    goes_red(&rig.fleet, 1815);
    rig.fleet.turn().await.unwrap();
    rig.a_terminal_session("s-a").await;
    rig.takes_the_red("s-a").await.expect("taken");

    // The red comes back with a Job's pull request named as its cause, as a later reading may find.
    let mut main = kept(&rig.fleet).await.unwrap();
    main.merge.as_mut().expect("a merge").job = Some(culprit.id().clone());
    rig.fleet.main_acted_on(&[MainChanged { change: MainChange::BackToRed, now: main }]).await;
    assert_eq!(takes(&rig.fleet).await.len(), 1, "no Job was sent the work");
    assert_eq!(rig.session_takes().await.len(), 1);
    assert_eq!(rig.jobs_made().await, 1, "only the culprit exists");
}

#[tokio::test]
async fn green_ends_a_sessions_take_as_it_ends_a_jobs() {
    let rig = local_rig().await;
    let id = rig.a_hosted_session().await;
    rig.held_pull(&id, 1815).await;
    goes_red(&rig.fleet, 1815);
    rig.turned().await;
    assert!(the_hub(&rig.fleet).await.fixing_session.is_some());
    let [take] = takes(&rig.fleet).await.try_into().expect("one take");
    assert!(take.ended_at.is_none());

    let forge = &rig.fleet.vcs().main_ci;
    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    rig.fleet.turn().await.unwrap();
    assert!(the_hub(&rig.fleet).await.fixing_session.is_none());
    let ended = rig.fleet.store().lock().await.main_fixes_of(&take.repository, &take.red_at).unwrap();
    assert!(matches!(ended.as_slice(), [one] if one.ended_at.is_some() && one.fixed_in.is_none()), "{ended:?}");
}
