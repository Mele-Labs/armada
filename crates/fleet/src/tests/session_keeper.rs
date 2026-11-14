//! A Job's keeper, `crate::session_host::keeper`, run on the test's own runtime with a shell for an
//! agent. What is proved: a second keeper of the same Job starts no agent, and an agent whose output
//! the keeper cannot pass on is ended rather than left running unheard. Stopping a keeper that lost
//! its socket needs a process of its own: `crates/armada/tests/session_keep.rs`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::session_host::keeper::{self, Keeper};
use crate::tests::tmp::TempDir;

fn keeper_in(dir: &TempDir, script: &str, alone_for: Duration) -> Keeper {
    Keeper {
        socket: dir.path().join("k.sock"),
        spool: dir.path().join("k.spool"),
        log: dir.path().join("k.log"),
        program: String::from("/bin/sh"),
        args: vec![String::from("-c"), script.to_string()],
        directory: Some(dir.path().to_path_buf()),
        alone_for,
        ends_turn: Arc::new(|_| false),
        on_terminate: false,
    }
}

/// Whether the process at `pid` still runs.
fn running(pid: &str) -> bool {
    std::process::Command::new("ps").args(["-p", pid]).output().is_ok_and(|ps| ps.status.success())
}

async fn answering(socket: &Path) -> UnixStream {
    for _ in 0..200 {
        if let Ok(stream) = UnixStream::connect(socket) {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the keeper never answered at {}", socket.display());
}

// Two threads: the test reads the first keeper's socket blocking while that keeper runs on the runtime.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_keeper_of_the_same_job_starts_no_agent_and_leaves_the_first_reachable() {
    let dir = TempDir::new();
    let marks = dir.path().join("agents");
    let script = format!("echo started >> '{}'; exec cat", marks.display());
    let first = tokio::spawn(keeper::keep(keeper_in(&dir, &script, Duration::from_secs(600))));
    let mut stream = answering(&dir.path().join("k.sock")).await;
    for _ in 0..200 {
        if marks.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let second = tokio::time::timeout(Duration::from_secs(10), keeper::keep(keeper_in(&dir, &script, Duration::from_secs(600))))
        .await
        .expect("the second keeper returns rather than running beside the first");
    let refused = second.expect_err("the second keeper is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::AlreadyExists, "{refused}");
    assert_eq!(
        keeper::holder_of(&keeper::lock_of(&dir.path().join("k.sock"))),
        Some(std::process::id()),
        "the lock names the first keeper"
    );
    let started = std::fs::read_to_string(&marks).unwrap_or_default();
    assert_eq!(started.lines().count(), 1, "one agent started: {started:?}");

    // The first is still the Job's, at its socket.
    stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    stream.write_all(b"ATTACH\n").unwrap();
    let mut ready = String::new();
    BufReader::new(&stream).read_line(&mut ready).unwrap();
    assert!(ready.starts_with("READY "), "{ready:?}");
    stream.write_all(b"END\n").unwrap();
    tokio::time::timeout(Duration::from_secs(20), first).await.expect("the first keeper ends on END").unwrap().unwrap();
    assert_eq!(keeper::holder_of(&keeper::lock_of(&dir.path().join("k.sock"))), None, "an ended keeper lets the lock go");
}

#[tokio::test]
async fn an_agent_whose_output_the_keeper_cannot_read_is_ended_with_the_keeper() {
    let dir = TempDir::new();
    let pid_file = dir.path().join("agent.pid");
    // Not UTF-8, so the keeper cannot read the line; then the agent runs on.
    let script = format!("echo $$ > '{}'; printf '\\377\\n'; exec sleep 600", pid_file.display());
    let kept = keeper::keep(keeper_in(&dir, &script, Duration::from_millis(300)));

    tokio::time::timeout(Duration::from_secs(20), kept)
        .await
        .expect("the keeper ends, rather than holding an agent nobody hears")
        .unwrap();

    let pid = std::fs::read_to_string(&pid_file).expect("the agent started");
    assert!(!running(pid.trim()), "the agent {} was ended", pid.trim());
    let log = std::fs::read_to_string(dir.path().join("k.log")).unwrap_or_default();
    assert!(log.contains("could not pass on what the agent said"), "the log says why: {log:?}");
}
