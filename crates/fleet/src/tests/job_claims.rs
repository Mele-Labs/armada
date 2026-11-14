//! A live Job's claims in its project's store, with two Macs on one store and one beads remote: a
//! second Job on a held item waits naming the first and carries on once it closes, a person's beads
//! item is never taken, two Jobs on one file are both told and both carry on, a Job keeps working
//! while the store is down and its claims go out when it answers, and a renewal is one call a Job
//! every five minutes, taken again once its claims lapsed.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{Backlog, ClaimKind, Store, Tracker};
use api::{HostedSessions, Sessions};
use ipc::{ClaimWaitEnds, IssueReference, SessionId, SessionRow};
use testkit::{FakeBeads, FakeBeadsRemote, FakeStore, FakeTrackers};

use crate::tests::backlog::BEADS;
use crate::tests::planted::Held;
use crate::tests::red_claim::{in_a_project_storing_at, Planted, SLUG};
use crate::tests::session_host::{rig_fitted, Rig};

/// One Mac: its Fleet on a clock the test moves, its view of the shared store, and its beads copy.
struct Mac {
    rig: Rig,
    clock: Arc<Held>,
    view: Arc<FakeStore>,
    copy: Arc<FakeBeads>,
}

async fn mac(name: &str, shared: &FakeStore, remote: &FakeBeadsRemote) -> Mac {
    let (view, copy, clock) = (Arc::new(shared.view()), Arc::new(FakeBeads::on(remote)), Arc::new(Held::started()));
    let (planted, beads, ticking) = (Arc::clone(&view), Arc::clone(&copy), Arc::clone(&clock));
    let rig = rig_fitted(move |fittings| {
        fittings.trackers = Arc::new(FakeTrackers::empty().beads_named("beads", beads));
        fittings.starting().manifest = config::Manifest::parse(std::path::Path::new("armada.yml"), BEADS).expect("loads");
        fittings.reaching = Arc::new(Planted::with_store(planted));
        fittings.clock = ticking;
    });
    crate::profiles::Profiles::in_machine_directory(rig.home())
        .join(&adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() }, crate::profiles::Orgs::Read(vec!["mele-labs".into()]), "2026-10-14T09:00:00.000Z")
        .expect("a profile");
    let changes = [("machine.name".to_string(), Some(ipc::SettingValue::Text(name.to_string())))].into_iter().collect();
    api::Settings::save_settings(&*rig.fleet, ipc::SaveSettings { changes }).await.expect("saved");
    in_a_project_storing_at(&rig.fleet, "https://store.test").await;
    Mac { rig, clock, view, copy }
}

impl Mac {
    async fn job(&self, title: &str) -> SessionId {
        self.rig.start_titled(Some(title)).await
    }

    /// The Job works `item`, as a person's words or its dispatch name it.
    async fn works(&self, job: &SessionId, item: &str) {
        let named = BTreeMap::from([(IssueReference::read(item).expect("a key"), BTreeMap::new())]);
        self.rig.fleet.issues_held(job.as_str(), self.rig.manifest.as_str(), named).await;
    }

    /// Fleet saw the Job write `path`.
    fn writes(&self, job: &SessionId, path: &str) {
        let now = self.rig.fleet.now();
        self.rig.fleet.writes().wrote(&crate::clashing::Who::Session(job.as_str().to_string()), self.rig.manifest.as_str(), [path.to_string()], &now);
    }

    /// A pass, `seconds` after the last.
    async fn pass(&self, seconds: u64) {
        self.clock.on(seconds);
        self.rig.fleet.claims_passed().await;
    }

    async fn record(&self, job: &SessionId) -> ipc::SessionRecord {
        self.rig.fleet.list_sessions(None, Some(job.as_str().into()), None).await.expect("listed").sessions.remove(0)
    }

    /// What Fleet told the Job of its claims, oldest first.
    async fn told(&self, job: &SessionId) -> Vec<String> {
        let rows = Arc::clone(&self.rig.fleet).get_session(job.clone()).await.expect("read").rows;
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { text, notice: Some(notice), .. } if notice == "claims" => Some(text),
                _ => None,
            })
            .collect()
    }
}

fn holder_of(store: &FakeStore, kind: ClaimKind, named: &str) -> Option<String> {
    let repository = crate::red_claim::repository_key_of(SLUG).expect("a key");
    let key = crate::job_claims::key_of(&crate::job_claims::Wanted { kind, named: named.to_string() }, SLUG).expect("a key");
    store.read_claim(&repository, kind, &key).expect("read").map(|held| held.holder.id)
}

#[tokio::test]
async fn a_second_job_on_a_held_item_waits_naming_the_first_and_carries_on_once_it_closes() {
    let (shared, remote) = (FakeStore::new(), FakeBeadsRemote::new());
    let (home, work) = (mac("home", &shared, &remote).await, mac("work", &shared, &remote).await);
    let item = home.copy.create("Fix the flaky queue test");
    home.copy.sync("/").expect("pushed");
    work.copy.sync("/").expect("pulled");
    let first = home.job("Fix the flaky queue test").await;
    let second = work.job("Land the queue race fix").await;
    home.works(&first, &item).await;
    work.works(&second, &item).await;

    home.pass(1).await;
    work.pass(1).await;

    assert_eq!(holder_of(&shared, ClaimKind::Item, &item).as_deref(), Some(first.as_str()));
    let waits = work.record(&second).await.claim_waits;
    let [wait] = waits.as_slice() else { panic!("one wait: {waits:?}") };
    assert_eq!((wait.item.as_str(), wait.holder.as_str()), (item.as_str(), "octocat"));
    assert_eq!((wait.machine.as_deref(), wait.title.as_deref()), (Some("home"), Some("Fix the flaky queue test")));
    assert_eq!(wait.ends, ClaimWaitEnds::ClaimEnds);
    let told = work.told(&second).await;
    assert!(told.len() == 1 && told[0].contains("Job \"Fix the flaky queue test\" on octocat's Mac home"), "{told:?}");
    assert!(home.record(&first).await.claim_waits.is_empty(), "the holder waits on nothing");

    home.copy.close_tree("/", &item, "fixed").expect("closed");
    home.rig.fleet.issue_ledger_kept_current(&home.rig.manifest, &home.rig.root).await;
    home.pass(60).await;
    assert_eq!(holder_of(&shared, ClaimKind::Item, &item), None, "the closed item's claim ends");

    work.pass(300).await;
    assert!(work.record(&second).await.claim_waits.is_empty(), "the wait ends");
    let told = work.told(&second).await;
    assert_eq!(told.last().map(String::as_str), Some(format!("{item} is free now. Carry on with it.").as_str()), "{told:?}");
}

#[tokio::test]
async fn an_item_assigned_to_a_person_in_beads_is_not_taken() {
    let (shared, remote) = (FakeStore::new(), FakeBeadsRemote::new());
    let home = mac("home", &shared, &remote).await;
    let item = home.copy.create("His own work");
    home.copy.claim("/", &item, "nick").expect("assigned to him");
    let open = home.copy.open_issues("/").expect("listed");
    assert!(!open.iter().filter(|one| one.reference == item).any(crate::job_claims::no_person_holds), "Autopilot leaves it");

    let job = home.job("Pick up his item").await;
    home.works(&job, &item).await;
    home.rig.fleet.issue_ledger_kept_current(&home.rig.manifest, &home.rig.root).await;
    home.pass(1).await;

    assert_eq!(holder_of(&shared, ClaimKind::Item, &item), None, "nothing is claimed for it");
    let waits = home.record(&job).await.claim_waits;
    let [wait] = waits.as_slice() else { panic!("one wait: {waits:?}") };
    assert_eq!((wait.holder.as_str(), wait.machine.as_ref(), wait.ends), ("nick", None, ClaimWaitEnds::Unassigned));
    assert!(home.told(&job).await[0].contains("assigned to nick in beads"));

    home.copy.release("/", &item, "nick", "").expect("unassigned");
    home.rig.fleet.issue_ledger_kept_current(&home.rig.manifest, &home.rig.root).await;
    home.pass(60).await;
    assert!(home.record(&job).await.claim_waits.is_empty());
    assert_eq!(holder_of(&shared, ClaimKind::Item, &item).as_deref(), Some(job.as_str()), "taken once he let it go");
}

#[tokio::test]
async fn two_jobs_writing_one_file_are_both_told_and_both_carry_on() {
    let (shared, remote) = (FakeStore::new(), FakeBeadsRemote::new());
    let (home, work) = (mac("home", &shared, &remote).await, mac("work", &shared, &remote).await);
    let first = home.job("Tidy the log").await;
    let second = work.job("Quiet the log").await;
    home.writes(&first, "src/log.rs");
    work.writes(&second, "src/log.rs");

    home.pass(1).await;
    work.pass(1).await;
    home.pass(300).await;

    let to_second = work.told(&second).await;
    assert!(to_second.len() == 1 && to_second[0].contains("Job \"Tidy the log\" on octocat's Mac home also writes `src/log.rs`"), "{to_second:?}");
    let to_first = home.told(&first).await;
    assert!(to_first.len() == 1 && to_first[0].contains("Job \"Quiet the log\" on octocat's Mac work also writes `src/log.rs`"), "{to_first:?}");
    for (mac, job) in [(&home, &first), (&work, &second)] {
        let record = mac.record(job).await;
        assert!(record.claim_waits.is_empty(), "a file is never waited on");
        assert_eq!(record.state, ipc::SessionState::Live);
    }
}

#[tokio::test]
async fn a_job_keeps_working_with_the_store_down_and_its_claims_go_out_when_it_answers() {
    let (shared, remote) = (FakeStore::new(), FakeBeadsRemote::new());
    let home = mac("home", &shared, &remote).await;
    let job = home.job("Work offline").await;
    home.writes(&job, "src/a.rs");
    home.view.set_unreachable(true);

    home.pass(1).await;
    let record = home.record(&job).await;
    assert!(record.claims_unsent, "its record says its claims are not sent");
    assert_eq!(record.state, ipc::SessionState::Live);
    assert!(home.told(&job).await.is_empty(), "nothing stops it");
    assert_eq!(holder_of(&shared, ClaimKind::File, "src/a.rs"), None);
    home.writes(&job, "src/b.rs");

    home.view.set_unreachable(false);
    home.pass(60).await;
    assert!(!home.record(&job).await.claims_unsent);
    for path in ["src/a.rs", "src/b.rs"] {
        assert_eq!(holder_of(&shared, ClaimKind::File, path).as_deref(), Some(job.as_str()), "{path} sent once it answered");
    }
}

/// The store drops a claim 30 minutes after its holder's last renewal: `apps/store/test/store.test.ts`,
/// *the lapse*, proves it. Here a lapse is the claim gone from the store, and the renewal reading `0`.
#[tokio::test]
async fn a_renewal_is_one_call_a_job_every_five_minutes_and_lapsed_claims_are_taken_again() {
    let (shared, remote) = (FakeStore::new(), FakeBeadsRemote::new());
    let home = mac("home", &shared, &remote).await;
    let job = home.job("Hold two files").await;
    home.writes(&job, "src/a.rs");
    home.writes(&job, "src/b.rs");
    let renewals = || home.view.calls().into_iter().filter(|call| *call == "renew").count();

    home.pass(1).await;
    assert_eq!(renewals(), 1, "one call renews both claims");
    for _ in 0..3 {
        home.pass(60).await;
    }
    assert_eq!(renewals(), 1, "none within five minutes");
    home.pass(120).await;
    assert_eq!(renewals(), 2, "one at five minutes");

    let other = shared.view();
    let repository = crate::red_claim::repository_key_of(SLUG).expect("a key");
    for path in ["src/a.rs", "src/b.rs"] {
        let key = crate::job_claims::key_of(&crate::job_claims::Wanted::file(path), SLUG).expect("a key");
        other.release_claim(&repository, ClaimKind::File, &key).expect("lapsed");
    }
    home.pass(300).await;
    home.pass(60).await;
    for path in ["src/a.rs", "src/b.rs"] {
        assert_eq!(holder_of(&shared, ClaimKind::File, path).as_deref(), Some(job.as_str()), "{path} taken again");
    }
}
