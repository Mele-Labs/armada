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
    // A second project holds the repository too, so a start naming none is in none.
    project(&rig, "Yard").await;
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

fn started_on_work(rig: &Rig, row: &SessionRecord) -> (ipc::JobAct, api::ActAnswered) {
    let start = ipc::StartSession {
        project: None,
        manifest_id: rig.manifest.clone(),
        title: row.title.clone(),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    (ipc::JobAct::Start(start), api::ActAnswered { status: 200, body: ipc::encode(row).expect("encodes").into_bytes() })
}

/// The link reads Work's list, then hands it here. A start answered between the two must not be undone by that list.
#[tokio::test]
async fn a_job_started_on_another_mac_outlasts_the_one_list_read_before_the_start_and_no_more() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    let listed = || async { ids(&rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions).iter().map(|id| id.to_string()).collect::<Vec<_>>() };

    let (start, answered) = started_on_work(&rig, &theirs(&template, "01WORKS", "Tape the box"));
    rig.fleet.act_answered(WORK, &start, answered).await;
    hold(&rig, Vec::new()).await;
    assert!(listed().await.contains(&"01WORKS".to_string()), "the list read before the start leaves the new row standing");
    hold(&rig, Vec::new()).await;
    assert!(!listed().await.contains(&"01WORKS".to_string()), "a list read after it is Work's word, and drops it");

    // Once Work has spoken of the Job, the next list is read after the start, so it holds the row no longer.
    let (start, answered) = started_on_work(&rig, &theirs(&template, "01WORKT", "Seal the box"));
    rig.fleet.act_answered(WORK, &start, answered).await;
    let changed = Event::SessionChanged(theirs(&template, "01WORKT", "Seal the box"));
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: changed }).await.expect("kept");
    hold(&rig, Vec::new()).await;
    assert!(!listed().await.contains(&"01WORKT".to_string()));
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
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: changed }).await.expect("kept");
    assert!(sent(&mut watching).await.iter().any(|one| matches!(one, Event::SessionChanged(row) if row.title.as_deref() == Some("Fix the sync for good"))));
    let listed = rig.fleet.list_sessions(None, Some("for good".into()), None).await.expect("listed").sessions;
    assert_eq!(ids(&listed), ["01WORKA"], "kept, not only passed on");

    let moved = |job: &str| Event::JobThreadChanged(JobThreadChanged { session_id: SessionId::carried(job), from: "r1".into() });
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: moved("01WORKA") }).await.expect("passed on");
    assert!(sent(&mut watching).await.contains(&moved("01WORKA")));
    let unheld = Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: moved("01WORKZ") }).await.expect_err("not held");
    assert_eq!(unheld.status(), 404);
    let elsewhere = Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: "mac-home".into(), event: moved("01WORKA") }).await;
    assert_eq!(elsewhere.expect_err("held from another Mac").status(), 404);

    let projects = Event::ProjectsChanged(ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new(), left_behind: Vec::new() });
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: projects }).await.expect("taken, and nothing done");
    let studio = Event::StudioDeleted(ipc::StudioDeleted { id: ipc::StudioId::carried("01S"), manifest_id: rig.manifest.clone() });
    let refused = Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: studio }).await.expect_err("not a kind that crosses");
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
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: emptied }).await.expect("kept");
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

#[tokio::test]
async fn the_artifacts_of_a_job_held_from_another_mac_are_read_through_the_link() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let scene = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
    let drawn = format!(r#"{{"title":"Request flow","question":"Does this read right?","session_id":"{}","scene":{scene}}}"#, mine.as_str());
    rig.fleet.show_sketch(None, ipc::decode("a sketch to show", drawn.as_bytes()).expect("a sketch")).await.expect("drawn");
    let kept = rig.fleet.get_job_artifacts(mine.clone()).await.expect("this Mac's own list");
    assert_eq!(kept.len(), 1);
    let template = record_of(&rig, &mine).await;
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    let link = Arc::new(Link { asked: Mutex::new(Vec::new()), answer: Mutex::new((200, ipc::encode(&kept).expect("encodes").into_bytes())) });
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    let held = SessionId::carried("01WORKA");

    let listed = rig.fleet.get_job_artifacts(held.clone()).await.expect("read through the link");
    assert_eq!(listed, kept, "what Work lists is what Bridge is given");

    let page = ipc::ArtifactPageShared { revision: 2, html: "<html>sealed</html>".into(), policy: "default-src 'none'".into() };
    *link.answer.lock().expect("not poisoned") = (200, ipc::encode(&page).expect("encodes").into_bytes());
    let sealed = rig.fleet.get_artifact_page(held.clone(), "/doc/lease.html".into(), Some(2)).await.expect("the page, through the link");
    assert_eq!((sealed.revision, sealed.html.as_str(), sealed.policy.as_str()), (2, "<html>sealed</html>", "default-src 'none'"));
    let asked = link.asked.lock().expect("not poisoned").clone();
    assert_eq!(
        asked,
        [
            (WORK.to_string(), "/other_mac/artifacts?session_id=01WORKA".to_string()),
            (WORK.to_string(), "/other_mac/artifact_page?session_id=01WORKA&artifact=%2Fdoc%2Flease.html&revision=2".to_string()),
        ]
    );

    let theirs = WireError::raised("fleet.artifact_not_held", "this job holds no artifact `/doc/lease.html`.", ipc::RunId::carried("01THEIRRUN"));
    *link.answer.lock().expect("not poisoned") = (422, ipc::encode(&theirs).expect("encodes").into_bytes());
    let refused = rig.fleet.get_artifact_page(held, "/doc/lease.html".into(), None).await.expect_err("refused there");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.artifact_not_held"), "what Work refuses is passed on");

    assert_eq!(rig.fleet.get_job_artifacts(mine).await.expect("own"), kept);
    assert_eq!(link.asked.lock().expect("not poisoned").len(), 3, "this Mac's own Job is read here");
}

#[tokio::test]
async fn a_jobs_artifacts_go_out_only_for_a_shared_job_and_are_refused_as_for_no_job() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    project(&rig, "Yard").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;
    let nobody = SessionId::carried("01NOSUCHJOB0000000000000000");
    let missing = rig.fleet.share_job_artifacts(nobody.clone()).await.expect_err("no such Job");
    let missing_page = rig.fleet.share_job_artifact_page(nobody.clone(), "/a.html".into(), None).await.expect_err("no such Job");
    assert_eq!((missing.status(), missing.error().code.as_str()), (404, "fleet.no_such_session"));

    for job in [&in_shop, &in_none] {
        let refused = rig.fleet.share_job_artifacts(job.clone()).await.expect_err("not shared");
        assert_eq!(said(&refused, job.as_str()), said(&missing, nobody.as_str()), "refused as a Job that does not exist");
        let refused = rig.fleet.share_job_artifact_page(job.clone(), "/a.html".into(), None).await.expect_err("not shared");
        assert_eq!(said(&refused, job.as_str()), said(&missing_page, nobody.as_str()));
    }

    shared(&rig, &shop, true).await;
    assert!(rig.fleet.share_job_artifacts(in_shop.clone()).await.expect("a shared Job's list").is_empty());
    let refused = rig.fleet.share_job_artifact_page(in_shop, "/a.html".into(), None).await.expect_err("it holds no such page");
    assert_eq!((refused.status(), refused.error().code.as_str()), (422, "fleet.artifact_not_held"), "past the share filter, the page's own refusal");
    let refused = rig.fleet.share_job_artifacts(in_none.clone()).await.expect_err("still none");
    assert_eq!(said(&refused, in_none.as_str()), said(&missing, nobody.as_str()));
}

#[tokio::test]
async fn a_held_jobs_row_and_window_are_read_from_the_mac_that_holds_it() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    rig.fleet.fleet_note(mine.as_str(), "Work has begun".into()).await;
    let template = record_of(&rig, &mine).await;
    hold(&rig, vec![theirs(&template, "01WORKA", "Fix the sync")]).await;
    // What Work answers: its own row, which has no `mac` there, and its newest rows with more behind them.
    let mut at_work = Arc::clone(&rig.fleet).get_session(mine.clone()).await.expect("a thread");
    at_work.session.id = SessionId::carried("01WORKA");
    at_work.older = true;
    let link = Arc::new(Link { asked: Mutex::new(Vec::new()), answer: Mutex::new((200, ipc::encode(&at_work).expect("encodes").into_bytes())) });
    rig.fleet.switching_phone_with(Arc::clone(&link) as Arc<dyn PhoneSwitch>);
    let held = SessionId::carried("01WORKA");

    let opened = Arc::clone(&rig.fleet).get_session(held.clone()).await.expect("read through the link");
    assert_eq!(opened.rows, at_work.rows, "Work's rows");
    assert_eq!(opened.session.mac.as_deref(), Some(WORK), "the row names the Mac that holds it, as the list does");

    let window = ipc::SessionWindow { last: 3, before: Some("row 1".into()) };
    let cut = Arc::clone(&rig.fleet).get_session_window(held.clone(), window.clone()).await.expect("a window, through the link");
    assert!(cut.older, "Work said more stand before it");
    assert_eq!(
        *link.asked.lock().expect("not poisoned"),
        [
            (WORK.to_string(), "/other_mac/session?session_id=01WORKA".to_string()),
            (WORK.to_string(), "/other_mac/session?session_id=01WORKA&last=3&before=row%201".to_string()),
        ],
        "the window is asked of Work, which cuts it, and not cut here from the whole"
    );

    let theirs = WireError::raised("fleet.no_such_session", "No Job Fleet holds is named 01WORKA.", ipc::RunId::carried("01THEIRRUN"));
    *link.answer.lock().expect("not poisoned") = (404, ipc::encode(&theirs).expect("encodes").into_bytes());
    let refused = Arc::clone(&rig.fleet).get_session(held).await.expect_err("refused there");
    assert_eq!((refused.status(), refused.error().code.as_str()), (404, "fleet.no_such_session"), "what Work refuses is passed on");

    let own = Arc::clone(&rig.fleet).get_session_window(mine, window).await.expect("this Mac's own Job");
    assert_eq!(own.session.mac, None);
    assert_eq!(link.asked.lock().expect("not poisoned").len(), 3, "this Mac's own Job is read here");
}

#[tokio::test]
async fn a_jobs_row_and_window_go_out_only_for_a_shared_job_and_are_refused_as_for_no_job() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    project(&rig, "Yard").await;
    let in_shop = job_in(&rig, Some(&shop)).await;
    let in_none = job_in(&rig, None).await;
    let nobody = SessionId::carried("01NOSUCHJOB0000000000000000");
    let missing = Arc::clone(&rig.fleet).share_job_session(nobody.clone(), None).await.expect_err("no such Job");
    assert_eq!((missing.status(), missing.error().code.as_str()), (404, "fleet.no_such_session"));
    for job in [&in_shop, &in_none] {
        let refused = Arc::clone(&rig.fleet).share_job_session(job.clone(), None).await.expect_err("not shared");
        assert_eq!(said(&refused, job.as_str()), said(&missing, nobody.as_str()), "refused as a Job that does not exist");
    }

    shared(&rig, &shop, true).await;
    for note in 0..5 {
        rig.fleet.fleet_note(in_shop.as_str(), format!("Note {note}")).await;
    }
    let whole = Arc::clone(&rig.fleet).share_job_session(in_shop.clone(), None).await.expect("a shared Job");
    let (rows, window) = (whole.rows.len(), |last, before: Option<String>| ipc::SessionWindow { last, before });
    assert!(rows > 4 && !whole.older);
    let newest = Arc::clone(&rig.fleet).share_job_session(in_shop.clone(), Some(window(2, None))).await.expect("the newest");
    assert_eq!((&newest.rows[..], newest.older), (&whole.rows[rows - 2..], true), "only the window is answered");
    let before = newest.rows[0].id().to_string();
    let earlier = Arc::clone(&rig.fleet).share_job_session(in_shop.clone(), Some(window(2, Some(before)))).await.expect("the two before");
    assert_eq!((&earlier.rows[..], earlier.older), (&whole.rows[rows - 4..rows - 2], true));
    let all = Arc::clone(&rig.fleet).share_job_session(in_shop, Some(window(rows + 10, None))).await.expect("more than it holds");
    assert_eq!((all.rows, all.older), (whole.rows, false));
    let refused = Arc::clone(&rig.fleet).share_job_session(in_none.clone(), Some(window(2, None))).await.expect_err("still none");
    assert_eq!(said(&refused, in_none.as_str()), said(&missing, nobody.as_str()));
}

/// Job `job`'s row here ended as `moved_away` leaves it, the Mac it went to holding it now.
async fn moved_to_work(rig: &Rig, job: &SessionId) {
    let mut ledger = rig.fleet.store().lock().await;
    let mut row = ledger.session(job.as_str()).expect("read").expect("this Mac's row");
    row.state = store::SessionState::Ended;
    row.end_reason = Some("moved_to_mac".into());
    ledger.keep_session(&row).expect("kept");
}

#[tokio::test]
async fn a_job_that_moved_to_another_mac_is_never_sent_back_to_it() {
    let rig = rig_placing(None);
    let shop = project(&rig, "Shop").await;
    shared(&rig, &shop, true).await;
    let job = job_in(&rig, Some(&shop)).await;
    let mut there = record_of(&rig, &job).await;
    there.state = SessionState::Live;
    moved_to_work(&rig, &job).await;
    hold(&rig, vec![there.clone()]).await;

    let held = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    let held = held.into_iter().find(|one| one.id == job).expect("Work's row");
    assert_eq!((held.state, held.mac.as_deref()), (SessionState::Live, Some(WORK)), "Work's live row stands for it");
    // Each Mac relays what the other says of it, so either sending it back makes the two echo it for good.
    assert!(rig.fleet.share_event(Event::SessionChanged(held)).await.is_none(), "Work's own row went back to Work");
    let moved = Event::JobThreadChanged(JobThreadChanged { session_id: job.clone(), from: "r1".into() });
    assert!(rig.fleet.share_event(moved).await.is_none(), "Work's thread news went back to Work");

    // Ended there, the row that says so is kept and reaches Bridge, and no older one is sent to stand over it.
    let mut watching = rig.fleet.events().subscribe();
    let mut ended = there.clone();
    ended.state = SessionState::Ended;
    ended.end_reason = Some("closed".into());
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: Event::SessionChanged(ended) }).await.expect("kept");
    let told = sent(&mut watching).await;
    assert!(told.iter().any(|one| matches!(one, Event::SessionChanged(row) if row.id == job && row.state == SessionState::Ended && row.mac.as_deref() == Some(WORK))));
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    let listed = listed.into_iter().find(|one| one.id == job).expect("Work's row");
    assert_eq!((listed.state, listed.end_reason.as_deref()), (SessionState::Ended, Some("closed")));
}

#[tokio::test]
async fn another_macs_row_of_a_job_this_mac_holds_itself_is_never_kept_or_told_to_bridge() {
    let rig = rig_placing(None);
    let mine = job_in(&rig, None).await;
    let template = record_of(&rig, &mine).await;
    let mut watching = rig.fleet.events().subscribe();
    let held = || async { rig.fleet.store().lock().await.held_from_macs(store::FromMac::Sessions).expect("read") };

    // The echo of this Mac's own row, as a Mac on an older build sends it back: its copy, held from this Mac.
    let mut echoed = theirs(&template, mine.as_str(), "An older title");
    echoed.mac = Some("mac-home".into());
    let echo = Event::SessionChanged(echoed);
    Arc::clone(&rig.fleet).hold_other_mac_event(OtherMacEvent { mac: WORK.into(), event: echo }).await.expect("taken, and nothing done");
    assert!(held().await.is_empty(), "kept as Work's");
    // One an older build kept goes with the next replace, and Bridge, whose row is this Mac's own, is told nothing.
    let body = ipc::encode(&theirs(&template, mine.as_str(), "An older title")).expect("encoded");
    rig.fleet.store().lock().await.keep_from_mac(store::FromMac::Sessions, WORK, mine.as_str(), &body).expect("kept");
    hold(&rig, Vec::new()).await;
    assert!(held().await.is_empty(), "the replace kept it");

    let about_mine = |one: &Event| match one {
        Event::SessionChanged(row) => row.id == mine,
        Event::SessionRemoved(gone) => gone.session_id == mine,
        Event::JobThreadChanged(changed) => changed.session_id == mine,
        _ => false,
    };
    let told = sent(&mut watching).await;
    assert!(!told.iter().any(about_mine), "Bridge's own row was stood over or taken away: {told:?}");
}

/// The connection of the first agent the stand-in starts, pid 1000: the sender's.
fn sender_caller() -> api::Caller {
    api::Caller::at("127.0.0.1:50999".parse().unwrap())
}

#[tokio::test]
async fn a_message_to_a_job_that_moved_to_another_mac_goes_to_the_mac_holding_it() {
    let peers = crate::tests::peer::Placing::nothing();
    peers.holding(1000, 50999, 12345);
    let rig = rig_placing(Some(peers));
    let shop = project(&rig, "Shop").await;
    shared(&rig, &shop, true).await;
    let (sender, job) = (job_in(&rig, Some(&shop)).await, job_in(&rig, Some(&shop)).await);
    rig.send(&sender, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ready");
    let mut there = record_of(&rig, &job).await;
    there.state = SessionState::Live;
    moved_to_work(&rig, &job).await;
    hold(&rig, vec![there.clone()]).await;

    let to = crate::session_host::address_of(job.as_str());
    let ask = |words: &str| ipc::MessageTo { to: to.clone(), message: Some(words.to_string()) };
    let reach = Arc::clone(&rig.fleet).message_reach(sender_caller(), ask("Please review")).await.expect("answered");
    assert!(matches!(&reach, ipc::MessageReach::Queued { message } if message.contains("his other Mac")), "passed on to Work, not refused: {reach:?}");

    // A Job that really ended on Work is still refused as ended.
    let mut ended = there;
    ended.state = SessionState::Ended;
    ended.end_reason = Some("closed".into());
    hold(&rig, vec![ended]).await;
    let reach = Arc::clone(&rig.fleet).message_reach(sender_caller(), ask("Too late")).await.expect("answered");
    assert!(matches!(&reach, ipc::MessageReach::Refused { message } if message.contains("that Job has ended")), "{reach:?}");
}
