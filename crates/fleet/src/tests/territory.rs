//! What Fleet serves the flight deck's map: each served repository's directories with their files,
//! read from git once for each head of the base, and what each Job or Session holding a slot has
//! changed in it. Against real git in a real repository; what git counts is `adapters`' to assert,
//! under test here is which repositories, which holders, how many reads and what is cut.

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use ipc::SlotHolderKind;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::fittings;
use super::http::call;
use super::tmp::TempDir;
use crate::daemon::Fleet;
use crate::slots::Concurrency;
use crate::territory::capped;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

struct Rig {
    fleet: Arc<Hosted>,
    home: TempDir,
}

fn write(home: &TempDir, file: &str, text: &str) {
    let path = home.path().join(file);
    std::fs::create_dir_all(path.parent().expect("a parent")).unwrap();
    std::fs::write(path, text).unwrap();
}

fn a_rig() -> Rig {
    let home = TempDir::new();
    git(home.path(), &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    for file in ["README.md", "crates/fleet/a.rs", "crates/fleet/b.rs", "crates/ipc/c.rs", "docs/d.md"] {
        write(&home, file, "x\n");
    }
    git(home.path(), &["add", "."]);
    git(home.path(), &["commit", "--quiet", "-m", "start"]);
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.concurrency = Concurrency::of(2);
    Rig {
        fleet: Arc::new(Fleet::assembled(fittings)),
        home,
    }
}

impl Rig {
    fn root(&self) -> String {
        self.fleet.repositories().first().expect("served").root().to_string()
    }

    fn manifest(&self) -> String {
        self.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
    }

    /// Slot `n` made as a worktree on its own branch and held by `by`, the way the pool reads a Job's.
    fn held_slot(&self, n: u32, by: &str) -> PathBuf {
        let at = PathBuf::from(adapter_traits::slot_path(&self.root(), n));
        git(
            self.home.path(),
            &["worktree", "add", "--quiet", "-b", &format!("branch-{n}"), &at.to_string_lossy()],
        );
        self.fleet.vcs().hold_slot(&self.root(), n, by);
        at
    }
}

#[test]
fn only_the_forty_largest_directories_are_listed_and_a_top_directory_is_never_cut_from_under_one_inside_it() {
    let mut dirs = Vec::new();
    for top in 0..30 {
        // Every directory inside a top one holds as many files as the top one, the tie that
        // would cut a parent first if parents were not listed first.
        dirs.push(adapters::territory::Directory { path: format!("t{top:02}"), files: 5 });
        dirs.push(adapters::territory::Directory { path: format!("t{top:02}/k"), files: 5 });
    }

    let (areas, omitted) = capped(dirs);

    let listed: usize = areas.iter().map(|area| 1 + area.kids.len()).sum();
    assert_eq!(listed, 40, "none of the forty was a directory whose top was cut");
    assert_eq!(omitted, 20);
    assert_eq!(areas.len(), 20);
    assert!(areas.iter().all(|area| area.kids.len() == 1), "each top came with the directory inside it");
}

#[tokio::test]
async fn a_slots_touches_are_the_paths_its_holder_changed_and_a_holder_with_no_slot_or_no_change_has_none() {
    let rig = a_rig();
    let busy = rig.held_slot(1, "01JOBBUSY");
    let _idle = rig.held_slot(2, "01JOBIDLE");
    std::fs::write(busy.join("crates/fleet/a.rs"), "edited\n").unwrap();
    std::fs::write(busy.join("new.rs"), "new\n").unwrap();

    let touches = rig.fleet.slot_touches().await.expect("read");

    let [here] = touches.repositories.as_slice() else {
        panic!("one repository is served: {touches:?}");
    };
    assert_eq!(here.manifest_id.as_str(), rig.manifest());
    let [holder] = here.holders.as_slice() else {
        panic!("only the Job that changed something: {here:?}");
    };
    assert_eq!((holder.kind, holder.id.as_str(), holder.slot), (SlotHolderKind::Session, "01JOBBUSY", 1));
    assert_eq!(holder.paths, ["crates/fleet/a.rs", "new.rs"]);
    assert_eq!(holder.omitted, 0);
}

#[tokio::test]
async fn a_holders_paths_are_cut_at_two_hundred_and_the_rest_counted() {
    let rig = a_rig();
    let busy = rig.held_slot(1, "01JOBBUSY");
    for n in 0..230 {
        std::fs::write(busy.join(format!("f{n:03}.txt")), "x\n").unwrap();
    }

    let touches = rig.fleet.slot_touches().await.expect("read");

    let holder = &touches.repositories[0].holders[0];
    assert_eq!(holder.paths.len(), 200);
    assert_eq!(holder.omitted, 30);
}

#[tokio::test]
async fn touches_read_again_within_the_short_wait_ask_git_for_nothing() {
    let rig = a_rig();
    let busy = rig.held_slot(1, "01JOBBUSY");
    std::fs::write(busy.join("new.rs"), "new\n").unwrap();

    let first = rig.fleet.slot_touches().await.expect("read");
    let again = rig.fleet.slot_touches().await.expect("read");

    assert_eq!(again, first);
    assert_eq!(rig.fleet.territory().slot_reads(), 1, "the one slot was read once");
}

#[tokio::test]
async fn both_reads_are_served_where_the_router_says() {
    let rig = a_rig();
    let app = api::router(api::Served::sharing(Arc::clone(&rig.fleet), ipc::RunId::carried("01RUN"), rig.fleet.events()));

    let (areas, areas_body) = call(&app, "GET", "/repository_areas", "").await;
    let (touches, touches_body) = call(&app, "GET", "/slot_touches", "").await;

    assert_eq!((areas, touches), (StatusCode::OK, StatusCode::OK));
    let areas: ipc::RepositoryAreas = ipc::decode("areas", &areas_body).expect("decodes");
    let touches: ipc::SlotTouches = ipc::decode("touches", &touches_body).expect("decodes");
    assert_eq!(areas.repositories.len(), 1);
    assert!(touches.repositories.is_empty() || touches.repositories[0].holders.is_empty());
}

fn git(repo: &std::path::Path, args: &[&str]) {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
}
