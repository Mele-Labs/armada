//! His other Macs through this Mac's Phone Gateway: only a shared Job, its thread, its live changes and its
//! report go out; what another Mac shares is kept with that Mac's id, listed beside this Mac's own, never
//! sent back out, and dropped when a replace leaves it out or the Mac is forgotten.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::{HostedSessions, Next, OtherMacs, Refusal, Sessions, Subscription};
use ipc::{
    BugReport, BugReports, Event, JobThread, JobThreadChanged, JobThreadRead, MacNamed, OtherMacBugReports, OtherMacEvent,
    OtherMacSessions, PhoneGateway, ReportBug, SessionId, SessionRecord, SessionState, WireError,
};

use super::session_host::{rig_placing, Rig};
use super::share::{job_in, project, shared};
use crate::phone::{OtherMacRead, PhoneSwitch};

const WORK: &str = "mac-work";

async fn record_of(rig: &Rig, job: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session
}

/// A row another Mac holds, shaped as this Mac's `template` with its own id and title.
fn theirs(template: &SessionRecord, id: &str, title: &str) -> SessionRecord {
    let mut record = template.clone();
    record.id = SessionId::carried(id);
    record.title = Some(title.to_string());
    record
}

async fn reported(rig: &Rig, job: &SessionId) {
    let report = ReportBug {
        title: "Refresh drops the lease".into(),
        what: "It quotes private work.".into(),
        steps: Vec::new(),
        errors: Vec::new(),
        logs: Vec::new(),
        session_id: Some(job.clone()),
    };
    rig.fleet.report_bug(None, report).await.expect("reported");
}

async fn hold(rig: &Rig, sessions: Vec<SessionRecord>) {
    rig.fleet.hold_other_mac_sessions(OtherMacSessions { mac: WORK.into(), sessions }).await.expect("held");
}

fn read(job: &SessionId) -> JobThreadRead {
    JobThreadRead { session_id: job.clone(), before: None, limit: None, upto: None }
}

fn ids(sessions: &[SessionRecord]) -> Vec<&str> {
    sessions.iter().map(|one| one.id.as_str()).collect()
}

/// What a refusal says, with the id it names taken out, so two refusals can be compared whole.
fn said(refused: &Refusal, named: &str) -> (u16, String, String) {
    (refused.status(), refused.error().code.clone(), refused.error().message.replace(named, "<id>"))
}

/// Every event already sent. **Read after the call that sent them returns**, so nothing races the read.
async fn sent(watching: &mut Subscription) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(next)) = tokio::time::timeout(Duration::from_millis(100), watching.next()).await {
        if let Next::Send(delivered) = next {
            events.push(delivered.event);
        }
    }
    events
}

#[tokio::test]
async fn an_unshared_job_and_its_report_never_go_out_and_a_shared_one_does() {
    let rig = rig_placing(None);
    let _keeping = crate::thread_keeping::keep_the_threads(Arc::clone(&rig.fleet));
    let shop = project(&rig, "Shop").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;
    reported(&rig, &in_shop).await;
    reported(&rig, &in_none).await;
    let nobody = SessionId::carried("01NOSUCHJOB0000000000000000");
    let missing = Arc::clone(&rig.fleet).share_job_thread(read(&nobody)).await.expect_err("no such Job");

    assert!(rig.fleet.share_sessions().await.expect("listed").sessions.is_empty(), "no row of an unshared project");
    assert!(rig.fleet.share_bug_reports().await.expect("listed").reports.is_empty(), "no report of one");
    for job in [&in_shop, &in_none] {
        let row = Event::SessionChanged(record_of(&rig, job).await);
        assert!(rig.fleet.share_event(row).await.is_none(), "no live change of one");
        let refused = Arc::clone(&rig.fleet).share_job_thread(read(job)).await.expect_err("no thread of one");
        assert_eq!(said(&refused, job.as_str()), said(&missing, nobody.as_str()), "refused as a Job that does not exist");
    }

    shared(&rig, &shop, true).await;
    assert_eq!(ids(&rig.fleet.share_sessions().await.expect("listed").sessions), [in_shop.as_str()]);
    let reports = rig.fleet.share_bug_reports().await.expect("listed").reports;
    assert_eq!(reports.iter().map(|one| one.session_id.as_str()).collect::<Vec<_>>(), [in_shop.as_str()]);
    assert!(rig.fleet.share_event(Event::SessionChanged(record_of(&rig, &in_shop).await)).await.is_some());
    let thread = Arc::clone(&rig.fleet).share_job_thread(read(&in_shop)).await.expect("a shared Job's thread");
    assert_eq!(thread.session_id, in_shop);
    let refused = Arc::clone(&rig.fleet).share_job_thread(read(&in_none)).await.expect_err("a Job in no project");
    assert_eq!(said(&refused, in_none.as_str()), said(&missing, nobody.as_str()));
}

#[tokio::test]
async fn turning_a_projects_share_off_drops_its_job_from_the_next_list() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    shared(&rig, &shop, true).await;
    assert_eq!(ids(&rig.fleet.share_sessions().await.expect("listed").sessions), [in_shop.as_str()]);

    shared(&rig, &shop, false).await;
    assert!(rig.fleet.share_sessions().await.expect("listed").sessions.is_empty());
}

#[tokio::test]
async fn another_macs_rows_are_listed_with_their_mac_and_go_when_left_out_or_forgotten() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync"), theirs(&template, "01WORKB", "Write the notes")]).await;

    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    let from: Vec<(&str, Option<&str>)> = listed.iter().map(|one| (one.id.as_str(), one.mac.as_deref())).collect();
    assert!(from.contains(&("01WORKA", Some(WORK))) && from.contains(&("01WORKB", Some(WORK))), "{from:?}");
    assert!(from.contains(&(mine.as_str(), None)), "this Mac's own row has no Mac: {from:?}");

    let found = rig.fleet.list_sessions(None, Some("sync".into()), None).await.expect("listed").sessions;
    assert_eq!(ids(&found), ["01WORKA"], "the same text filter reads their rows");
    let other = if template.state == SessionState::Ended { SessionState::Live } else { SessionState::Ended };
    let found = rig.fleet.list_sessions(None, None, Some(other)).await.expect("listed").sessions;
    assert!(found.iter().all(|one| one.mac.is_none()), "and the same state filter");

    hold(&rig, vec![theirs(&template, "01WORKB", "Write the notes")]).await;
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    assert!(!ids(&listed).contains(&"01WORKA"), "a replace that leaves a row out drops it");
    assert!(ids(&listed).contains(&"01WORKB"));

    rig.fleet.forget_other_mac(MacNamed { mac: WORK.into() }).await.expect("forgot");
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    assert!(listed.iter().all(|one| one.mac.is_none()), "forgetting the Mac drops its rows");
    assert!(ids(&listed).contains(&mine.as_str()));
}

#[tokio::test]
async fn what_another_mac_shared_never_goes_back_out() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    shared(&rig, &shop, true).await;
    let mine = job_in(&rig, Some(&shop)).await;
    reported(&rig, &mine).await;
    let template = record_of(&rig, &mine).await;
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    let mut report = rig.fleet.list_bug_reports().await.expect("listed").reports.remove(0);
    report.id = "work-report".into();
    report.session_id = "01WORKA".into();
    rig.fleet.hold_other_mac_bug_reports(OtherMacBugReports { mac: WORK.into(), reports: vec![report] }).await.expect("held");

    assert_eq!(ids(&rig.fleet.share_sessions().await.expect("listed").sessions), [mine.as_str()]);
    let reports = rig.fleet.share_bug_reports().await.expect("listed").reports;
    assert_eq!(reports.iter().map(|one| one.session_id.as_str()).collect::<Vec<_>>(), [mine.as_str()]);

    let held = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    let held = held.into_iter().find(|one| one.id.as_str() == "01WORKA").expect("held");
    assert!(rig.fleet.share_event(Event::SessionChanged(held)).await.is_none(), "its live changes stay");
    let moved = Event::JobThreadChanged(JobThreadChanged { session_id: SessionId::carried("01WORKA"), from: "r1".into() });
    assert!(rig.fleet.share_event(moved).await.is_none());
    let every = Event::BugReportsChanged(rig.fleet.list_bug_reports().await.expect("listed"));
    let Some(Event::BugReportsChanged(crossing)) = rig.fleet.share_event(every).await else {
        panic!("bug_reports.changed crosses, cut");
    };
    assert_eq!(crossing.reports.iter().map(|one| one.id.as_str()).collect::<Vec<_>>().len(), 1);
    assert!(crossing.reports.iter().all(|one| one.mac.is_none()), "{crossing:?}");
    let refused = Arc::clone(&rig.fleet).share_job_thread(read(&SessionId::carried("01WORKA"))).await.expect_err("not this Mac's");
    assert_eq!(refused.status(), 404);
}

#[tokio::test]
async fn what_another_mac_says_of_its_jobs_reaches_bridge_and_never_fleets_own_listeners() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    let heard = Arc::new(Mutex::new(0));
    let into = Arc::clone(&heard);
    rig.fleet.events().tap(move |event| {
        let about_theirs = match event {
            Event::SessionChanged(record) => record.mac.is_some(),
            Event::JobThreadChanged(changed) => changed.session_id.as_str().starts_with("01WORK"),
            _ => false,
        };
        if about_theirs {
            *into.lock().expect("not poisoned") += 1;
        }
    });
    let mut watching = rig.fleet.events().subscribe();

    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    let told = sent(&mut watching).await;
    assert!(told.iter().any(|one| matches!(one, Event::SessionChanged(row) if row.id.as_str() == "01WORKA" && row.mac.as_deref() == Some(WORK))));
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    assert!(!sent(&mut watching).await.iter().any(|one| matches!(one, Event::SessionChanged(_))), "an unchanged row is not sent again");

    let changed = Event::SessionChanged(theirs(&template, "01WORKA", "Fix the sync for good"));
    rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: changed }).await.expect("kept");
    assert!(sent(&mut watching).await.iter().any(|one| matches!(one, Event::SessionChanged(row) if row.title.as_deref() == Some("Fix the sync for good"))));
    let listed = rig.fleet.list_sessions(None, Some("for good".into()), None).await.expect("listed").sessions;
    assert_eq!(ids(&listed), ["01WORKA"], "kept, not only passed on");

    let moved = |job: &str| Event::JobThreadChanged(JobThreadChanged { session_id: SessionId::carried(job), from: "r1".into() });
    rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: moved("01WORKA") }).await.expect("passed on");
    assert!(sent(&mut watching).await.contains(&moved("01WORKA")));
    let unheld = rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: moved("01WORKZ") }).await.expect_err("not held");
    assert_eq!(unheld.status(), 404);
    let elsewhere = rig.fleet.hold_other_mac_event(OtherMacEvent { mac: "mac-home".into(), event: moved("01WORKA") }).await;
    assert_eq!(elsewhere.expect_err("held from another Mac").status(), 404);

    let projects = Event::ProjectsChanged(ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() });
    rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: projects }).await.expect("taken, and nothing done");
    let studio = Event::StudioDeleted(ipc::StudioDeleted { id: ipc::StudioId::carried("01S"), manifest_id: rig.manifest.clone() });
    let refused = rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: studio }).await.expect_err("not a kind that crosses");
    assert_eq!(refused.status(), 422);
    assert!(refused.error().message.ends_with('.'), "{}", refused.error().message);

    assert_eq!(*heard.lock().expect("not poisoned"), 0, "Fleet's own listeners heard none of it");
}

/// The ids of every `session.removed` among `events`, in order.
fn removed(events: &[Event]) -> Vec<&str> {
    events.iter().filter_map(|one| if let Event::SessionRemoved(gone) = one { Some(gone.session_id.as_str()) } else { None }).collect()
}

#[tokio::test]
async fn a_row_another_mac_drops_reaches_bridge_as_removed_and_never_fleets_own_listeners() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    let heard = Arc::new(Mutex::new(0));
    let into = Arc::clone(&heard);
    rig.fleet.events().tap(move |event| {
        if matches!(event, Event::SessionRemoved(_)) {
            *into.lock().expect("not poisoned") += 1;
        }
    });
    let mut watching = rig.fleet.events().subscribe();
    let three = ["01WORKA", "01WORKB", "01WORKC"].map(|id| theirs(&template, id, "Theirs"));
    hold(&rig, three.to_vec()).await;
    assert!(removed(&sent(&mut watching).await).is_empty(), "nothing went");

    hold(&rig, vec![three[1].clone()]).await;
    assert_eq!(removed(&sent(&mut watching).await), ["01WORKA", "01WORKC"], "exactly the rows the replace left out");

    rig.fleet.forget_other_mac(MacNamed { mac: WORK.into() }).await.expect("forgot");
    assert_eq!(removed(&sent(&mut watching).await), ["01WORKB"], "exactly the rows the Mac still had");
    rig.fleet.forget_other_mac(MacNamed { mac: WORK.into() }).await.expect("forgot again");
    assert!(removed(&sent(&mut watching).await).is_empty(), "a Mac with no rows left drops none");

    assert_eq!(*heard.lock().expect("not poisoned"), 0, "Fleet's own listeners heard none of it");
}

#[tokio::test]
async fn another_macs_bug_reports_are_listed_and_published_beside_this_macs() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    reported(&rig, &mine).await;
    let own: BugReport = rig.fleet.list_bug_reports().await.expect("listed").reports.remove(0);
    let mut theirs = own.clone();
    theirs.id = "work-report".into();
    theirs.session_id = "01WORKA".into();
    let mut watching = rig.fleet.events().subscribe();

    let held = OtherMacBugReports { mac: WORK.into(), reports: vec![theirs.clone()] };
    rig.fleet.hold_other_mac_bug_reports(held).await.expect("held");
    let macs = |reports: &BugReports| reports.reports.iter().map(|one| (one.id.clone(), one.mac.clone())).collect::<Vec<_>>();
    let listed = rig.fleet.list_bug_reports().await.expect("listed");
    assert!(macs(&listed).contains(&("work-report".into(), Some(WORK.into()))), "{listed:?}");
    assert!(macs(&listed).contains(&(own.id.clone(), None)));
    let published = |events: Vec<Event>| events.into_iter().filter_map(|one| if let Event::BugReportsChanged(reports) = one { Some(reports) } else { None }).last();
    assert_eq!(published(sent(&mut watching).await).as_ref(), Some(&listed), "published whole, both Macs'");

    reported(&rig, &mine).await;
    let after = published(sent(&mut watching).await).expect("this Mac's report published");
    assert!(macs(&after).contains(&("work-report".into(), Some(WORK.into()))), "this Mac's own change keeps theirs in the list");

    let emptied = Event::BugReportsChanged(BugReports { reports: Vec::new() });
    rig.fleet.hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: emptied }).await.expect("kept");
    assert!(rig.fleet.list_bug_reports().await.expect("listed").reports.iter().all(|one| one.mac.is_none()));

    rig.fleet.hold_other_mac_bug_reports(OtherMacBugReports { mac: WORK.into(), reports: vec![theirs] }).await.expect("held");
    sent(&mut watching).await;
    rig.fleet.forget_other_mac(MacNamed { mac: WORK.into() }).await.expect("forgot");
    let left = published(sent(&mut watching).await).expect("published on forget");
    assert!(left.reports.iter().all(|one| one.mac.is_none()));
    assert_eq!(rig.fleet.list_bug_reports().await.expect("listed"), left);
}

/// The Gateway as a stand-in: what Fleet asked of which Mac, and one answer for every read.
struct Link {
    asked: Mutex<Vec<(String, String)>>,
    answer: Mutex<(u16, Vec<u8>)>,
}

impl PhoneSwitch for Link {
    fn state(&self) -> PhoneGateway {
        PhoneGateway { on: true, port: 8443, listening: true, why: None }
    }

    fn set(&self, _on: bool) -> PhoneGateway {
        self.state()
    }

    fn read_other_mac(&self, mac: &str, path: &str) -> OtherMacRead {
        self.asked.lock().expect("not poisoned").push((mac.to_string(), path.to_string()));
        let answer = self.answer.lock().expect("not poisoned").clone();
        Box::pin(async move { Ok(answer) })
    }
}

#[tokio::test]
async fn a_thread_of_a_job_held_from_another_mac_is_read_through_the_link() {
    let rig = rig_placing(None);
    let _keeping = crate::thread_keeping::keep_the_threads(Arc::clone(&rig.fleet));
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    let mut thread: JobThread = Arc::clone(&rig.fleet).get_job_thread(read(&mine)).await.expect("this Mac's thread");
    thread.session_id = SessionId::carried("01WORKA");
    let link = Arc::new(Link { asked: Mutex::new(Vec::new()), answer: Mutex::new((200, ipc::encode(&thread).expect("encodes").into_bytes())) });
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);

    let mut asked = read(&SessionId::carried("01WORKA"));
    asked.before = Some("row 1".into());
    let read_there = Arc::clone(&rig.fleet).get_job_thread(asked.clone()).await.expect("read through the link");
    assert_eq!(read_there, thread);
    assert_eq!(*link.asked.lock().expect("not poisoned"), [(WORK.to_string(), "/other_mac/thread?session_id=01WORKA&before=row%201".to_string())]);

    let theirs = WireError::raised("fleet.no_such_session", "No Job Fleet holds is named 01WORKA.", ipc::RunId::carried("01THEIRRUN"));
    *link.answer.lock().expect("not poisoned") = (404, ipc::encode(&theirs).expect("encodes").into_bytes());
    let refused = Arc::clone(&rig.fleet).get_job_thread(asked).await.expect_err("refused there");
    assert_eq!((refused.status(), refused.error().code.as_str(), refused.error().message.as_str()), (404, "fleet.no_such_session", "No Job Fleet holds is named 01WORKA."));

    Arc::clone(&rig.fleet).get_job_thread(read(&mine)).await.expect("this Mac's own thread");
    assert_eq!(link.asked.lock().expect("not poisoned").len(), 2, "this Mac's own Job is read here");
}
