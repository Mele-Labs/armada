//! `armada terminal-keep`, the binary, started as Fleet starts it: detached, without Fleet's mark,
//! with its output in a log. What is proved is what the keeper's own tests cannot: that the verb is
//! wired, that a keeper in a process of its own binds its socket, runs the shell and relays a typed
//! line, and that it and its shell go when told to end.
//!
//! | Proved | Not proved |
//! |---|---|
//! | `Starting::Binary` starts the verb and answers a connection once the socket is up | That a Fleet restart finds it again. `fleet::tests::session_terminal_kept` |
//! | The shell runs in the folder given and prints what is typed into it | The frames' every corner. The same file |
//! | `END` ends the keeper, which takes its socket with it | |

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use fleet::terminal_keeping::{Keep, Starting};

/// A folder only this user may open, as a keeper's folder under /tmp must be.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Scratch {
        let path = std::env::temp_dir().join(format!("armada-tk-{}", std::process::id()));
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

fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut whole = vec![kind];
    whole.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    whole.extend_from_slice(payload);
    whole
}

fn next(stream: &mut UnixStream) -> (u8, Vec<u8>) {
    let mut head = [0u8; 5];
    stream.read_exact(&mut head).expect("a frame in time");
    let mut payload = vec![0u8; u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize];
    stream.read_exact(&mut payload).expect("its payload");
    (head[0], payload)
}

// A runtime, since Fleet spawns a detached child through tokio's own process type.
#[tokio::test]
async fn the_verb_holds_a_shell_that_relays_a_typed_line_and_ends_on_end() {
    use std::io::Write;
    let scratch = Scratch::new();
    let folders = scratch.0.join("sessions");
    let shell_in = scratch.0.join("work");
    std::fs::create_dir_all(&shell_in).unwrap();
    let keep = Keep {
        socket: folders.join("k.sock"),
        spool: folders.join("k.spool"),
        dir: shell_in.clone(),
        cols: 80,
        rows: 24,
        alone_for: Duration::from_secs(600),
    };
    let starting = Starting::Binary { exe: PathBuf::from(env!("CARGO_BIN_EXE_armada")), home: scratch.0.clone() };
    let mut stream = starting.start(&keep, &folders.join("k.log")).expect("a keeper in a process of its own");
    stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();

    stream.write_all(&frame(b'A', &0u64.to_be_bytes())).unwrap();
    assert_eq!(next(&mut stream).0, b'Y', "ready first");
    stream.write_all(&frame(b'I', b"echo bin-$((40+2)); pwd\r")).unwrap();
    let mut printed = String::new();
    while !printed.contains("bin-42") || !printed.contains("work") {
        let (kind, payload) = next(&mut stream);
        assert_ne!(kind, b'X', "the shell left early: {printed:?}");
        if kind == b'O' {
            printed.push_str(&String::from_utf8_lossy(&payload));
        }
    }

    stream.write_all(&frame(b'E', &[])).unwrap();
    while next(&mut stream).0 != b'X' {}
    for _ in 0..300 {
        if !keep.socket.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the keeper did not take its socket away");
}
