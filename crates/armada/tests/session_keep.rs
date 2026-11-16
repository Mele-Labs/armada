//! `armada session-keep`, the binary, twice for one Job. What is proved is what the keeper's own tests
//! cannot, since they share the test's process: a second keeper process of the same Job starts no
//! second agent, and a keeper whose socket was taken away is found by its lock and stopped, with its
//! agent, before Fleet starts another (`fleet::session_host::keeper::make_way`).
//!
//! | Proved | Not proved |
//! |---|---|
//! | A second keeper of one Job exits without starting an agent | That Fleet's start reaches `make_way`. It is its first act after the paths |
//! | `make_way` adopts a keeper that answers, and stops one that does not | Keepers from builds before the lock, which hold none |

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use fleet::session_host::keeper;

/// A short folder only this user may open: a socket path must fit in 104 bytes.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        let path = PathBuf::from(format!("/tmp/armada-sk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch folder");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A keeper of the Job whose socket is `socket`, as Fleet starts one: its own group, its agent a
/// shell that writes its pid to `marks` and then echoes its input.
fn keeper_of(socket: &Path, marks: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_armada"))
        .arg("session-keep")
        .arg(socket)
        .arg(socket.with_extension("spool"))
        .arg("600")
        .arg("--")
        .args(["/bin/sh", "-c", &format!("echo $$ >> '{}'; exec cat", marks.display())])
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the keeper starts")
}

fn exits_within(child: &mut Child, within: Duration) -> Option<std::process::ExitStatus> {
    let waited = std::time::Instant::now();
    while waited.elapsed() < within {
        if let Some(status) = child.try_wait().unwrap() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

fn running(pid: &str) -> bool {
    Command::new("ps").args(["-p", pid]).output().is_ok_and(|ps| ps.status.success())
}

#[test]
fn a_second_start_of_one_job_makes_no_second_agent_and_a_keeper_that_lost_its_socket_is_stopped() {
    let scratch = Scratch::new();
    let socket = scratch.0.join("job.sock");
    let marks = scratch.0.join("agents");
    let mut first = keeper_of(&socket, &marks);
    let waited = std::time::Instant::now();
    while std::os::unix::net::UnixStream::connect(&socket).is_err() || !marks.exists() {
        assert!(waited.elapsed() < Duration::from_secs(20), "the first keeper never answered");
        std::thread::sleep(Duration::from_millis(25));
    }

    let mut second = keeper_of(&socket, &marks);
    let refused = exits_within(&mut second, Duration::from_secs(10)).expect("the second keeper exits");
    assert!(!refused.success(), "the second keeper is refused");
    let agents = std::fs::read_to_string(&marks).unwrap();
    assert_eq!(agents.lines().count(), 1, "one agent: {agents:?}");
    let agent = agents.trim().to_string();

    // A keeper that answers is adopted, not stopped.
    let adopted = keeper::make_way(&socket, Duration::from_secs(10)).expect("way made");
    assert!(adopted.is_some(), "the running keeper is handed back");
    drop(adopted);
    assert!(exits_within(&mut first, Duration::from_millis(300)).is_none(), "adopting stops nothing");

    // Fleet takes the socket away when it ends a process; a keeper that never ends is then unreachable.
    std::fs::remove_file(&socket).unwrap();
    let made = keeper::make_way(&socket, Duration::from_secs(20)).expect("the old keeper stops");
    assert!(made.is_none(), "nothing is left to adopt");
    assert!(exits_within(&mut first, Duration::from_secs(10)).is_some(), "the old keeper is gone");
    assert!(!running(&agent), "its agent {agent} is gone too");
    assert_eq!(keeper::holder_of(&keeper::lock_of(&socket)), None);
}
