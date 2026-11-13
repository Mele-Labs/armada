//! Two of his Macs on loopback, each a real Fleet served over its own router and its own Phone Gateway in front,
//! paired by link and Confirm through the Gateways' admin routes. What one shares reaches the other's lists with
//! its Mac's id; a Job of an unshared project, and its report, never reach the other Mac at all. A shared project
//! and its tasks are kept on both, an edit on either shows on the other, and one made while a Mac slept is
//! replayed when it wakes, the later edit of a field winning on both.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use api::{Projects as _, Sessions};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use core_model::Ulid;
use futures_util::StreamExt;
use ipc::{BugReports, Mac, MacLink, NewProjectTask, ProjectRow, ProjectTask, Projects, ReportBug, SessionId, SessionList};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower::ServiceExt;

mod acting;
mod messages;
mod moving;
mod pairing_calls;
mod terminals;
mod updating;

use super::session_host::{rig_fitted, Rig};
use super::share::{job_in, project, shared};
use crate::mint::Mint;
use crate::phone::{OtherMacRead, PhoneSwitch};
use crate::tests::planted::Ticking;
use crate::tests::tmp::TempDir;

/// The link's pace, shortened as `pocket`'s own tests shorten it.
const QUICK: pocket::Timing = pocket::Timing {
    beat: Duration::from_millis(100),
    silence: Duration::from_millis(1500),
    first_retry: Duration::from_millis(100),
    longest_retry: Duration::from_millis(400),
};

/// Ids that start with the Mac's own letters, so no project or report id one Fleet mints is also the other's,
/// and a search for one of Home's ids on Work cannot find Work's own.
struct Minting(&'static str, AtomicU64);

impl Mint for Minting {
    fn ulid(&self) -> Ulid {
        Ulid::carried(format!("{}{:020}", self.0, self.1.fetch_add(1, Ordering::SeqCst)))
    }
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("after 1970").as_secs() as i64
}

/// What `pocketing` hands Fleet: the other Mac reached through this Mac's Gateway while it runs.
struct Switch(Arc<Mutex<Option<pocket::Gateway>>>);

impl PhoneSwitch for Switch {
    fn state(&self) -> ipc::PhoneGateway {
        ipc::PhoneGateway { on: true, port: 0, listening: true, why: None }
    }

    fn set(&self, _on: bool) -> ipc::PhoneGateway {
        self.state()
    }

    fn read_other_mac(&self, mac: &str, path: &str) -> OtherMacRead {
        match self.0.lock().expect("not poisoned").as_ref() {
            Some(gateway) => Box::pin(pocket::read_other_mac(gateway, mac, path)),
            None => Box::pin(std::future::ready(Err(String::from("The Phone Gateway is off on this Mac.")))),
        }
    }

    fn send_other_mac(&self, mac: &str, path: &str, body: Vec<u8>) -> OtherMacRead {
        match self.0.lock().expect("not poisoned").as_ref() {
            Some(gateway) => Box::pin(pocket::send_other_mac(gateway, mac, path, body)),
            None => Box::pin(std::future::ready(Err(String::from("The Phone Gateway is off on this Mac.")))),
        }
    }

    fn follow_other_mac(&self, mac: &str, path: &str) -> crate::phone::OtherMacFollowing {
        match self.0.lock().expect("not poisoned").as_ref() {
            Some(gateway) => {
                let following = pocket::follow_other_mac(gateway, mac, path);
                Box::pin(async move {
                    following.await.map(|followed| match followed {
                        pocket::Followed::Open(lines) => crate::phone::OtherMacFollowed::Open(lines),
                        pocket::Followed::Refused { status, body } => crate::phone::OtherMacFollowed::Refused { status, body },
                    })
                })
            }
            None => Box::pin(std::future::ready(Err(String::from("The Phone Gateway is off on this Mac.")))),
        }
    }
}

/// One Mac: its Fleet served on a loopback port, and its Gateway served on another, linking.
struct OneMac {
    id: String,
    rig: Rig,
    fleet: axum::Router,
    gateway: pocket::Gateway,
    keys: Arc<pocket::MemoryMacKeys>,
    serving: Arc<Mutex<Option<pocket::Gateway>>>,
    port: u16,
    running: Option<(oneshot::Sender<()>, JoinHandle<()>, JoinHandle<()>)>,
    _gateway_home: TempDir,
}

impl OneMac {
    /// `clock` is shared by both Macs, so which of two edits came later is the order the test made them in.
    async fn new(name: &str, ids: &'static str, clock: &Arc<Ticking>) -> OneMac {
        let clock = Arc::clone(clock);
        let rig = rig_fitted(|fittings| {
            fittings.mint = Arc::new(Minting(ids, AtomicU64::new(1)));
            fittings.clock = clock;
        });
        let fleet = api::router(api::Served::sharing(Arc::clone(&rig.fleet), ipc::RunId::carried("01RUN"), rig.fleet.events()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a loopback port for Fleet");
        let fleet_port = listener.local_addr().expect("bound").port();
        let serving = fleet.clone();
        tokio::spawn(async move { axum::serve(listener, serving).await.ok() });

        let home = TempDir::new();
        let id: String = name.bytes().map(|b| format!("{b:02x}")).collect();
        let mut store = store::Store::open_migrated_copy(&home.path().join("armada.db")).expect("the Gateway's store");
        let this = store::ThisMac { id: id.clone(), name: name.to_string(), colour: "teal".into(), minted_at: 1 };
        store.this_mac_or(&this).expect("this Mac");
        let port = pocket::bind(0).await.expect("a loopback port").local_addr().expect("bound").port();
        let address = format!("http://127.0.0.1:{port}");
        let keys = Arc::new(pocket::MemoryMacKeys::default());
        let gateway = pocket::Gateway {
            fleet: Arc::new(move || Ok(fleet_port)),
            assets: None,
            pairing: pocket::Pairing::new(store, Arc::new(now), Arc::new(move || Ok(address.clone()))),
            push: pocket::Push::open(home.path(), pocket::PLACEHOLDER_SUBJECT).expect("a push key"),
            way_in: pocket::mac_way_in(home.path().to_path_buf(), port, None),
            macs: pocket::Macs::with_timing(Arc::clone(&keys) as Arc<dyn pocket::MacKeys>, QUICK).expect("keys"),
        };
        let serving = Arc::new(Mutex::new(None));
        rig.fleet.switching_phone_with(Arc::new(Switch(Arc::clone(&serving))));
        let mut mac = OneMac { id, rig, fleet, gateway, keys, serving, port, running: None, _gateway_home: home };
        mac.start();
        mac
    }

    /// Serve and link, as `pocketing` starts the Gateway.
    fn start(&mut self) {
        let listener = pocket::bind_now(self.port).expect("the Gateway's port");
        let (stop, stopped) = oneshot::channel::<()>();
        let app = pocket::router(self.gateway.clone());
        let serving = tokio::spawn(async move {
            let stopped = async move {
                stopped.await.ok();
            };
            axum::serve(listener, app).with_graceful_shutdown(stopped).await.ok();
        });
        let linking = tokio::spawn(pocket::link(self.gateway.clone()));
        self.running = Some((stop, serving, linking));
        *self.serving.lock().expect("not poisoned") = Some(self.gateway.clone());
    }

    /// Start it again after [`OneMac::stop`], as `pocketing` makes a new Gateway over the same keys and store.
    fn restart(&mut self) {
        self.gateway.macs = pocket::Macs::with_timing(Arc::clone(&self.keys) as Arc<dyn pocket::MacKeys>, QUICK).expect("keys");
        self.start();
    }

    /// Stop it, as `pocketing` stops the Gateway.
    async fn stop(&mut self) {
        let (stop, serving, linking) = self.running.take().expect("running");
        *self.serving.lock().expect("not poisoned") = None;
        self.gateway.macs.close();
        stop.send(()).ok();
        linking.abort();
        tokio::time::timeout(Duration::from_secs(5), serving).await.expect("the Gateway stopped").ok();
    }

    /// A call on the Gateway's own admin routes, from loopback, as Bridge makes it.
    async fn admin(&self, method: Method, path: &str, body: String) -> (StatusCode, String) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("host", format!("127.0.0.1:{}", self.port))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .expect("a request");
        let answer = pocket::router(self.gateway.clone()).oneshot(request).await.expect("answered");
        let status = answer.status();
        let body = to_bytes(answer.into_body(), 1 << 20).await.expect("a body");
        (status, String::from_utf8_lossy(&body).to_string())
    }

    async fn macs(&self) -> Vec<Mac> {
        let (status, body) = self.admin(Method::GET, "/admin/macs", String::new()).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        ipc::decode("macs", body.as_bytes()).expect("the Macs")
    }

    async fn mac(&self, id: &str) -> Option<Mac> {
        self.macs().await.into_iter().find(|mac| mac.id == id)
    }

    /// What Bridge reads from this Mac's Fleet at `path`, as the bytes it is sent.
    async fn read(&self, path: &str) -> String {
        let request = Request::get(path).body(Body::empty()).expect("a request");
        let answer = self.fleet.clone().oneshot(request).await.expect("answered");
        assert_eq!(answer.status(), StatusCode::OK, "{path}");
        let body = to_bytes(answer.into_body(), 1 << 24).await.expect("a body");
        String::from_utf8_lossy(&body).to_string()
    }

    async fn sessions(&self) -> (String, SessionList) {
        let raw = self.read("/sessions").await;
        let list = ipc::decode("the Jobs list", raw.as_bytes()).expect("a Jobs list");
        (raw, list)
    }

    async fn bug_reports(&self) -> (String, BugReports) {
        let raw = self.read("/bug_reports").await;
        let reports = ipc::decode("the bug reports", raw.as_bytes()).expect("bug reports");
        (raw, reports)
    }

    /// Every message this Fleet's `/events` socket sends from now on, as text, kept for the test to search.
    async fn every_event(&self) -> Arc<Mutex<Vec<String>>> {
        let port = (self.gateway.fleet)().expect("Fleet's port");
        let stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.expect("Fleet's port");
        let (mut socket, _) = tokio_tungstenite::client_async(format!("ws://127.0.0.1:{port}/events"), stream)
            .await
            .expect("the events socket");
        let carried = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&carried);
        tokio::spawn(async move {
            while let Some(Ok(message)) = socket.next().await {
                let bytes = message.into_data();
                into.lock().expect("not poisoned").push(String::from_utf8_lossy(&bytes).to_string());
            }
        });
        carried
    }

    async fn projects(&self) -> (String, Projects) {
        let raw = self.read("/projects").await;
        let projects = ipc::decode("the projects", raw.as_bytes()).expect("projects");
        (raw, projects)
    }

    async fn reported(&self, job: &SessionId) -> String {
        let report = ReportBug {
            title: "Refresh drops the lease".into(),
            what: "It quotes private work.".into(),
            steps: Vec::new(),
            errors: Vec::new(),
            logs: Vec::new(),
            session_id: Some(job.clone()),
        };
        self.rig.fleet.report_bug(None, report).await.expect("reported");
        let reports = self.rig.fleet.list_bug_reports().await.expect("listed").reports;
        reports.into_iter().find(|one| one.session_id == job.as_str()).expect("the report").id
    }
}

/// Home shows the link, Work pastes it, Home presses Confirm.
async fn paired(home: &OneMac, work: &OneMac) {
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/start", String::new()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let link: ipc::MacPairLink = ipc::decode("a link", body.as_bytes()).expect("a link");
    let claim = ipc::encode(&ipc::ClaimMacLink { link: link.link }).expect("encodes");
    let (status, body) = work.admin(Method::POST, "/admin/macs/pair/claim", claim).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id.clone() }).expect("encodes");
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Wait until `check` holds, for at most ten seconds.
async fn until<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never: {what}");
}

fn from(list: &SessionList, job: &SessionId) -> Option<Option<String>> {
    list.sessions.iter().find(|one| one.id == *job).map(|one| one.mac.clone())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_paired_macs_see_each_others_shared_jobs_and_never_a_private_one() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let open = project(&home.rig, "Shop").await;
    let private = project(&home.rig, "Taxes").await;
    shared(&home.rig, &open, true).await;
    let s = job_in(&home.rig, Some(&open)).await;
    let p = job_in(&home.rig, Some(&private)).await;
    let s_report = home.reported(&s).await;
    let p_report = home.reported(&p).await;
    let theirs = project(&work.rig, "Notes").await;
    shared(&work.rig, &theirs, true).await;
    let t = job_in(&work.rig, Some(&theirs)).await;
    let carried = work.every_event().await;

    // 1. Paired by link and Confirm, each lists the other as linked.
    paired(&home, &work).await;
    assert_eq!(home.mac(&work.id).await.expect("Work on Home").link, Some(MacLink::Linked));
    assert_eq!(work.mac(&home.id).await.expect("Home on Work").link, Some(MacLink::Linked));

    // 2. Each Mac's Jobs list shows the other's shared Job with that Mac's id, and Work lists S's report.
    until("S reaches Work's Jobs list from Home", || async { from(&work.sessions().await.1, &s) == Some(Some(home.id.clone())) }).await;
    until("S's report reaches Work's reports from Home", || async {
        let reports = work.bug_reports().await.1.reports;
        reports.iter().any(|one| one.id == s_report && one.mac.as_deref() == Some(home.id.as_str()))
    })
    .await;
    until("T reaches Home's Jobs list from Work", || async { from(&home.sessions().await.1, &t) == Some(Some(work.id.clone())) }).await;
    assert!(carried.lock().expect("not poisoned").iter().any(|one| one.contains(s.as_str())), "Work's stream carried S");

    // 3. P and its report are nowhere on Work.
    let nowhere = |what: &str, raw: &str| {
        assert!(!raw.contains(p.as_str()), "P's id is in Work's {what}: {raw}");
        assert!(!raw.contains(p_report.as_str()), "P's report is in Work's {what}: {raw}");
    };
    nowhere("Jobs list", &work.sessions().await.0);
    nowhere("reports", &work.bug_reports().await.0);

    // 4. S's project turned off on Home, S leaves Work's list.
    shared(&home.rig, &open, false).await;
    until("S leaves Work's Jobs list", || async { from(&work.sessions().await.1, &s).is_none() }).await;
    nowhere("Jobs list", &work.sessions().await.0);
    nowhere("reports", &work.bug_reports().await.0);

    // 5. Home's Gateway down, Work shows Home asleep, since when and why.
    let before = now();
    let mut home = home;
    home.stop().await;
    until("Home shows asleep on Work", || async {
        matches!(work.mac(&home.id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    let asleep = work.mac(&home.id).await.expect("Home on Work");
    let Some(MacLink::Asleep { since }) = asleep.link else { unreachable!() };
    assert!(since >= before - 1 && since <= now(), "asleep since {since}");
    assert!(asleep.why.as_deref().is_some_and(|why| why.starts_with("Home ")), "{asleep:?}");

    let carried = carried.lock().expect("not poisoned").join("\n");
    nowhere("event stream", &carried);
}

/// The titles of project `id`'s tasks in `projects`, and the Mac it is held from. None where it is not listed.
fn tasks_of(projects: &Projects, id: &str) -> Option<(Option<String>, Vec<String>)> {
    let project = projects.projects.iter().find(|one| one.id == id)?;
    let titles = project
        .rows
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Task { title, .. } => Some(title.clone()),
            ProjectRow::Issue { .. } => None,
        })
        .collect();
    Some((project.mac.clone(), titles))
}

/// A task in `project` on `mac`, by its title: its row's id.
async fn task_in(mac: &OneMac, project: &str, title: &str) -> String {
    let task = NewProjectTask { manifest_id: mac.rig.manifest.clone(), project: Some(project.to_string()), title: title.into(), notes: String::new() };
    let projects = mac.rig.fleet.add_project_task(task).await.expect("added");
    let rows = &projects.projects.iter().find(|one| one.id == project).expect("the project").rows;
    rows.iter()
        .find_map(|row| match row {
            ProjectRow::Task { id, title: said, .. } if said == title => Some(id.clone()),
            _ => None,
        })
        .expect("the task")
}

/// Whether `mac` lists project `open`, held by Mac `holder`, with the one task `title`.
async fn shows(mac: &OneMac, holder: &str, open: &str, title: &str) -> bool {
    let held_from = (mac.id != holder).then(|| holder.to_string());
    tasks_of(&mac.projects().await.1, open) == Some((held_from, vec![title.to_string()]))
}

async fn titled(mac: &OneMac, row: &str, title: &str) {
    let task = ProjectTask { title: title.into(), notes: String::new() };
    mac.rig.fleet.edit_project_task(row.to_string(), task).await.expect("edited");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shared_project_is_kept_in_step_on_both_macs_and_a_private_one_never_crosses() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let open = project(&home.rig, "Shop").await;
    let private = project(&home.rig, "Taxes").await;
    shared(&home.rig, &open, true).await;
    let row = task_in(&home, &open, "Buy tape").await;
    let secret = task_in(&home, &private, "Amend the 2025 return").await;
    let carried = work.every_event().await;
    paired(&home, &work).await;
    let mut home = home;
    let home_id = home.id.clone();

    // 1. The shared project reaches Work with its task, held from Home. Taxes and its task are nowhere on Work.
    until("Shop reaches Work with its task", || shows(&work, &home_id, &open, "Buy tape")).await;
    let nowhere = |what: &str, raw: &str| {
        assert!(!raw.contains(private.as_str()), "Taxes is in Work's {what}: {raw}");
        assert!(!raw.contains(secret.as_str()) && !raw.contains("Amend the 2025 return"), "Taxes' task is in Work's {what}: {raw}");
    };
    nowhere("projects", &work.projects().await.0);

    // 2. An edit on Work's copy is made on Home, and comes back to the copy; an edit on Home reaches Work.
    titled(&work, &row, "Buy blue tape").await;
    until("Work's edit reaches Home", || shows(&home, &home_id, &open, "Buy blue tape")).await;
    until("Work's copy shows Home's answer", || shows(&work, &home_id, &open, "Buy blue tape")).await;
    titled(&home, &row, "Buy red tape").await;
    until("Home's edit reaches Work", || shows(&work, &home_id, &open, "Buy red tape")).await;

    // 3. Both edit the title while Home sleeps, Work later: Work's edit shows on its copy at once, and wins on both.
    home.stop().await;
    until("Home shows asleep on Work", || async {
        matches!(work.mac(&home.id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    titled(&home, &row, "Home, first").await;
    titled(&work, &row, "Work, later").await;
    assert!(shows(&work, &home_id, &open, "Work, later").await, "Work's edit shows on its copy while Home sleeps");
    assert!(shows(&home, &home_id, &open, "Home, first").await, "Home has not heard it yet");
    home.restart();
    until("Work's later edit wins on Home", || shows(&home, &home_id, &open, "Work, later")).await;
    until("and on Work", || shows(&work, &home_id, &open, "Work, later")).await;

    // 4. Again, Home later: Home's edit wins on both.
    home.stop().await;
    until("Home shows asleep on Work again", || async {
        matches!(work.mac(&home.id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    titled(&work, &row, "Work, first").await;
    titled(&home, &row, "Home, later").await;
    assert!(shows(&work, &home_id, &open, "Work, first").await, "Work's edit shows on its copy while Home sleeps");
    home.restart();
    until("Home's later edit wins on Work", || shows(&work, &home_id, &open, "Home, later")).await;
    assert!(shows(&home, &home_id, &open, "Home, later").await, "and stays on Home");

    // 5. Sharing turned off on Home, the copy leaves Work.
    shared(&home.rig, &open, false).await;
    until("Shop leaves Work", || async { tasks_of(&work.projects().await.1, &open).is_none() }).await;
    nowhere("projects", &work.projects().await.0);
    let carried = carried.lock().expect("not poisoned").join("\n");
    nowhere("event stream", &carried);
}
