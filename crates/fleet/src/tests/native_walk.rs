//! **A native walk is a screen of the native Bridge a Job shows on `armada://walk/<name>`**, built by
//! Fleet from the Job's own commit, opened by the person's press and owned by Fleet for as long as the
//! walk is. `crates/fleet/src/native_walks.rs`, `crates/fleet/src/walk_builds.rs`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{OpenNativeWalk, SessionId, SessionRow, ShowWindow, WalkBuild, WalkBuildState, WalkStep, WindowClosed, WithdrawWindow};

use super::session_gate::{in_its_slot, in_its_slot_fitted};
use super::session_host::Rig;
use crate::tests::peer::Placing;

pub(super) const URL: &str = "armada://walk/sidebar";
pub(super) const OTHER: &str = "armada://walk/footer";
const APP: &str = "Armada.app/Contents/MacOS/Armada";

/// The kept build he runs, under the rig's own home: never this Mac's.
fn build(rig: &Rig) -> PathBuf {
    let home = rig.fleet.host().home.clone();
    assert_ne!(Some(home.as_str()), std::env::var("HOME").ok().as_deref(), "a test's home, never his");
    crate::native_walks::running_build(&home)
}

pub(super) fn shown(id: &SessionId, url: &str) -> ShowWindow {
    ShowWindow { url: url.into(), title: None, session_id: Some(id.clone()), ..Default::default() }
}

fn press(id: &SessionId, url: &str) -> OpenNativeWalk {
    OpenNativeWalk { session_id: id.clone(), url: url.into() }
}

/// What a refusal says.
fn said<T: std::fmt::Debug>(answer: Result<T, api::Refusal>) -> String {
    format!("{:?}", answer.expect_err("refused"))
}

/// Wait for `check`, which may take git and a script: a minute.
pub(super) async fn until<F: std::future::Future<Output = bool>>(mut check: impl FnMut() -> F) {
    for _ in 0..6000 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("it did not come true within a minute");
}

/// `git <args>` in `at`, as a test's own author.
fn git(at: &Path, args: &[&str]) -> String {
    let done = std::process::Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .current_dir(at)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

/// The slot the Job holds, a repository with one commit.
pub(super) async fn leased(rig: &Rig, id: &SessionId) -> PathBuf {
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = Some(1);
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let slot = PathBuf::from(adapter_traits::slot_path(&rig.root, 1));
    std::fs::create_dir_all(&slot).expect("a slot");
    git(&slot, &["init", "--quiet"]);
    commit(&slot, "first");
    slot
}

/// One more commit on the slot's branch: the Job moved on. Its sha.
pub(super) fn commit(slot: &Path, name: &str) -> String {
    std::fs::write(slot.join(format!("{name}.txt")), name).expect("a file");
    git(slot, &["add", "."]);
    git(slot, &["commit", "--quiet", "-m", name]);
    git(slot, &["rev-parse", "HEAD"])
}

/// What a build writes as the native Bridge: it plays `walks`, records where and how each walk was
/// started, then stays up.
fn script(walks: &[&str]) -> String {
    let said: String = walks.iter().map(|walk| format!("echo {walk}; ")).collect();
    format!(
        "mkdir -p crates/bridge-native/target/release && cat > crates/bridge-native/target/release/bridge-native <<'EOF'\n\
         #!/bin/sh\n\
         if [ \"$1\" = --list-walks ]; then {said}exit 0; fi\n\
         echo \"$$ $(pwd -P) $@\" >> starts.txt\n\
         exec sleep 60\n\
         EOF\n\
         chmod +x crates/bridge-native/target/release/bridge-native"
    )
}

/// The script a repository's declared `build` runs, in its home: what a test writes to say how the build goes.
const BUILD_SCRIPT: &str = "walk-build.sh";

/// An `armada.yml` declaring `walks:` as this repository's does, but for `build`, which is `script` in the home
/// the test is given so the build is a stand-in and not cargo.
pub(super) fn declared(home: &str) -> config::Manifest {
    let text = format!(
        "version: 1\nid: 01FIXTUREMANIFEST\nwalks:\n  build: sh {home}/{BUILD_SCRIPT}\n  binary: crates/bridge-native/target/release/bridge-native\n  \
         list: ${{binary}} --list-walks\n  open: ${{binary}} --walk ${{walk}} --job ${{job}} --url ${{url}} --fleet-port ${{fleet_port}}\n  \
         take: --take ${{name}}=${{walk}}\n  shots: ${{binary}} --walk ${{walk}} --check --shots ${{shots}} --hold 0\n  \
         against: ${{binary}} --walk ${{walk}} --check --shots ${{shots}} --against ${{against}} --out ${{out}} --hold 0\n  \
         seed: crates/bridge-native/target\n"
    );
    config::Manifest::parse(Path::new("armada.yml"), &text).expect("walks as armada.yml declares them")
}

/// [`in_its_slot`] in a repository whose Manifest declares its walks.
pub(super) async fn walking_slot() -> (Rig, SessionId) {
    in_its_slot_fitted(&Placing::nothing(), |fittings| {
        let home = fittings.host.home.clone();
        fittings.starting_in.as_mut().expect("a served repository").manifest = declared(&home);
    })
    .await
}

/// Fleet builds as `script` says, in place of cargo: the script the declared `build` runs.
pub(super) fn builds_with(rig: &Rig, script: String) {
    std::fs::write(Path::new(&rig.fleet.host().home).join(BUILD_SCRIPT), script).expect("a build script");
}

/// Where the build of `commit` is kept in the rig's repository.
pub(super) fn kept_at(rig: &Rig, commit: &str) -> PathBuf {
    let id = rig.manifest.as_str();
    let walks = rig.fleet.walks_declared(id).expect("declared");
    crate::walk_builds::binary_of(&rig.fleet.host().home, &crate::walk_builds::key_of(id), commit, &walks)
}

/// The walk's build on the newest open row of `url`.
pub(super) async fn build_of(rig: &Rig, id: &SessionId, url: &str) -> Option<WalkBuild> {
    rig.rows(id).await.into_iter().rev().find_map(|row| match row {
        SessionRow::Window { url: shown, ended_at: None, build, .. } if shown == url => Some(build),
        _ => None,
    })?
}

async fn state_of(rig: &Rig, id: &SessionId, url: &str) -> Option<WalkBuildState> {
    build_of(rig, id, url).await.map(|build| build.state)
}

/// The notice loop's looks, until no build is waiting or running.
pub(super) async fn built_all(rig: &Rig) {
    until(|| async {
        rig.fleet.walk_builds_driven().await;
        !rig.fleet.walk_builds().busy()
    })
    .await;
}

/// Show the walk and let its build end.
pub(super) async fn shown_ready(rig: &Rig, id: &SessionId, url: &str) {
    rig.fleet.show_window(None, shown(id, url)).await.expect("shown");
    built_all(rig).await;
    assert_eq!(state_of(rig, id, url).await, Some(WalkBuildState::Ready), "{:?}", build_of(rig, id, url).await);
}

/// The ids on his list.
async fn waiting_ids(rig: &Rig, id: &SessionId) -> Vec<String> {
    let record = std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("the Job").session;
    record.waiting_for.into_iter().map(|item| item.id).collect()
}

/// A job holding a slot with a commit, whose build plays both walks, with a native walk shown and built.
async fn walking() -> (Rig, SessionId) {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["sidebar", "footer"]));
    shown_ready(&rig, &id, URL).await;
    (rig, id)
}

/// Every line the walks' apps wrote when they started, in every build kept.
fn starts(rig: &Rig) -> Vec<String> {
    let bins = crate::runtime::support_dir(&rig.fleet.host().home).join(crate::walk_builds::DIR);
    let mut lines = Vec::new();
    for repository in std::fs::read_dir(bins).into_iter().flatten().flatten() {
        for commit in std::fs::read_dir(repository.path().join("bins")).into_iter().flatten().flatten() {
            if let Ok(text) = std::fs::read_to_string(commit.path().join("starts.txt")) {
                lines.extend(text.lines().map(str::to_string));
            }
        }
    }
    lines
}

/// Press Open and wait for the app to say it is up, `nth` of them in all.
async fn opened(rig: &Rig, id: &SessionId, url: &str, nth: usize) {
    rig.fleet.open_native_walk(press(id, url)).await.expect("opened");
    until(|| async { starts(rig).len() == nth }).await;
}

fn running(rig: &Rig, id: &SessionId, url: &str) -> bool {
    rig.fleet.native_walks().running(id.as_str(), url)
}

#[tokio::test]
async fn show_window_takes_a_native_walk_address_and_refuses_what_only_a_web_page_takes_on_it() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
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

/// The Manifest of another repository: its own app, built to `out/app.sh`, named and started in its own words.
fn another_repositorys(home: &str, take: &str) -> config::Manifest {
    let text = format!(
        "version: 1\nid: 01FIXTUREMANIFEST\nwalks:\n  build: sh {home}/{BUILD_SCRIPT}\n  binary: out/app.sh\n  list: ${{binary}} names\n  \
         open: ${{binary}} play ${{walk}} for ${{job}} at ${{url}} on ${{fleet_port}}\n{take}  shots: ${{binary}} snap ${{walk}} ${{shots}}\n  \
         against: ${{binary}} snap ${{walk}} ${{shots}} ${{against}} ${{out}}\n"
    );
    config::Manifest::parse(Path::new("armada.yml"), &text).expect("another repository's walks")
}

/// What that repository's build writes as its app: `names` lists two walks, anything else records how it was started.
const ANOTHER_APP: &str = "mkdir -p out && cat > out/app.sh <<'EOF'\n\
     #!/bin/sh\n\
     if [ \"$1\" = names ]; then echo sidebar; echo footer; exit 0; fi\n\
     echo \"$$ $(pwd -P) $@\" >> starts.txt\n\
     exec sleep 60\n\
     EOF\n\
     chmod +x out/app.sh";

#[tokio::test]
async fn a_repository_declaring_its_own_walk_is_built_readied_and_opened_through_the_declaration() {
    let (rig, id) = in_its_slot_fitted(&Placing::nothing(), |fittings| {
        let home = fittings.host.home.clone();
        fittings.starting_in.as_mut().expect("a served repository").manifest = another_repositorys(&home, "  take: with ${name} as ${walk}\n");
    })
    .await;
    leased(&rig, &id).await;
    builds_with(&rig, ANOTHER_APP.into());
    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];
    rig.fleet.show_window(None, ShowWindow { variations: takes, ..shown(&id, URL) }).await.expect("shown");
    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Building));

    built_all(&rig).await;

    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Ready), "{:?}", build_of(&rig, &id, URL).await);
    opened(&rig, &id, URL, 1).await;
    let started = starts(&rig).remove(0);
    let said = format!("play sidebar for {} at {URL} on 12345 with Sidebar as sidebar with Footer as footer", id.as_str());
    assert!(started.ends_with(&said), "started as the declaration says: {started}");
    assert!(!started.contains("--walk"), "none of the native Bridge's own flags: {started}");
    let commit = build_of(&rig, &id, URL).await.expect("a build").commit;
    let kept = kept_at(&rig, &commit);
    assert_eq!(kept.file_name().and_then(|name| name.to_str()), Some("app.sh"), "kept under the name the declaration gives");
    assert!(kept.is_file());
}

#[tokio::test]
async fn a_repository_with_no_walks_that_holds_the_native_bridge_builds_through_the_built_in_declaration() {
    let (rig, id) = in_its_slot_fitted(&Placing::nothing(), |fittings| {
        // A `cargo` that writes the native Bridge's stand-in where the real one leaves it.
        let bin = Path::new(&fittings.host.home).join("bin");
        std::fs::create_dir_all(&bin).expect("a folder");
        std::fs::write(bin.join("cargo"), format!("#!/bin/sh\n{}\n", script(&["sidebar", "footer"]))).expect("a cargo");
        std::fs::set_permissions(bin.join("cargo"), std::fs::Permissions::from_mode(0o755)).expect("executable");
        fittings.host.path = format!("{}:/usr/bin:/bin", bin.display());
    })
    .await;
    std::fs::create_dir_all(Path::new(&rig.root).join("crates/bridge-native")).expect("the crate");
    std::fs::write(Path::new(&rig.root).join("crates/bridge-native/Cargo.toml"), "[package]\n").expect("its manifest");
    leased(&rig, &id).await;

    shown_ready(&rig, &id, URL).await;

    opened(&rig, &id, URL, 1).await;
    let started = starts(&rig).remove(0);
    assert!(started.ends_with(&format!("--walk sidebar --job {} --url {URL} --fleet-port 12345", id.as_str())), "{started}");
}

#[tokio::test]
async fn a_repository_whose_manifest_declares_no_walks_refuses_a_native_walk_and_names_the_field() {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    leased(&rig, &id).await;

    let refused = said(rig.fleet.show_window(None, shown(&id, URL)).await);

    assert!(refused.contains("`walks:`") && refused.contains("armada.yml"), "{refused}");
    assert_eq!(rig.fleet.walk_open(id.as_str()).await, None, "nothing was written");
    rig.fleet.show_window(None, shown(&id, "http://localhost:5173/x")).await.expect("a page is still shown");
}

#[tokio::test]
async fn a_walk_with_takes_is_refused_where_the_declaration_has_no_take() {
    let (rig, id) = in_its_slot_fitted(&Placing::nothing(), |fittings| {
        let home = fittings.host.home.clone();
        fittings.starting_in.as_mut().expect("a served repository").manifest = another_repositorys(&home, "");
    })
    .await;
    leased(&rig, &id).await;
    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];

    let refused = said(rig.fleet.show_window(None, ShowWindow { variations: takes, ..shown(&id, URL) }).await);

    assert!(refused.contains("walks.take"), "{refused}");
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("a walk with no takes is shown");
}

#[tokio::test]
async fn a_native_walk_the_build_he_runs_lacks_is_shown_and_its_row_says_building_at_the_jobs_commit() {
    let (rig, id) = walking_slot().await;
    let slot = leased(&rig, &id).await;
    assert!(!build(&rig).join(APP).exists(), "he has no build that plays it");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("accepted");

    let building = build_of(&rig, &id, URL).await.expect("a build on the row");
    assert_eq!((building.state, building.commit.as_str()), (WalkBuildState::Building, git(&slot, &["rev-parse", "HEAD"]).as_str()));
    assert!(!waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "nothing asks him while it builds");
}

#[tokio::test]
async fn a_build_that_plays_the_walk_and_its_takes_is_ready_and_only_then_asks_him() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["sidebar", "footer"]));
    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];
    rig.fleet.show_window(None, ShowWindow { variations: takes, ..shown(&id, URL) }).await.expect("shown");
    assert!(!waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "building");

    built_all(&rig).await;

    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Ready));
    assert!(waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "ready: it asks him");
}

#[tokio::test]
async fn a_build_that_does_not_play_the_walk_fails_names_it_to_the_job_and_asks_him_nothing() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["footer"]));
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");

    built_all(&rig).await;

    let failed = build_of(&rig, &id, URL).await.expect("a build");
    assert_eq!(failed.state, WalkBuildState::Failed);
    assert!(failed.why.as_deref().is_some_and(|why| why.contains("has no walk named sidebar")), "{failed:?}");
    assert!(!waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "never a Needs you item for him");
    let sent = || rig.stand_in.process(1).sent.lock().unwrap().clone();
    until(|| async { sent().iter().any(|line| line.contains("has no walk named sidebar") && line.contains(URL)) }).await;
}

#[tokio::test]
async fn a_failed_cargo_build_is_told_as_its_last_twenty_lines() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, "for n in $(seq 1 30); do echo \"line $n\" >&2; done; exit 101".into());
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");

    built_all(&rig).await;

    let why = build_of(&rig, &id, URL).await.and_then(|build| build.why).expect("why it failed");
    assert!(why.contains("line 30") && why.contains("line 11") && !why.contains("line 10\n"), "{why}");
    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Failed));
}

#[tokio::test]
async fn a_job_that_holds_no_slot_has_nothing_to_build_and_is_told() {
    let (rig, id) = walking_slot().await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = None;
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");

    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    rig.fleet.walk_builds_driven().await;

    let failed = build_of(&rig, &id, URL).await.expect("a build");
    assert_eq!(failed.state, WalkBuildState::Failed);
    assert!(failed.why.as_deref().is_some_and(|why| why.contains("holds no slot")), "{failed:?}");
    let sent = || rig.stand_in.process(1).sent.lock().unwrap().clone();
    until(|| async { sent().iter().any(|line| line.contains("holds no slot")) }).await;
}

#[tokio::test]
async fn a_branch_that_moved_is_built_again_where_the_jobs_turn_ends_and_the_press_opens_the_new_build() {
    let (rig, id) = walking().await;
    let slot = PathBuf::from(adapter_traits::slot_path(&rig.root, 1));
    opened(&rig, &id, URL, 1).await;
    let first = build_of(&rig, &id, URL).await.expect("a build").commit;

    let moved = commit(&slot, "second");
    rig.fleet
        .report_session(ipc::SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact: ipc::SessionFact::TurnCompleted { said: None } })
        .await
        .expect("the turn ended");

    let building = build_of(&rig, &id, URL).await.expect("a build");
    assert_eq!((building.state, building.commit.as_str()), (WalkBuildState::Building, moved.as_str()), "no one asked");
    assert!(!waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "nothing asks him while it builds again");
    let refused = said(rig.fleet.open_native_walk(press(&id, URL)).await);
    assert!(refused.contains("still building"), "{refused}");

    built_all(&rig).await;

    assert_eq!(build_of(&rig, &id, URL).await.map(|build| (build.state, build.commit)), Some((WalkBuildState::Ready, moved.clone())));
    assert_ne!(first, moved);
    let opened_again = rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened");
    assert!(opened_again.started, "the newer commit stops the walk of the older and starts again");
    until(|| async { starts(&rig).len() == 2 }).await;
    for commit in [&first, &moved] {
        assert!(kept_at(&rig, commit).is_file(), "the first build stays beside the second: {commit}");
    }
}

#[tokio::test]
async fn a_walk_left_building_by_a_fleet_that_stopped_is_built_when_fleet_starts_again() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["sidebar", "footer"]));
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    // The queue is Fleet's memory alone: a restart loses the ask and leaves the row reading Building.
    let lost = rig.fleet.walk_builds().next_for_test().expect("the build was asked for");
    rig.fleet.walk_builds().done_for_test();
    assert_eq!(lost.url, URL);
    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Building));

    built_all(&rig).await;

    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Ready), "nothing built the walk a restart left Building");
    assert!(waiting_ids(&rig, &id).await.contains(&format!("walk:{URL}")), "it asks him once built");
}

#[tokio::test]
async fn a_walk_that_failed_is_refused_on_the_press_with_why() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, "echo broke >&2; exit 1".into());
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");

    let building = said(rig.fleet.open_native_walk(press(&id, URL)).await);
    assert!(building.contains("still building"), "{building}");
    built_all(&rig).await;
    let failed = said(rig.fleet.open_native_walk(press(&id, URL)).await);

    assert!(failed.contains("did not build") && failed.contains("broke"), "{failed}");
    assert!(!running(&rig, &id, URL));
}

#[tokio::test]
async fn a_build_gone_since_the_walk_was_ready_is_built_again_and_the_press_is_refused() {
    let (rig, id) = walking().await;
    let commit = build_of(&rig, &id, URL).await.expect("a build").commit;
    std::fs::remove_file(kept_at(&rig, &commit)).expect("gone");

    let refused = said(rig.fleet.open_native_walk(press(&id, URL)).await);

    assert!(refused.contains("no longer kept"), "{refused}");
    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Building));
    built_all(&rig).await;
    assert_eq!(state_of(&rig, &id, URL).await, Some(WalkBuildState::Ready));
    assert!(rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened").started);
}

#[tokio::test]
async fn a_row_written_before_walks_were_built_runs_from_the_build_he_keeps() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["sidebar"]));
    rig.fleet.show_window(None, shown(&id, URL)).await.expect("shown");
    let row = rig.rows(&id).await.into_iter().rev().find(|row| matches!(row, SessionRow::Window { .. })).expect("the row");
    let SessionRow::Window { id: row_id, at, title, url, variations, steps, origins, brief, group, ended_at, .. } = row else { unreachable!() };
    rig.fleet.row_put(id.as_str(), SessionRow::Window { id: row_id, at, title, url, variations, steps, origins, brief, group, build: None, ended_at }).await;
    let binary = build(&rig).join(APP);
    std::fs::create_dir_all(binary.parent().unwrap()).expect("the app's folder");
    std::fs::write(&binary, "#!/bin/sh\necho \"$$ $(pwd -P) $@\" >> starts.txt\nexec sleep 60\n").expect("a build");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("executable");

    rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened");

    until(|| async { std::fs::read_to_string(build(&rig).join("starts.txt")).is_ok_and(|text| text.lines().count() == 1) }).await;
    assert!(running(&rig, &id, URL));
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
async fn the_walk_runs_as_armada_from_a_bundle_of_its_own_around_the_jobs_built_app() {
    use std::os::unix::fs::MetadataExt;
    let (rig, id) = walking().await;
    opened(&rig, &id, URL, 1).await;
    let bundle = crate::runtime::support_dir(&rig.fleet.host().home).join("walk/Armada.app/Contents");
    let info = std::fs::read_to_string(bundle.join("Info.plist")).expect("an Info.plist");
    assert!(info.contains("<key>CFBundleName</key><string>Armada</string>"), "{info}");
    assert!(info.contains("dev.armada.bridge.native.walk"), "apart from his running native Bridge: {info}");
    let linked = std::fs::metadata(bundle.join("MacOS/Armada")).expect("the executable");
    let commit = build_of(&rig, &id, URL).await.expect("a build").commit;
    let app = std::fs::metadata(kept_at(&rig, &commit)).expect("the built app");
    assert_eq!(linked.ino(), app.ino(), "a hard link to the Job's build, not a copy");
}

#[tokio::test]
async fn a_native_walk_opens_beside_a_page_shown_after_it() {
    let (rig, id) = walking().await;
    rig.fleet.show_window(None, shown(&id, "http://localhost:5792/?scenario=compare")).await.expect("a second tab");
    let opened = rig.fleet.open_native_walk(press(&id, URL)).await.expect("its tab is still open");
    assert!(opened.started);
}

#[tokio::test]
async fn two_presses_start_one_app_from_the_jobs_build_with_the_walk_the_job_the_address_and_fleets_port() {
    let (rig, id) = walking().await;

    let first = rig.fleet.open_native_walk(press(&id, URL)).await.expect("opened");
    let second = rig.fleet.open_native_walk(press(&id, URL)).await.expect("still ok");

    assert!(first.started && !second.started);
    until(|| async { starts(&rig).len() == 1 }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let line = starts(&rig);
    assert_eq!(line.len(), 1, "one process: {line:?}");
    let port = rig.fleet.host().port;
    let commit = build_of(&rig, &id, URL).await.expect("a build").commit;
    assert!(
        line[0].contains(&format!("/bins/{commit} --walk sidebar --job {} --url {URL} --fleet-port {port}", id.as_str())),
        "started in the Job's build, with the arguments the app reads: {line:?}"
    );
    assert!(running(&rig, &id, URL));
}

#[tokio::test]
async fn a_native_walks_variations_are_other_native_walks_and_the_app_starts_with_each_as_a_take() {
    let (rig, id) = walking_slot().await;
    leased(&rig, &id).await;
    builds_with(&rig, script(&["sidebar", "footer"]));
    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];
    rig.fleet.show_window(None, ShowWindow { variations: takes, ..shown(&id, URL) }).await.expect("takes of one walk");
    built_all(&rig).await;

    opened(&rig, &id, URL, 1).await;

    let line = starts(&rig);
    assert!(line[0].contains(&format!("--url {URL} --fleet-port {} --take Sidebar=sidebar --take Footer=footer", rig.fleet.host().port)), "{line:?}");
}

#[tokio::test]
async fn the_app_goes_when_the_walk_is_closed_or_approved_and_stays_beside_another_tab() {
    let (rig, id) = walking().await;

    opened(&rig, &id, URL, 1).await;
    rig.fleet.window_closed(WindowClosed { session_id: id.clone(), url: URL.into() }).await.expect("closed");
    assert!(!running(&rig, &id, URL), "closed");

    shown_ready(&rig, &id, URL).await;
    opened(&rig, &id, URL, 2).await;
    shown_ready(&rig, &id, OTHER).await;
    assert!(running(&rig, &id, URL), "a page shown beside it leaves it running");

    opened(&rig, &id, OTHER, 3).await;
    rig.send(&id, &format!("Approved: {OTHER}")).await;
    assert!(!running(&rig, &id, OTHER), "approved");
    assert!(running(&rig, &id, URL), "approving one walk leaves the other");

    shown_ready(&rig, &id, OTHER).await;
    opened(&rig, &id, OTHER, 4).await;
    assert!(running(&rig, &id, OTHER), "opened again after its app went");
}

#[tokio::test]
async fn the_app_goes_when_the_job_ends_and_fleet_stops() {
    let (rig, id) = walking().await;

    opened(&rig, &id, URL, 1).await;
    rig.fleet.session_wound_up(id.as_str()).await;
    assert!(!running(&rig, &id, URL), "the job ended");

    shown_ready(&rig, &id, URL).await;
    opened(&rig, &id, URL, 2).await;
    assert!(running(&rig, &id, URL));
    rig.fleet.native_walks_stopped();
    assert!(!running(&rig, &id, URL), "fleet stopped");
}

/// A plan of three groups, the first and last gated on his approval, so which gate a press closes shows.
async fn planned_on_his_approve(rig: &Rig, id: &SessionId) {
    let gated = |title: &str, gate| ipc::PlanGroupInput {
        gate,
        ..super::session_plan::group(None, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))])
    };
    let groups = vec![gated("Walk", ipc::PlanGate::YouApproved), gated("Build", ipc::PlanGate::None), gated("Sign off", ipc::PlanGate::YouApproved)];
    rig.fleet.record_plan(None, super::session_plan::plan(id, groups), false).await.expect("recorded");
    // The clock reads milliseconds: let the press land after the plan.
    tokio::time::sleep(Duration::from_millis(5)).await;
}

async fn closed_gates(rig: &Rig, id: &SessionId) -> Vec<bool> {
    let record = std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("the Job").session;
    record.plan.expect("a plan").groups.iter().map(|group| group.gate_closed.is_some()).collect()
}

#[tokio::test]
async fn ship_it_ends_the_walk_closes_the_first_you_approved_gate_and_tells_the_job_to_land_without_a_preview() {
    let (rig, id) = walking().await;
    planned_on_his_approve(&rig, &id).await;
    opened(&rig, &id, URL, 1).await;

    rig.send(&id, &format!("Shipped: {URL}")).await;

    assert!(!running(&rig, &id, URL), "the walk ended");
    assert!(rig.fleet.walk_open(id.as_str()).await.is_none(), "the window is closed");
    assert_eq!(closed_gates(&rig, &id).await, [true, false, false], "the first open you_approved gate and no other");
    let sent = || rig.stand_in.process(1).sent.lock().unwrap().clone();
    until(|| async { sent().iter().any(|line| line.contains("He pressed Ship it: land the change without offering a preview.")) }).await;
    assert!(sent().iter().all(|line| !line.contains("then offer a preview")), "it was not told the other");
}

#[tokio::test]
async fn approve_ends_the_walk_closes_no_gate_and_tells_the_job_to_wire_it_then_offer_a_preview() {
    let (rig, id) = walking().await;
    planned_on_his_approve(&rig, &id).await;
    opened(&rig, &id, URL, 1).await;

    rig.send(&id, &format!("Approved: {URL}")).await;

    assert!(rig.fleet.walk_open(id.as_str()).await.is_none());
    assert_eq!(closed_gates(&rig, &id).await, [false, false, false], "Approve is not Ship it");
    let sent = || rig.stand_in.process(1).sent.lock().unwrap().clone();
    until(|| async { sent().iter().any(|line| line.contains("He approved the look: wire it, then offer a preview.")) }).await;
}

#[test]
fn ship_it_and_approve_both_name_the_address_in_the_two_shapes_the_bar_sends() {
    use crate::session_host::walking::{approved_address, walk_pressed, Pressed};
    assert_eq!(walk_pressed("Shipped: armada://walk/sidebar"), Some((Pressed::ShipIt, "armada://walk/sidebar")));
    assert_eq!(walk_pressed("Shipped B, Needs you first: armada://walk/sidebar"), Some((Pressed::ShipIt, "armada://walk/sidebar")));
    assert_eq!(walk_pressed("Approved: armada://walk/sidebar\nmore"), Some((Pressed::Approve, "armada://walk/sidebar")));
    assert_eq!(approved_address("Shipped: armada://walk/sidebar"), Some("armada://walk/sidebar"));
    assert_eq!(approved_address("Shipped it, thanks"), None);
    assert_eq!(approved_address("Shipping: armada://walk/sidebar"), None);
}

const MESSAGE: &str = "{session_id: string, text: string, attachments?: [{name: string, media_type: string, data: string}], \
mentions?: [{kind: session|pull_request|branch|issue|report|terminal|tool|request, id: string, title: string, url?: string}], \
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
    // Ship it, as the bar sends it: Fleet reads it as the end of the walk too.
    assert_eq!(crate::session_host::walking::approved_address("Shipped: armada://walk/sidebar"), Some("armada://walk/sidebar"));
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
