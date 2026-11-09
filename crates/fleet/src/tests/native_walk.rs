//! **A native walk is a screen of the native Bridge a Job shows on `armada://walk/<name>`**, opened by
//! the person's press from the Job's own slot, and owned by Fleet for as long as the walk is.
//! `crates/fleet/src/native_walks.rs`.

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use api::Sessions;
use ipc::{OpenNativeWalk, SessionId, ShowWindow, WalkStep, WindowClosed, WithdrawWindow};

use super::session_gate::in_its_slot;
use super::session_host::{eventually, Rig};
use crate::tests::peer::Placing;

const URL: &str = "armada://walk/sidebar";
const OTHER: &str = "armada://walk/footer";
const BUILD: &str = "crates/bridge-native/target/debug/bridge-native";

fn slot(rig: &Rig) -> String {
    adapter_traits::slot_path(&rig.root, 1)
}

fn shown(id: &SessionId, url: &str) -> ShowWindow {
    ShowWindow { url: url.into(), title: None, session_id: Some(id.clone()), ..Default::default() }
}

fn press(id: &SessionId, url: &str) -> OpenNativeWalk {
    OpenNativeWalk { session_id: id.clone(), url: url.into() }
}

/// What a refusal says.
fn said<T: std::fmt::Debug>(answer: Result<T, api::Refusal>) -> String {
    format!("{:?}", answer.expect_err("refused"))
}

/// A build in the slot that records where and how it was started, then stays up.
fn built(rig: &Rig) {
    let binary = format!("{}/{BUILD}", slot(rig));
    std::fs::create_dir_all(std::path::Path::new(&binary).parent().unwrap()).expect("a target folder");
    std::fs::write(&binary, "#!/bin/sh\necho \"$$ $(pwd -P) $@\" >> starts.txt\nexec sleep 60\n").expect("a build");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("executable");
    // The slot's `HEAD` was made long before this build.
    rig.fleet.vcs().head_made_at(&slot(rig), 1);
}

fn starts(rig: &Rig) -> Vec<String> {
    std::fs::read_to_string(format!("{}/starts.txt", slot(rig)))
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// A job in its slot with a native walk shown and a build to open it with.
async fn walking() -> (Rig, SessionId) {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    built(&rig);
    (rig, id)
}

/// Press Open and wait for the app to say it is up, `nth` of them in this slot.
async fn opened(rig: &Rig, id: &SessionId, url: &str, nth: usize) {
    rig.fleet.open_native_walk(press(id, url)).await.expect("opened");
    eventually(|| async { starts(rig).len() == nth }).await;
}

fn running(rig: &Rig, id: &SessionId, url: &str) -> bool {
    rig.fleet.native_walks().running(id.as_str(), url)
}

#[tokio::test]
async fn show_window_takes_a_native_walk_address_and_refuses_what_only_a_web_page_takes_on_it() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    let record = rig.fleet.show_window(None, shown(&id, URL)).await.expect("a native walk is shown");
    assert!(record.attachments.iter().any(|one| one.target == URL), "kept on the ledger as any window is");
    assert_eq!(rig.fleet.walk_open(id.as_str()).await.as_deref(), Some(URL));

    let step: Vec<WalkStep> = ipc::decode("a step", br#"[{"say":"Look","look":{"role":"button","name":"Add"}}]"#).expect("a step");
    let steps = said(rig.fleet.show_window(None, ShowWindow { steps: step, ..shown(&id, OTHER) }).await);
    assert!(steps.contains("a native walk's steps live in its walk file in crates/bridge-native"), "{steps}");
    let take = ipc::WindowTake { name: "A".into(), url: OTHER.into() };
    assert!(rig.fleet.show_window(None, ShowWindow { variations: vec![take], ..shown(&id, OTHER) }).await.is_err());
    assert!(rig.fleet.show_window(None, ShowWindow { origins: vec!["https://login.example.com".into()], ..shown(&id, OTHER) }).await.is_err());
    assert!(rig.fleet.show_window(None, ShowWindow { sign_in: ipc::SignIn(vec![OTHER.into()]), ..shown(&id, OTHER) }).await.is_err());
    for bad in ["armada://walk/", "armada://walk/Sidebar", "armada://walk/a_b", "armada://walk/a/b"] {
        assert!(rig.fleet.show_window(None, shown(&id, bad)).await.is_err(), "{bad}");
    }
    assert_eq!(rig.fleet.walk_open(id.as_str()).await.as_deref(), Some(URL), "nothing refused was written");
}

#[tokio::test]
async fn opening_a_native_walk_with_no_build_in_the_slot_is_refused_with_what_to_build() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");

    let refused = said(rig.fleet.open_native_walk(press(&id, URL)).await);

    assert!(refused.contains(&format!("Build the walk first: cargo build in crates/bridge-native in {}", slot(&rig))), "{refused}");
    assert!(!running(&rig, &id, URL));
}

#[tokio::test]
async fn opening_a_native_walk_with_a_build_older_than_the_slots_head_commit_is_refused() {
    let (rig, id) = walking().await;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    rig.fleet.vcs().head_made_at(&slot(&rig), now + 3600);

    let refused = said(rig.fleet.open_native_walk(press(&id, URL)).await);

    assert!(refused.contains("Build the walk first: cargo build in crates/bridge-native in"), "{refused}");
    assert!(starts(&rig).is_empty(), "nothing was started");
}

#[tokio::test]
async fn opening_a_native_walk_that_is_closed_withdrawn_or_never_shown_is_refused() {
    let (rig, id) = walking().await;
    assert!(rig.fleet.open_native_walk(press(&id, OTHER)).await.is_err(), "never shown");
    assert!(rig.fleet.open_native_walk(press(&id, "http://localhost:5173/x")).await.is_err(), "not a native walk's address");

    rig.fleet.window_closed(WindowClosed { session_id: id.clone(), url: URL.into() }).await.expect("closed");
    assert!(rig.fleet.open_native_walk(press(&id, URL)).await.is_err(), "closed");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown again");
    rig.fleet.withdraw_window(None, WithdrawWindow { url: URL.into(), session_id: Some(id.clone()) }).await.expect("withdrawn");
    assert!(rig.fleet.open_native_walk(press(&id, URL)).await.is_err(), "withdrawn");

    assert!(starts(&rig).is_empty(), "nothing was started");
}

#[tokio::test]
async fn a_native_walk_opens_beside_a_page_shown_after_it() {
    let (rig, id) = walking().await;
    rig.fleet.show_window(None, shown(&id, "http://localhost:5792/?scenario=compare")).await.expect("a second tab");
    let opened = rig.fleet.open_native_walk(press(&id, URL)).await.expect("its tab is still open");
    assert!(opened.started);
}

#[tokio::test]
async fn two_presses_start_one_app_from_the_slot_with_the_walk_the_job_the_address_and_fleets_port() {
    let (rig, id) = walking().await;

    let first = rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened");
    let second = rig.fleet.open_native_walk(press(&id, URL)).await.expect("still ok");

    assert!(first.started && !second.started);
    eventually(|| async { starts(&rig).len() == 1 }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let line = starts(&rig);
    assert_eq!(line.len(), 1, "one process: {line:?}");
    let port = rig.fleet.host().port;
    let real = std::fs::canonicalize(slot(&rig)).expect("the slot");
    assert!(
        line[0].contains(&format!(" {} --walk sidebar --job {} --url {URL} --fleet-port {port}", real.display(), id.as_str())),
        "started in the slot, with the arguments the app reads: {line:?}"
    );
    assert!(running(&rig, &id, URL));
}

#[tokio::test]
async fn the_app_goes_when_the_walk_is_closed_or_approved_and_stays_beside_another_tab() {
    let (rig, id) = walking().await;

    opened(&rig, &id, URL, 1).await;
    rig.fleet.window_closed(WindowClosed { session_id: id.clone(), url: URL.into() }).await.expect("closed");
    assert!(!running(&rig, &id, URL), "closed");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    opened(&rig, &id, URL, 2).await;
    rig.fleet.show_window(None, shown(&id, OTHER)).await.expect("another tab");
    assert!(running(&rig, &id, URL), "a page shown beside it leaves it running");

    opened(&rig, &id, OTHER, 3).await;
    rig.send(&id, &format!("Approved: {OTHER}")).await;
    assert!(!running(&rig, &id, OTHER), "approved");
    assert!(running(&rig, &id, URL), "approving one walk leaves the other");

    rig.fleet.show_window(None, shown(&id, OTHER)).await.expect("shown");
    opened(&rig, &id, OTHER, 4).await;
    assert!(running(&rig, &id, OTHER), "opened again after its app went");
}

#[tokio::test]
async fn the_app_goes_when_the_job_ends_the_slot_is_given_back_and_fleet_stops() {
    let (rig, id) = walking().await;

    opened(&rig, &id, URL, 1).await;
    rig.fleet.session_wound_up(id.as_str()).await;
    assert!(!running(&rig, &id, URL), "the job ended");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    opened(&rig, &id, URL, 2).await;
    rig.fleet.stopped_servers_in(&slot(&rig)).await;
    assert!(!running(&rig, &id, URL), "the slot was given back");

    opened(&rig, &id, URL, 3).await;
    assert!(running(&rig, &id, URL));
    rig.fleet.native_walks_stopped();
    assert!(!running(&rig, &id, URL), "fleet stopped");
}

const MESSAGE: &str = "{session_id: string, text: string, attachments?: [{name: string, media_type: string, data: string}], \
mentions?: [{kind: session|pull_request|branch|issue|report|terminal, id: string, title: string, url?: string}]}";

/// One note, as `send_session_note` takes it beside its `session_id` and `send_session_notes` takes a list of them.
const NOTE: &str = "{said: string, address: string, note_id?: string, \
place: {component?: string, owners?: [string], selector: string, \
element: {tag: string, text: string, label?: string}, nearby?: [string], stamp?: string, location: string, \
screen?: string, layer?: string}, \
state: {selected?: [string], walk?: {name: string, step: number, of: number}, take?: {letter: string, name: string}, \
scenario?: string}, crop?: {media_type: string, data: string}, job?: [string]}";

/// **The wire the native app speaks in walk mode**: it skips the protocol-id check and uses these
/// three routes of Fleet and no other. They are written out whole, so a change to a path, a method
/// or a field of any of them fails here and not in a walk of a branch built before the change.
#[test]
fn the_routes_a_native_walk_speaks_keep_the_paths_methods_and_fields_older_builds_send() {
    let route = |operation: &str| {
        let found = api::SERVED.iter().find(|one| one.operation == operation).unwrap_or_else(|| panic!("{operation} is not served"));
        format!("{} {}", found.method, found.path)
    };
    assert_eq!(route("send_session_message"), "POST /sessions/message");
    assert_eq!(route("send_session_note"), "POST /sessions/note");
    assert_eq!(route("send_session_notes"), "POST /sessions/notes");
    assert_eq!(route("window_closed"), "POST /sessions/window/closed");

    // The approval: `text` is "Approved: <url>", which Fleet reads as the end of the walk. Body
    // types that flatten another read as `object`, so the inner type is pinned and the body decoded below.
    assert_eq!(ipc::shape::of::<ipc::SendSessionMessage>(), MESSAGE);
    assert_eq!(crate::session_host::walking::approved_address("Approved: armada://walk/sidebar"), Some("armada://walk/sidebar"));
    assert_eq!(ipc::shape::of::<ipc::SessionNote>(), NOTE);
    assert_eq!(ipc::shape::of::<ipc::SendSessionNotes>(), format!("{{session_id: string, notes: [{NOTE}]}}"));
    assert_eq!(ipc::shape::of::<ipc::WindowClosed>(), "{session_id: string, url: string}");

    let message: ipc::SendSessionMessageBody =
        ipc::decode_body("a message", br#"{"session_id":"S1","text":"Approved: armada://walk/sidebar"}"#).expect("the approval the app sends");
    assert_eq!(message.message.text, "Approved: armada://walk/sidebar");
    let note = br#"{"session_id":"S1","said":"too tight","address":"armada://walk/sidebar",
        "place":{"selector":"sidebar > row","element":{"tag":"row","text":"Jobs"},"location":"/"},"state":{}}"#;
    let note: ipc::SendSessionNote = ipc::decode_body("a note", note).expect("the note the app sends");
    assert_eq!((note.note.said.as_str(), note.note.address.as_str()), ("too tight", "armada://walk/sidebar"));
    let closed: ipc::WindowClosed =
        ipc::decode_body("a closed window", br#"{"session_id":"S1","url":"armada://walk/sidebar"}"#).expect("the close the app sends");
    assert_eq!(closed.url, "armada://walk/sidebar");
}
