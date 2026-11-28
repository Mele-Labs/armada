//! **The look he approved is kept, and the screen that lands is checked against it.** His Ship it or
//! Approve takes the pictures of the walk from the build that was Ready; a Job's pull request does not
//! open on a screen that drifted from them. `crates/fleet/src/approved_looks.rs`,
//! `crates/fleet/src/approved_looks_checking.rs`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{CheckBeforePr, PrePrChecked, SessionId};

use super::native_walk::{build_of, builds_with, built_all, commit, leased, shown, shown_ready, until, OTHER, URL};
use super::session_gate::in_its_slot;
use super::session_host::Rig;
use crate::tests::peer::Placing;

/// What a build writes as the native Bridge: it plays `walks`. For `--shots` it writes one picture of
/// fixed bytes and the name of the build's folder, and for `--against` it writes a report, drifted where
/// the file `drift` is. Each `--against` is logged in `log`, so a test can see nothing ran.
fn looking(drift: &Path, log: &Path) -> String {
    let (drift, log) = (drift.display(), log.display());
    format!(
        "mkdir -p crates/bridge-native/target/release && cat > crates/bridge-native/target/release/bridge-native <<'EOF'\n\
         #!/bin/sh\n\
         if [ \"$1\" = --list-walks ]; then echo sidebar; echo footer; exit 0; fi\n\
         shots=; against=; out=\n\
         while [ $# -gt 0 ]; do case \"$1\" in --shots) shots=$2;; --against) against=$2;; --out) out=$2;; esac; shift; done\n\
         if [ -n \"$shots\" ]; then mkdir -p \"$shots\"; printf 'PNG:%s' \"$(basename \"$(pwd -P)\")\" > \"$shots/step-01.png\"; fi\n\
         if [ -n \"$against\" ]; then\n\
           echo against >> {log}\n\
           if [ -f {drift} ]; then\n\
             echo '{{\"walk\":\"sidebar\",\"steps\":[{{\"step\":1,\"file\":\"step-01.png\",\"drifted\":true,\"mean\":0.1,\"cells\":0.25,\"side_by_side\":\"step-01.side.png\",\"difference\":\"step-01.diff.png\"}}],\"drifted\":true}}' > \"$out/report.json\"\n\
             exit 1\n\
           fi\n\
           echo '{{\"walk\":\"sidebar\",\"steps\":[{{\"step\":1,\"file\":\"step-01.png\",\"drifted\":false,\"mean\":0.0,\"cells\":0.0}}],\"drifted\":false}}' > \"$out/report.json\"\n\
         fi\n\
         exit 0\n\
         EOF\n\
         chmod +x crates/bridge-native/target/release/bridge-native"
    )
}

struct Looking {
    rig: Rig,
    id: SessionId,
    slot: PathBuf,
    drift: PathBuf,
    log: PathBuf,
}

/// A Job whose native walk is built and Ready, in a build that plays the two walks.
async fn looking_at_a_walk() -> Looking {
    let (rig, id) = in_its_slot(&Placing::nothing()).await;
    let slot = leased(&rig, &id).await;
    let marks = std::env::temp_dir().join(format!("looks-{}", id.as_str()));
    std::fs::create_dir_all(&marks).expect("a folder");
    let (drift, log) = (marks.join("drift"), marks.join("against.log"));
    builds_with(&rig, looking(&drift, &log));
    shown_ready(&rig, &id, URL).await;
    Looking { rig, id, slot, drift, log }
}

impl Looking {
    async fn kept(&self) -> Vec<store::KeptLook> {
        self.rig.fleet.store().lock().await.kept_looks(self.id.as_str()).expect("read")
    }

    /// The build the walk's row carries now.
    async fn built_at(&self, url: &str) -> String {
        build_of(&self.rig, &self.id, url).await.expect("a build").commit
    }

    /// Wait for the look of `walk` to be kept at `commit`.
    async fn kept_at(&self, walk: &str, commit: &str) -> store::KeptLook {
        until(|| async { self.kept().await.iter().any(|one| one.walk == walk && one.commit == commit) }).await;
        self.kept().await.into_iter().find(|one| one.walk == walk).expect("kept")
    }

    /// The pre-PR answer, while the notice loop's looks drive the build queue.
    async fn pre_pr(&self) -> PrePrChecked {
        let asked = CheckBeforePr { session_id: Some(self.id.clone()) };
        let driver = async {
            loop {
                self.rig.fleet.walk_builds_driven().await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        tokio::select! {
            answered = self.rig.fleet.pre_pr_checked(None, asked) => answered.expect("answered"),
            () = driver => unreachable!("never ends"),
        }
    }

    fn compared(&self) -> usize {
        std::fs::read_to_string(&self.log).map(|text| text.lines().count()).unwrap_or(0)
    }
}

fn picture(look: &store::KeptLook) -> String {
    std::fs::read_to_string(Path::new(&look.folder).join("step-01.png")).expect("a picture")
}

#[tokio::test]
async fn ship_it_keeps_the_pictures_of_the_walk_from_the_commit_of_the_build_that_was_ready() {
    let it = looking_at_a_walk().await;
    let commit = it.built_at(URL).await;

    it.rig.send(&it.id, &format!("Shipped: {URL}")).await;

    let look = it.kept_at("sidebar", &commit).await;
    assert_eq!((look.url.as_str(), look.session_id.as_str()), (URL, it.id.as_str()));
    assert_eq!(picture(&look), format!("PNG:{commit}"), "taken by the binary of that commit");
    assert!(look.folder.contains("approved-looks"), "{}", look.folder);
}

#[tokio::test]
async fn approve_keeps_them_too_and_a_take_keeps_the_walk_of_the_take_he_pressed_on() {
    let it = looking_at_a_walk().await;
    let commit = it.built_at(URL).await;
    it.rig.send(&it.id, &format!("Approved: {URL}")).await;
    it.kept_at("sidebar", &commit).await;

    let takes = vec![ipc::WindowTake { name: "Sidebar".into(), url: URL.into() }, ipc::WindowTake { name: "Footer".into(), url: OTHER.into() }];
    it.rig.fleet.show_window(None, ipc::ShowWindow { variations: takes, ..shown(&it.id, URL) }).await.expect("takes of one walk");
    built_all(&it.rig).await;
    it.rig.send(&it.id, &format!("Approved B, Footer: {URL}")).await;

    let look = it.kept_at("footer", &commit).await;
    assert_eq!(look.url, URL, "the address his press named");
    assert_eq!(it.kept().await.len(), 2, "the take's walk beside the first");
}

#[tokio::test]
async fn a_second_approval_of_the_same_walk_replaces_the_older_pictures() {
    let it = looking_at_a_walk().await;
    let first = it.built_at(URL).await;
    it.rig.send(&it.id, &format!("Approved: {URL}")).await;
    let older = it.kept_at("sidebar", &first).await;

    let second = commit(&it.slot, "second");
    shown_ready(&it.rig, &it.id, URL).await;
    assert_eq!(it.built_at(URL).await, second);
    it.rig.send(&it.id, &format!("Shipped: {URL}")).await;

    let newer = it.kept_at("sidebar", &second).await;
    assert_eq!(it.kept().await.len(), 1, "one look a walk");
    assert_eq!(newer.folder, older.folder, "in the same place");
    assert_eq!(picture(&newer), format!("PNG:{second}"), "the pictures are the newer build's");
}

#[tokio::test]
async fn pictures_that_could_not_be_taken_keep_nothing_and_the_job_is_told() {
    let it = looking_at_a_walk().await;
    let commit = it.built_at(URL).await;
    let key = crate::walk_builds::key_of(&it.rig.fleet.session_and_hosting(it.id.as_str()).await.expect("hosted").1.manifest_id);
    let binary = crate::walk_builds::binary_of(&it.rig.fleet.host().home, &key, &commit);
    std::fs::write(&binary, "#!/bin/sh\nexit 2\n").expect("a binary that cannot play");

    it.rig.send(&it.id, &format!("Shipped: {URL}")).await;

    let sent = || it.rig.stand_in.process(1).sent.lock().unwrap().clone();
    until(|| async { sent().iter().any(|line| line.contains("could not be kept")) }).await;
    assert!(it.kept().await.is_empty(), "nothing is kept");
}

#[tokio::test]
async fn the_pre_pr_check_passes_where_the_screen_is_as_he_approved_it() {
    let it = looking_at_a_walk().await;
    let approved = it.built_at(URL).await;
    it.rig.send(&it.id, &format!("Shipped: {URL}")).await;
    it.kept_at("sidebar", &approved).await;
    commit(&it.slot, "moved");

    let answered = it.pre_pr().await;

    assert!(answered.passed, "{}", answered.says);
    assert_eq!(answered.checks.iter().map(|one| (one.name.as_str(), one.passed)).collect::<Vec<_>>(), [("look:sidebar", true)]);
    assert_eq!(it.compared(), 1, "the walk was played against the pictures once");
}

#[tokio::test]
async fn the_pre_pr_check_fails_where_a_step_drifted_naming_the_pictures_to_look_at() {
    let it = looking_at_a_walk().await;
    let approved = it.built_at(URL).await;
    it.rig.send(&it.id, &format!("Shipped: {URL}")).await;
    it.kept_at("sidebar", &approved).await;
    commit(&it.slot, "moved");
    std::fs::write(&it.drift, "").expect("drift");

    let answered = it.pre_pr().await;

    assert!(!answered.passed);
    assert!(answered.says.contains("look:sidebar failed:"), "{}", answered.says);
    assert!(answered.says.contains("step 1 (step-01.png)"), "{}", answered.says);
    assert!(answered.says.contains(".against/step-01.side.png"), "{}", answered.says);
    assert!(answered.says.contains(".against/step-01.diff.png"), "{}", answered.says);
    assert!(answered.says.contains("show the walk again"), "{}", answered.says);
}

#[tokio::test]
async fn a_job_with_no_kept_look_has_nothing_run_before_its_pull_request_opens() {
    let it = looking_at_a_walk().await;
    commit(&it.slot, "moved");

    let answered = it.pre_pr().await;

    assert!(answered.passed, "{}", answered.says);
    assert!(answered.checks.is_empty());
    assert_eq!(it.compared(), 0);
    assert!(!it.rig.fleet.walk_builds().busy(), "no build was asked for");
}
