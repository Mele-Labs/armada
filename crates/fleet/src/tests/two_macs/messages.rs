//! Messages and alerts across his Macs, on loopback. A Job on Home writes to a Job Work holds, through the door
//! and queue every act on Work takes, and both threads name the other Mac. Work's shared Jobs that need him call
//! on Home; a private one never does, and a row that comes back unchanged from a sleep is no news.

use std::sync::{Arc, Mutex};

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Sessions};
use ipc::{CallsChanged, Event, JobAct, JobMessage, SessionId, SessionRow, SessionTurn, SessionVoice, SetWaitingFor, WaitingInput};

use super::{job_in, paired, project, shared, until, OneMac, Ticking};
use crate::session_host::address_of;

/// Home's Job `from` writes `text` to address `to` with `SendMessage`, as its agent calls it.
async fn wrote(home: &OneMac, process: usize, to: &str, text: &str) {
    home.rig.stand_in.says(
        process,
        vec![DroneEvent::Called { tool: "SendMessage".into(), call: format!("toolu_{text}"), detail: CallDetail::of(&format!("to {to}: {text}")) }],
    );
}

/// Job `job`'s agent on `mac` started and its first turn over, so it can call a tool.
async fn at_rest(mac: &OneMac, job: &SessionId, process: usize) {
    mac.rig.send(job, "Start").await;
    mac.rig.stand_in.init(process);
    mac.rig.stand_in.finishes(process, "Ready.");
    let rig = &mac.rig;
    until("the agent is idle", || async { rig.turn(job).await == SessionTurn::Idle }).await;
}

/// The Job messages in `job`'s thread on `mac`: who, which Mac, what.
async fn from_jobs(mac: &OneMac, job: &SessionId) -> Vec<(String, Option<String>, String)> {
    mac.rig
        .rows(job)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::Session { id, mac, .. }, text, .. } if id != "fleet" => Some((id, mac, text)),
            _ => None,
        })
        .collect()
}

async fn job_messages_run(mac: &OneMac) -> usize {
    let ran = mac.rig.fleet.store().lock().await.acts_run().expect("reads");
    ran.iter().filter(|one| one.what == "job_message").count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_messages_a_job_on_the_other_mac_and_both_threads_name_that_mac() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, mut work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let desk = project(&home.rig, "Desk").await;
    shared(&home.rig, &desk, true).await;
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let (sender, receiver) = (job_in(&home.rig, Some(&desk)).await, job_in(&work.rig, Some(&shop)).await);
    paired(&home, &work).await;
    let (on_home, on_work) = (receiver.clone(), sender.clone());
    until("each Mac lists the other's Job", || async {
        super::from(&home.sessions().await.1, &on_home).is_some() && super::from(&work.sessions().await.1, &on_work).is_some()
    })
    .await;
    at_rest(&home, &sender, 0).await;

    // Home's Job writes to Work's by its address: Work's thread holds it in the sender's voice, naming Home.
    wrote(&home, 0, &address_of(receiver.as_str()), "Please review the auth change").await;
    let (work_ref, to) = (&work, receiver.clone());
    until("Work's thread holds the message", || async { !from_jobs(work_ref, &to).await.is_empty() }).await;
    assert_eq!(from_jobs(&work, &receiver).await, [(sender.as_str().to_string(), Some(home.id.clone()), "Please review the auth change".to_string())]);

    // Home's thread names Work's Job on the row of the call, which Bridge reads the Mac of off Work's row.
    let rows = home.rig.rows(&sender).await;
    let to = rows.iter().find_map(|row| match row {
        SessionRow::Tool { message: Some(message), .. } => Some(message.to.clone()),
        _ => None,
    });
    let to = to.expect("the call's row");
    assert_eq!(to.session_id.as_ref(), Some(&receiver), "{to:?}");
    let listed = Arc::clone(&home.rig.fleet).list_sessions(None, None, None).await.expect("listed").sessions;
    assert_eq!(listed.iter().find(|one| one.id == receiver).and_then(|one| one.mac.clone()), Some(work.id.clone()));

    // Work asleep: the message waits on Home as a job_message, and goes once when Work wakes.
    work.stop().await;
    let work_id = work.id.clone();
    until("Work shows asleep on Home", || async {
        matches!(home.mac(&work_id).await.and_then(|mac| mac.link), Some(ipc::MacLink::Asleep { .. }))
    })
    .await;
    wrote(&home, 0, &address_of(receiver.as_str()), "While you slept").await;
    let home_ref = &home;
    until("the message waits for Work", || async {
        home_ref.rig.fleet.queued_listed(None).await.expect("listed").queued.iter().any(|one| matches!(&one.act, JobAct::JobMessage(sent) if sent.text == "While you slept"))
    })
    .await;
    work.restart();
    let (work_ref, to) = (&work, receiver.clone());
    until("it reached Work", || async { from_jobs(work_ref, &to).await.len() == 2 }).await;
    until("and left Home's queue", || async { home_ref.rig.fleet.queued_listed(None).await.expect("listed").queued.is_empty() }).await;
    assert_eq!(job_messages_run(&work).await, 2, "once each");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_message_into_or_out_of_a_private_job_is_not_found_and_says_nothing_of_it() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let (desk, den) = (project(&home.rig, "Desk").await, project(&home.rig, "Den").await);
    shared(&home.rig, &desk, true).await;
    let (shop, taxes) = (project(&work.rig, "Shop").await, project(&work.rig, "Taxes").await);
    shared(&work.rig, &shop, true).await;
    let (open, hidden) = (job_in(&work.rig, Some(&shop)).await, job_in(&work.rig, Some(&taxes)).await);
    let private_sender = job_in(&home.rig, Some(&den)).await;
    let _shared_sender = job_in(&home.rig, Some(&desk)).await;
    paired(&home, &work).await;
    let on_home = open.clone();
    until("Work's shared Job reaches Home", || async { super::from(&home.sessions().await.1, &on_home).is_some() }).await;
    let hidden_rows = work.rig.rows(&hidden).await.len();

    // A message sent straight at Work's private Job is answered as one to an id Work never had.
    let at = |to: &str, text: &str| JobAct::JobMessage(JobMessage { from: private_sender.clone(), title: "Den work".into(), to: SessionId::carried(to), text: text.into() });
    let sent = |act: JobAct| {
        let (home, work_id) = (&home, work.id.clone());
        async move {
            let said = home.rig.fleet.acted_on_other_mac(work_id, act).await.expect("answered, not queued");
            (said.status, String::from_utf8(said.body).expect("text"))
        }
    };
    let into_hidden = sent(at(hidden.as_str(), "Tax figures")).await;
    let into_nothing = sent(at("nobody", "Tax figures")).await;
    assert_eq!(into_hidden.0, 404, "{into_hidden:?}");
    assert_eq!(into_hidden.1.replace(hidden.as_str(), "<id>"), into_nothing.1.replace("nobody", "<id>"), "as for a Job that does not exist");
    assert!(!into_hidden.1.contains(&taxes) && !into_hidden.1.contains("Taxes"), "the project is not named: {}", into_hidden.1);

    // Home's private Job writing to Work's shared one sends nothing at all.
    at_rest(&home, &private_sender, 0).await;
    wrote(&home, 0, &address_of(open.as_str()), "Den notes").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(from_jobs(&work, &open).await.is_empty(), "a private Job's words never leave");
    assert!(home.rig.fleet.queued_listed(None).await.expect("listed").queued.is_empty(), "nor wait to");
    assert_eq!(job_messages_run(&work).await, 0, "Work ran no message");
    assert_eq!(work.rig.rows(&hidden).await.len(), hidden_rows, "nothing reached the private Job's thread");
}

/// What `calls.changed` told on `mac` from now on.
fn told(mac: &OneMac) -> Arc<Mutex<Vec<CallsChanged>>> {
    let told: Arc<Mutex<Vec<CallsChanged>>> = Arc::default();
    let seen = Arc::clone(&told);
    mac.rig.fleet.events().tap(move |event| {
        if let Event::CallsChanged(told) = event {
            seen.lock().expect("not poisoned").push(told.clone());
        }
    });
    told
}

fn arrivals_for(told: &Arc<Mutex<Vec<CallsChanged>>>, job: &SessionId) -> usize {
    told.lock().expect("not poisoned").iter().flat_map(|one| one.arrived.iter()).filter(|one| &one.session_id == job).count()
}

async fn asks(mac: &OneMac, job: &SessionId, id: &str, text: &str) {
    let item = WaitingInput { approves: false, id: id.into(), text: text.into(), act: None, options: Vec::new(), steps: Vec::new() };
    mac.rig.fleet.waiting_for(None, SetWaitingFor { items: vec![item], session_id: Some(job.clone()) }).await.expect("set");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_other_macs_shared_job_that_needs_him_calls_here_once_and_a_private_or_unchanged_one_never() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, mut work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let (shop, taxes) = (project(&work.rig, "Shop").await, project(&work.rig, "Taxes").await);
    shared(&work.rig, &shop, true).await;
    let (open, hidden, before) = (job_in(&work.rig, Some(&shop)).await, job_in(&work.rig, Some(&taxes)).await, job_in(&work.rig, Some(&shop)).await);
    // Waiting before the Macs ever linked: it stands on Home's list, and it is no news.
    asks(&work, &before, "old", "Pick a colour").await;
    let told = told(&home);
    let _calls = crate::standing_calls::keep_the_calls(Arc::clone(&home.rig.fleet));
    let home_ref = &home;
    until("Home's calls are seeded", || async { home_ref.rig.fleet.hosts().calls.seeded() }).await;
    paired(&home, &work).await;
    let on_home = before.clone();
    until("the Job that waited reaches Home's calls", || async {
        home_ref.rig.fleet.get_calls().await.expect("calls").calls.iter().any(|one| one.session_id.as_ref() == Some(&on_home))
    })
    .await;
    assert_eq!(arrivals_for(&told, &before), 0, "what stood before the link is no news");

    // A shared Job that asks him: one arrival, and a standing call that names Work.
    asks(&work, &open, "q1", "Approve the migration").await;
    let job = open.clone();
    until("the ask arrives on Home", || async { arrivals_for(&told, &job) == 1 }).await;
    let calls = home.rig.fleet.get_calls().await.expect("calls").calls;
    let call = calls.iter().find(|one| one.session_id.as_ref() == Some(&open)).expect("its call");
    assert_eq!((call.mac.as_deref(), call.text.as_str(), call.opened), (Some(work.id.as_str()), "Approve the migration", false));
    let needs = home.rig.fleet.get_needs_you(None, Some(open.clone())).await.expect("Needs you for Work's Job");
    assert!(ipc::encode(&needs).expect("encodes").contains("Approve the migration"), "{needs:?}");

    // A private Job that asks him raises nothing on Home.
    asks(&work, &hidden, "q2", "File the return").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_eq!(arrivals_for(&told, &hidden), 0);
    assert!(home.rig.fleet.get_calls().await.expect("calls").calls.iter().all(|one| one.session_id.as_ref() != Some(&hidden)));

    // Work sleeps and wakes with nothing changed: its rows come back as they were, and nothing arrives again.
    work.stop().await;
    let work_id = work.id.clone();
    until("Work shows asleep on Home", || async {
        matches!(home_ref.mac(&work_id).await.and_then(|mac| mac.link), Some(ipc::MacLink::Asleep { .. }))
    })
    .await;
    assert!(home.rig.fleet.get_calls().await.expect("calls").calls.iter().any(|one| one.session_id.as_ref() == Some(&open)), "a sleeping Mac's call stands");
    work.restart();
    until("Work is linked again", || async { matches!(home_ref.mac(&work_id).await.and_then(|mac| mac.link), Some(ipc::MacLink::Linked)) }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert_eq!(arrivals_for(&told, &open), 1, "an unchanged row is no news");
    assert_eq!(arrivals_for(&told, &before), 0);
}
