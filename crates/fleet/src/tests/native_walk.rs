//! **A native walk is a screen of the native Bridge a Job shows on `armada://walk/<name>`**, opened by
//! the person's press from the build he runs, and owned by Fleet for as long as the walk is.
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
const APP: &str = "Armada.app/Contents/MacOS/Armada";

/// The kept build he runs, under the rig's own home: never this Mac's.
fn build(rig: &Rig) -> std::path::PathBuf {
    let home = rig.fleet.host().home.clone();
    assert_ne!(Some(home.as_str()), std::env::var("HOME").ok().as_deref(), "a test's home, never his");
    crate::native_walks::running_build(&home)
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

/// His build's native Bridge, written as `script` says.
fn kept(rig: &Rig, script: &str) {
    let binary = build(rig).join(APP);
    std::fs::create_dir_all(binary.parent().unwrap()).expect("the app's folder");
    // Written to a new file and moved in: a running walk keeps the old one, as a restart's new build would.
    let fresh = binary.with_extension("new");
    std::fs::write(&fresh, script).expect("a build");
    std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755)).expect("executable");
    std::fs::rename(&fresh, &binary).expect("in place");
}

/// His build plays both walks, records where and how each was started, then stays up.
fn built(rig: &Rig) {
    kept(rig, "#!/bin/sh\nif [ \"$1\" = --list-walks ]; then echo sidebar; echo footer; exit 0; fi\necho \"$$ $(pwd -P) $@\" >> starts.txt\nexec sleep 60\n");
}

fn starts(rig: &Rig) -> Vec<String> {
    std::fs::read_to_string(build(rig).join("starts.txt"))
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// A job with a native walk shown that the build he runs plays.
async fn walking() -> (Rig, SessionId) {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    built(&rig);
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
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
    built(&rig);
    let record = rig.fleet.show_window(None, shown(&id, URL)).await.expect("a native walk is shown");
    assert!(record.attachments.iter().any(|one| one.target == URL), "kept on the ledger as any window is");
    assert_eq!(rig.fleet.walk_open(id.as_str()).await.as_deref(), Some(URL));

    let step: Vec<WalkStep> = ipc::decode("a step", br#"[{"say":"Look","look":{"role":"button","name":"Add"}}]"#).expect("a step");
    let steps = said(rig.fleet.show_window(None, ShowWindow { steps: step, ..shown(&id, OTHER) }).await);
    assert!(steps.contains("a native walk's steps live in its walk file in crates/bridge-native"), "{steps}");
    let page = ipc::WindowTake { name: "A".into(), url: "http://localhost:5173/x".into() };
    let web = said(rig.fleet.show_window(None, ShowWindow { variations: vec![page], ..shown(&id, OTHER) }).await);
    assert!(web.contains("a native walk's address on every variation of a native walk"), "{web}");
    assert!(rig.fleet.show_window(None, ShowWindow { origins: vec!["https://login.example.com".into()], ..shown(&id, OTHER) }).await.is_err());
    assert!(rig.fleet.show_window(None, ShowWindow { sign_in: ipc::SignIn(vec![OTHER.into()]), ..shown(&id, OTHER) }).await.is_err());
    for bad in ["armada://walk/", "armada://walk/Sidebar", "armada://walk/a_b", "armada://walk/a/b"] {
        assert!(rig.fleet.show_window(None, shown(&id, bad)).await.is_err(), "{bad}");
    }
    assert_eq!(rig.fleet.walk_open(id.as_str()).await.as_deref(), Some(URL), "nothing refused was written");
}

#[tokio::test]
async fn showing_a_native_walk_the_build_he_runs_does_not_play_is_refused_to_the_job() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    let none = said(rig.fleet.show_window(None, shown(&id, URL)).await);
    assert!(none.contains("the build he runs has no native Bridge at"), "{none}");
    assert!(none.contains("offer_preview the branch that adds it"), "{none}");

    // A build from before `--list-walks` exits 2 on it, as on any flag it does not read.
    kept(&rig, "#!/bin/sh\nexit 2\n");
    let old = said(rig.fleet.show_window(None, shown(&id, URL)).await);
    assert!(old.contains("cannot name its walks"), "{old}");

    built(&rig);
    let unknown = said(rig.fleet.show_window(None, shown(&id, "armada://walk/header")).await);
    assert!(unknown.contains("the build he runs has no walk named header: offer_preview"), "{unknown}");
    let take = vec![ipc::WindowTake { name: "A".into(), url: URL.into() }, ipc::WindowTake { name: "B".into(), url: "armada://walk/header".into() }];
    let takes = said(rig.fleet.show_window(None, ShowWindow { variations: take, ..shown(&id, URL) }).await);
    assert!(takes.contains("no walk named header"), "a take is held to it too: {takes}");
    assert_eq!(rig.fleet.walk_open(id.as_str()).await, None, "nothing refused was written");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("a walk it plays is shown");
    opened(&rig, &id, URL, 1).await;
}

#[tokio::test]
async fn a_build_gone_since_the_walk_was_shown_is_refused_on_the_press() {
    let (rig, id) = walking().await;
    std::fs::remove_file(build(&rig).join(APP)).expect("gone");

    let refused = said(rig.fleet.open_native_walk(press(&id, URL)).await);

    assert!(refused.contains("the build he runs has no native Bridge at"), "{refused}");
    assert!(!running(&rig, &id, URL));
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
async fn the_walk_runs_as_armada_from_a_bundle_of_its_own_around_his_builds_app() {
    use std::os::unix::fs::MetadataExt;
    let (rig, id) = walking().await;
    opened(&rig, &id, URL, 1).await;
    let bundle = crate::runtime::support_dir(&rig.fleet.host().home).join("walk/Armada.app/Contents");
    let info = std::fs::read_to_string(bundle.join("Info.plist")).expect("an Info.plist");
    assert!(info.contains("<key>CFBundleName</key><string>Armada</string>"), "{info}");
    assert!(info.contains("dev.armada.bridge.native.walk"), "apart from his running native Bridge: {info}");
    let linked = std::fs::metadata(bundle.join("MacOS/Armada")).expect("the executable");
    let app = std::fs::metadata(build(&rig).join(APP)).expect("his build's app");
    assert_eq!(linked.ino(), app.ino(), "a hard link to his build's app, not a copy");
}

#[tokio::test]
async fn a_native_walk_opens_beside_a_page_shown_after_it() {
    let (rig, id) = walking().await;
    rig.fleet.show_window(None, shown(&id, "http://localhost:5792/?scenario=compare")).await.expect("a second tab");
    let opened = rig.fleet.open_native_walk(press(&id, URL)).await.expect("its tab is still open");
    assert!(opened.started);
}

#[tokio::test]
async fn two_presses_start_one_app_from_his_build_with_the_walk_the_job_the_address_and_fleets_port() {
    let (rig, id) = walking().await;

    let first = rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened");
    let second = rig.fleet.open_native_walk(press(&id, URL)).await.expect("still ok");

    assert!(first.started && !second.started);
    eventually(|| async { starts(&rig).len() == 1 }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let line = starts(&rig);
    assert_eq!(line.len(), 1, "one process: {line:?}");
    let port = rig.fleet.host().port;
    let real = std::fs::canonicalize(build(&rig)).expect("his build");
    assert!(
        line[0].contains(&format!(" {} --walk sidebar --job {} --url {URL} --fleet-port {port}", real.display(), id.as_str())),
        "started in his build, with the arguments the app reads: {line:?}"
    );
    assert!(running(&rig, &id, URL));
}

#[tokio::test]
async fn a_native_walks_variations_are_other_native_walks_and_the_app_starts_with_each_as_a_take() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    built(&rig);
    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];
    rig.fleet.show_window(None, ShowWindow { variations: takes, ..shown(&id, URL) }).await.expect("takes of one walk");

    opened(&rig, &id, URL, 1).await;

    let line = starts(&rig);
    assert!(line[0].contains(&format!("--url {URL} --fleet-port {} --take Sidebar=sidebar --take Footer=footer", rig.fleet.host().port)), "{line:?}");
}

#[tokio::test]
async fn a_press_after_his_preview_restarts_stops_the_running_app_and_starts_the_new_one() {
    let (rig, id) = walking().await;
    opened(&rig, &id, URL, 1).await;
    let first = starts(&rig)[0].split(' ').next().map(str::to_string).expect("its pid");

    // A restart keeps a new app; its time is set, as a coarse filesystem may not move between the two writes.
    built(&rig);
    std::fs::File::open(build(&rig).join(APP))
        .expect("the new app")
        .set_modified(std::time::SystemTime::now() + Duration::from_secs(60))
        .expect("kept after the first");
    let again = rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened again");

    assert!(again.started, "the new build was started");
    eventually(|| async { starts(&rig).len() == 2 }).await;
    assert!(running(&rig, &id, URL));
    // Gone, or a zombie not yet reaped.
    let stopped = || {
        let out = std::process::Command::new("ps").args(["-o", "stat=", "-p", &first]).output().expect("ps runs");
        let stat = String::from_utf8_lossy(&out.stdout).trim().to_string();
        stat.is_empty() || stat.starts_with('Z')
    };
    eventually(|| async { stopped() }).await;
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
async fn the_app_goes_when_the_job_ends_and_fleet_stops() {
    let (rig, id) = walking().await;

    opened(&rig, &id, URL, 1).await;
    rig.fleet.session_wound_up(id.as_str()).await;
    assert!(!running(&rig, &id, URL), "the job ended");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    opened(&rig, &id, URL, 2).await;
    assert!(running(&rig, &id, URL));
    rig.fleet.native_walks_stopped();
    assert!(!running(&rig, &id, URL), "fleet stopped");
}

const MESSAGE: &str = "{session_id: string, text: string, attachments?: [{name: string, media_type: string, data: string}], \
mentions?: [{kind: session|pull_request|branch|issue|report|terminal, id: string, title: string, url?: string}], \
sketches?: [{id: string, title: string}]}";

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
