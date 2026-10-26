//! What `armada worktree --status` says of a slot a Session holds.

use std::path::PathBuf;

use adapters::leasing::{Holder, Slot, SlotState};

use crate::leasing::line;

fn held_by_a_session() -> Slot {
    Slot {
        number: 2,
        path: PathBuf::from("/repo/.armada/slots/slot-2"),
        state: SlotState::Held {
            branch: String::from("armada/1-fix"),
            holder: Holder::hosted("01SESSION"),
            since: 0,
        },
        closed: false,
    }
}

#[test]
fn a_slot_a_session_holds_reads_as_held() {
    let said = line(&held_by_a_session(), 600);
    assert!(said.starts_with("slot-2  held "), "{said}");
}

#[test]
fn a_closed_slot_says_so_beside_its_holder() {
    let slot = Slot {
        closed: true,
        ..held_by_a_session()
    };
    let said = line(&slot, 600);
    assert!(said.starts_with("slot-2 closed  held "), "{said}");
    assert!(said.contains("01SESSION"), "{said}");
}

/// `armada worktree lease` reads the pool fresh each time, the Manifest's
/// size and then this machine's own, so a slot closed or added from Bridge
/// is what the next lease finds.
#[test]
fn the_clis_lease_honours_a_closed_slot_and_this_machines_size() {
    use super::clean::a_repository;

    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();
    let pool = || crate::leasing::pool_of(repo.path()).expect("a pool");
    assert_eq!(pool().bays(), vec![1]);

    pool().close(1).expect("closed");
    let none = |_: &std::path::Path, _: &std::path::Path| Err(String::from("no seed"));
    let here = Holder::of(std::process::id()).expect("running");
    assert!(
        matches!(
            pool().try_lease("one", &here, 0, &none),
            Ok(adapters::leasing::Leased::Full(_))
        ),
        "a closed slot is not leased"
    );

    assert_eq!(pool().add().expect("added"), 2);
    assert_eq!(crate::leasing::lease(repo.path(), "one", false), 0);
    assert!(pool().path_of(2).is_dir(), "the added slot was made");
    assert!(!pool().path_of(1).exists(), "the closed one was not");
}

/// `--existing` leases a branch a release left behind, and refuses one that
/// does not exist, each with its own sentence and a nonzero exit.
#[test]
fn the_clis_lease_can_take_an_existing_branch() {
    use super::clean::a_repository;

    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();
    assert_eq!(crate::leasing::lease(repo.path(), "feature", false), 0);
    let slot = crate::leasing::pool_of(repo.path()).unwrap().path_of(1);
    assert_eq!(crate::leasing::release(repo.path(), Some(slot)), 0);

    assert_ne!(crate::leasing::lease(repo.path(), "nope", true), 0);
    assert_eq!(crate::leasing::lease(repo.path(), "feature", true), 0);
}

/// **A repository whose `armada.yml` is only Kit's copy leases a slot**, sized by that copy, as
/// `armada check` reads it. `docs/concepts/kit.md`, *What a repository runs*.
#[test]
fn a_repository_with_only_kits_armada_yml_leases_a_slot() {
    use super::clean::a_repository;
    use std::sync::Arc;

    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::remove_file(&manifest).unwrap();
    let kit = crate::tests::TempDir::new();
    kit.write("armada.yml", &format!("{declared}setup:\n  worktrees: 1\n"));
    let copy = |_: &std::path::Path| {
        Some(Arc::new(config::KitFile(kit.path().join("armada.yml"))) as Arc<dyn config::KitCopy>)
    };

    let pool = crate::leasing::pool_over(repo.path(), copy).expect("Kit's copy is the Manifest");
    assert_eq!(pool.bays(), vec![1], "sized by Kit's copy");
    let none = |_: &std::path::Path, _: &std::path::Path| Err(String::from("no seed"));
    let here = Holder::of(std::process::id()).expect("running");
    assert!(matches!(
        pool.try_lease("one", &here, 0, &none),
        Ok(adapters::leasing::Leased::Took(_))
    ));
}

/// A Fleet that answers `ask_for_slot` with `standing`, and keeps what it was told: each body.
mod fleet_asked {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub struct Heard {
        pub port: u16,
        pub bodies: Arc<Mutex<Vec<String>>>,
    }

    impl Heard {
        pub fn acts(&self) -> Vec<String> {
            let bodies = self.bodies.lock().unwrap();
            bodies
                .iter()
                .map(|body| body.split("\"act\":\"").nth(1).unwrap_or("").split('"').next().unwrap_or("").to_string())
                .collect()
        }
    }

    pub fn answering(standing: &'static str) -> Heard {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&bodies);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut raw = Vec::new();
                let mut chunk = [0u8; 1024];
                loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    raw.extend_from_slice(&chunk[..read]);
                    let text = String::from_utf8_lossy(&raw).to_string();
                    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
                    let wanted = head
                        .lines()
                        .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(|n| n.trim().parse::<usize>().unwrap()))
                        .unwrap_or(0);
                    if body.len() >= wanted || read == 0 {
                        kept.lock().unwrap().push(body.to_string());
                        break;
                    }
                }
                let body = format!("{{\"standing\":\"{standing}\"}}");
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Heard { port, bodies }
    }
}

/// A repository whose one slot another holder has, so the next lease waits.
fn a_pool_with_its_one_slot_held() -> (crate::tests::TempDir, std::path::PathBuf) {
    use super::clean::a_repository;
    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();
    let none = |_: &std::path::Path, _: &std::path::Path| Err(String::from("no seed"));
    let pool = crate::leasing::pool_of(repo.path()).expect("a pool");
    let held = pool.try_lease("theirs", &Holder::hosted("someone-else"), 0, &none);
    assert!(matches!(held, Ok(adapters::leasing::Leased::Took(_))), "the slot is taken: {held:?}");
    let slot = pool.path_of(1);
    (repo, slot)
}

use crate::slot_asking::Asking;

/// A lease on a thread, waiting; the test gives the slot back, or adds one, from here.
fn a_lease_waiting(repo: &std::path::Path, asker: Option<Asking>) -> std::thread::JoinHandle<u8> {
    let repo = repo.to_path_buf();
    std::thread::spawn(move || crate::leasing::lease_asking(&repo, "mine", false, move || asker))
}

fn until(mut come_true: impl FnMut() -> bool) {
    let by = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !come_true() {
        assert!(std::time::Instant::now() < by, "it did not come true within ten seconds");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// A lease that finds every slot held tells Fleet, again each second, and tells it when it is done
/// whether it leased or not. The slot it takes is the one given back.
#[test]
fn a_lease_that_waits_tells_fleet_and_says_it_is_done_when_it_leases() {
    let (repo, theirs) = a_pool_with_its_one_slot_held();
    let fleet = fleet_asked::answering("waiting");

    let lease = a_lease_waiting(repo.path(), Some(Asking::at(fleet.port, "01SESSION")));
    until(|| fleet.acts().contains(&"ask".to_string()));
    assert!(!lease.is_finished(), "it waits while the slot is held");
    assert_eq!(crate::leasing::release(repo.path(), Some(theirs)), 0);

    assert_eq!(lease.join().unwrap(), 0, "it leased the slot that was given back");
    let acts = fleet.acts();
    assert_eq!(acts.last().map(String::as_str), Some("done"), "{acts:?}");
    assert!(acts[..acts.len() - 1].iter().all(|act| act == "ask"), "{acts:?}");
    assert!(fleet.bodies.lock().unwrap()[0].contains("01SESSION"));
}

/// Added: Fleet made a slot, and the lease's own loop takes it on its next look.
#[test]
fn a_lease_that_is_told_a_slot_was_added_takes_it() {
    let (repo, _theirs) = a_pool_with_its_one_slot_held();
    let fleet = fleet_asked::answering("added");

    let lease = a_lease_waiting(repo.path(), Some(Asking::at(fleet.port, "01SESSION")));
    until(|| fleet.acts().contains(&"ask".to_string()));
    // Fleet made a slot; the lease's next look takes it.
    assert_eq!(crate::leasing::pool_of(repo.path()).unwrap().add().expect("added"), 2);

    assert_eq!(lease.join().unwrap(), 0);
    assert!(crate::leasing::pool_of(repo.path()).unwrap().path_of(2).is_dir());
    assert_eq!(fleet.acts().last().map(String::as_str), Some("done"));
}

/// No Fleet to ask, or a lease with no Session: it waits and leases as it always did, and tells nobody.
#[test]
fn a_lease_with_nobody_to_ask_waits_as_it_always_did() {
    let (repo, theirs) = a_pool_with_its_one_slot_held();

    let lease = a_lease_waiting(repo.path(), None);
    std::thread::sleep(std::time::Duration::from_millis(600));
    assert!(!lease.is_finished());
    assert_eq!(crate::leasing::release(repo.path(), Some(theirs)), 0);

    assert_eq!(lease.join().unwrap(), 0);
}

/// A lease that finds a slot at once never looks for anyone to ask.
#[test]
fn a_lease_that_finds_a_slot_does_not_ask() {
    use super::clean::a_repository;
    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();

    let found = crate::leasing::lease_asking(repo.path(), "mine", false, || panic!("nothing waited"));

    assert_eq!(found, 0);
}

/// `--status` names the branch the slot is on, which is where the work is, and says the leased one
/// beside it only where a session moved off it.
#[test]
fn status_names_the_branch_checked_out_and_the_leased_one_where_they_differ() {
    use super::clean::{a_repository, git};

    let repo = a_repository();
    let manifest = repo.path().join("armada.yml");
    let declared = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{declared}setup:\n  worktrees: 1\n")).unwrap();
    assert_eq!(crate::leasing::lease(repo.path(), "feature", false), 0);
    let pool = crate::leasing::pool_of(repo.path()).unwrap();
    let slot = pool.path_of(1);
    let said = || line(&pool.status()[0], 600);

    let on_it = said();
    assert!(on_it.contains("  feature  by "), "{on_it}");
    assert!(!on_it.contains("leased as"), "{on_it}");

    git(&slot, &["switch", "--quiet", "-c", "follow-up"]);
    let moved = said();
    assert!(moved.contains("  follow-up (leased as feature)  by "), "{moved}");

    git(&slot, &["switch", "--quiet", "--detach"]);
    let detached = said();
    assert!(detached.contains("  detached (leased as feature)  by "), "{detached}");
}
