//! A Job's terminal outliving Fleet: the keeper alone, over its socket; a Fleet's terminal run
//! through a keeper as it ran without one; and the next Fleet taking the terminal back under its id.
//! The keepers here are threads of the test (`Starting::InProcess`), which a Fleet that is dropped
//! leaves running as a keeper's process would; `crates/armada/tests/terminal_keep.rs` runs the real
//! binary.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::{Sessions, Watches};
use ipc::{ListTerminals, ResizeSessionRun, StartWatch, TypeInTerminal, WatchReading, WatchStarts, WatchState};
use testkit::FakeWorkProduct;

use crate::terminal_keeping::keeper::{self, ALONE_FOR};
use crate::terminal_keeping::{wire, Files, Keep, Record, Starting};
use crate::tests::daemon::a_fleet;
use crate::tests::session_host::eventually;
use crate::tests::session_running::{rig, Rig};
use crate::tests::session_terminal::{ends, shortly};
use crate::tests::tmp::TempDir;

/// A client of a keeper, speaking its frames.
struct Wire(UnixStream);

impl Wire {
    fn attach(socket: &Path, tail: u64) -> Wire {
        let mut stream = UnixStream::connect(socket).expect("a keeper answers");
        stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        wire::write_frame(&mut stream, wire::ATTACH, &tail.to_be_bytes()).unwrap();
        let mut wire = Wire(stream);
        let (kind, ready) = wire.next();
        assert_eq!(kind, wire::READY, "ready first");
        assert_eq!(ready.len(), 5);
        wire
    }

    fn next(&mut self) -> (u8, Vec<u8>) {
        wire::read_frame(&mut self.0).expect("a frame in time").expect("the keeper kept the connection")
    }

    fn send(&mut self, kind: u8, payload: &[u8]) {
        wire::write_frame(&mut self.0, kind, payload).unwrap();
    }

    fn types(&mut self, text: &str) {
        self.send(wire::INPUT, text.as_bytes());
    }

    /// Output until `needle` has been printed, all of it.
    fn until(&mut self, needle: &str) -> String {
        let mut printed = Vec::new();
        loop {
            let (kind, payload) = self.next();
            match kind {
                wire::OUTPUT => printed.extend(payload),
                wire::EXIT => panic!("the shell left before it printed {needle}: {:?}", String::from_utf8_lossy(&printed)),
                _ => {}
            }
            if String::from_utf8_lossy(&printed).contains(needle) {
                return String::from_utf8_lossy(&printed).into_owned();
            }
        }
    }

    /// Frames until the exit, and its code.
    fn until_exit(&mut self) -> Option<i32> {
        loop {
            let (kind, payload) = self.next();
            if kind == wire::EXIT {
                return match payload[..] {
                    [1, a, b, c, d] => Some(i32::from_be_bytes([a, b, c, d])),
                    _ => None,
                };
            }
        }
    }
}

fn keep_in(dir: &TempDir, alone_for: Duration) -> Keep {
    Keep {
        socket: dir.path().join("k.sock"),
        spool: dir.path().join("k.spool"),
        dir: dir.path().to_path_buf(),
        cols: 80,
        rows: 24,
        alone_for,
    }
}

/// Serve a keeper on a thread, and wait for the thread to finish once it has been told to end.
fn served(keep: &Keep) -> std::thread::JoinHandle<()> {
    let bound = keeper::bind(keep).expect("bound");
    std::thread::spawn(move || bound.serve())
}

fn finishes(serving: std::thread::JoinHandle<()>) {
    for _ in 0..300 {
        if serving.is_finished() {
            return serving.join().unwrap();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the keeper did not end");
}

#[test]
fn a_frame_reads_back_as_written_and_a_close_between_frames_is_none() {
    let mut bytes = wire::frame(wire::INPUT, b"ls\r");
    bytes.extend(wire::frame(wire::END, b""));
    let mut reading = bytes.as_slice();
    assert_eq!(wire::read_frame(&mut reading).unwrap(), Some((wire::INPUT, b"ls\r".to_vec())));
    assert_eq!(wire::read_frame(&mut reading).unwrap(), Some((wire::END, Vec::new())));
    assert_eq!(wire::read_frame(&mut reading).unwrap(), None);
}

#[test]
fn a_frame_cut_short_or_too_long_is_an_error() {
    let mut cut = wire::frame(wire::OUTPUT, b"hello");
    cut.truncate(7);
    assert!(wire::read_frame(&mut cut.as_slice()).is_err());
    let mut long = vec![wire::OUTPUT];
    long.extend_from_slice(&((wire::MOST as u32) + 1).to_be_bytes());
    assert!(wire::read_frame(&mut long.as_slice()).is_err());
}

#[test]
fn a_terminals_files_have_names_no_longer_than_a_session_keepers_and_its_record_reads_back() {
    let folder = Path::new("/keepers");
    let longest = "00000000-0000-0000-0000-000000000000";
    let files = Files::of(folder, longest, u32::MAX);
    let name = files.socket.file_name().unwrap().len();
    assert!(name <= format!("{longest}.sock").len(), "{} is {name} bytes", files.socket.display());
    assert_ne!(Files::of(folder, "a", 1).socket, Files::of(folder, "b", 1).socket);
    assert_ne!(Files::of(folder, "a", 1).socket, Files::of(folder, "a", 2).socket);

    let dir = TempDir::new();
    let record = Record {
        job: "a job".into(),
        number: 3,
        run_id: "session-run-X".into(),
        dir: "/a folder/with spaces".into(),
        socket: "/keepers/a.sock".into(),
        spool: "/keepers/a.spool".into(),
    };
    let at = dir.path().join("a.term");
    record.written(&at).unwrap();
    assert_eq!(Record::read(&at), Some(record));
    std::fs::write(&at, "job x\nterminal nonsense\n").unwrap();
    assert_eq!(Record::read(&at), None);
}

#[test]
fn a_terminal_keeper_relays_typed_input_spools_output_and_ends_with_its_shell_on_end() {
    let dir = TempDir::new();
    let keep = keep_in(&dir, ALONE_FOR);
    let serving = served(&keep);
    let mut client = Wire::attach(&keep.socket, 0);

    client.types("echo kept-$((40+2))\r");
    client.until("kept-42");
    let spooled = std::fs::read(&keep.spool).unwrap();
    assert!(String::from_utf8_lossy(&spooled).contains("kept-42"), "the spool holds what the shell printed");

    client.send(wire::END, &[]);
    client.until_exit();
    finishes(serving);
    assert!(!keep.socket.exists() && !keep.spool.exists(), "a keeper that ended takes its socket and spool with it");
}

#[test]
fn a_later_attach_is_replayed_the_tail_and_takes_the_connection_from_the_first() {
    let dir = TempDir::new();
    let keep = keep_in(&dir, ALONE_FOR);
    let serving = served(&keep);
    let mut first = Wire::attach(&keep.socket, 0);
    first.types("echo one-$((40+2))\r");
    first.until("one-42");

    let mut second = Wire::attach(&keep.socket, 1 << 20);
    let replayed = second.until("one-42");
    assert!(replayed.contains("one-42"));
    second.types("echo two-$((40+2))\r");
    second.until("two-42");

    second.send(wire::END, &[]);
    second.until_exit();
    finishes(serving);
}

#[test]
fn a_resize_reaches_the_shell_and_the_foreground_is_told_when_a_program_takes_it() {
    let dir = TempDir::new();
    let keep = keep_in(&dir, ALONE_FOR);
    let serving = served(&keep);
    let mut client = Wire::attach(&keep.socket, 0);

    let mut size = 100u16.to_be_bytes().to_vec();
    size.extend_from_slice(&30u16.to_be_bytes());
    client.send(wire::RESIZE, &size);
    client.types("stty size\r");
    client.until("30 100");

    client.types("sleep 30\r");
    loop {
        let (kind, payload) = client.next();
        if kind == wire::FOREGROUND && payload == [1] {
            break;
        }
    }
    client.types("\u{3}");
    loop {
        let (kind, payload) = client.next();
        if kind == wire::FOREGROUND && payload == [0] {
            break;
        }
    }
    client.send(wire::END, &[]);
    client.until_exit();
    finishes(serving);
}

#[test]
fn the_spool_is_cut_back_to_its_tail_and_a_late_attach_still_gets_the_end() {
    let dir = TempDir::new();
    let keep = keep_in(&dir, ALONE_FOR);
    let serving = served(&keep);
    let mut client = Wire::attach(&keep.socket, 0);
    client.types("head -c 3000000 /dev/zero | tr '\\000' x; printf '\\ndone-%d\\n' 2\r");
    client.until("done-2");

    assert!(std::fs::metadata(&keep.spool).unwrap().len() <= 1024 * 1024, "the spool never passes a megabyte");
    let mut late = Wire::attach(&keep.socket, 1 << 30);
    let replayed = late.until("done-2");
    assert!(replayed.len() <= 256 * 1024, "{} bytes replayed", replayed.len());
    late.send(wire::END, &[]);
    late.until_exit();
    finishes(serving);
}

#[test]
fn a_keeper_nobody_attaches_to_ends_itself_and_its_shell() {
    let dir = TempDir::new();
    let keep = keep_in(&dir, Duration::from_millis(500));
    finishes(served(&keep));
    assert!(!keep.socket.exists() && !keep.spool.exists());
}

fn keepers_of(rig: &Rig) -> std::path::PathBuf {
    std::path::PathBuf::from(&rig.fleet.host().keepers_dir)
}

/// A shell's pid, as it printed it for `echo pid-$$`: the digits after a `pid-`, wherever escapes put it.
fn pid_printed(history: &str) -> Option<String> {
    history.match_indices("pid-").find_map(|(at, word)| {
        let digits: String = history[at + word.len()..].chars().take_while(char::is_ascii_digit).collect();
        (!digits.is_empty()).then_some(digits)
    })
}

fn process_is_there(pid: &str) -> bool {
    std::process::Command::new("/bin/kill").args(["-0", pid]).stderr(std::process::Stdio::null()).status().is_ok_and(|status| status.success())
}

#[tokio::test]
async fn a_terminal_runs_through_its_keeper_as_before_and_stop_ends_the_keeper_and_the_shell() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    let files = Files::of(&keepers_of(&rig), rig.session.as_str(), 1);
    let record = Record::read(&files.record).expect("a record is written");
    assert_eq!((record.job.as_str(), record.number, record.run_id.as_str()), (rig.session.as_str(), 1, shell.as_str()));
    assert!(files.socket.exists() && files.spool.exists());

    rig.types(&shell, "echo pid-$$\r").await.expect("typed");
    assert!(shortly(|| async { pid_printed(&rig.observes(&shell).await.history).is_some() }).await, "the shell answered");
    let pid = pid_printed(&rig.observes(&shell).await.history).unwrap();
    let resized = rig.fleet.resize_session_run_terminal(ResizeSessionRun { run_id: shell.clone(), cols: 100, rows: 30 });
    assert!(resized.is_ok(), "a resize reaches the keeper");
    rig.types(&shell, "stty size\r").await.expect("typed");
    rig.printed(&shell, "30 100").await;
    assert!(process_is_there(&pid));

    ends(&rig, &shell).await;
    let end = rig.observes(&shell).await.live.expect_err("ended");
    assert!(end.stopped, "{end:?}");
    assert!(!files.record.exists() && !files.socket.exists() && !files.spool.exists(), "stop takes the record and the files");
    assert!(shortly(|| async { !process_is_there(&pid) }).await, "the shell is gone with its keeper");
}

#[tokio::test]
async fn a_terminal_whose_fleet_died_comes_back_at_boot_under_its_id_with_its_screen_and_a_watch_hears_it() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    rig.types(&shell, "KEPT=alive-$((40+2)); echo screen-$((40+2))\r").await.expect("typed");
    rig.printed(&shell, "screen-42").await;
    let Rig { fleet: old, session, dir, home } = rig;

    old.session_runs().connections_dropped();
    eventually(|| async { old.observe_session_run(shell.clone()).await.expect("held").live.is_err() }).await;
    let files = Files::of(Path::new(&old.host().keepers_dir), session.as_str(), 1);
    assert!(files.record.exists() && files.socket.exists(), "the keeper and its record outlive the Fleet that held them");
    drop(old);

    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&[])));
    assert_eq!(fleet.list_terminals(None, ListTerminals { session_id: Some(session.clone()) }).await.unwrap().terminals.len(), 0);
    assert_eq!(fleet.reattached_terminals().await, 1);
    let rig = Rig { fleet, session, dir, home };

    let listed = rig.fleet.list_terminals(None, ListTerminals { session_id: Some(rig.session.clone()) }).await.unwrap().terminals;
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].terminal.as_str(), listed[0].name.as_str(), listed[0].run_id.as_str()), ("t1", "Terminal 1", shell.as_str()));
    // The replay reaches the history on the run's output thread, after the reattach has returned.
    rig.printed(&shell, "screen-42").await;

    let watch = StartWatch {
        session_id: Some(rig.session.clone()),
        name: String::from("Server up"),
        starts: WatchStarts::Terminal { text: String::from(r"WATCHED-\d+"), regex: true, terminal: None, gives_up_after: Some(String::from("10m")) },
        follows: None,
        set_aside: Vec::new(),
    };
    rig.fleet.start_watch(None, watch).await.expect("a terminal is open to watch");
    rig.fleet.watches_run_when_due().await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let typed = TypeInTerminal {
        text: "echo $KEPT WATCHED-$((40+2))".into(),
        enter: true,
        clear: true,
        wait_seconds: Some(5),
        session_id: Some(rig.session.clone()),
        terminal: Some("t1".into()),
        new: false,
    };
    let typed = Arc::clone(&rig.fleet).type_in_terminal(None, typed).await.expect("the agent's last-seen shell is the one it types into");
    assert_eq!(typed.run_id, shell);
    assert!(typed.printed.lines().any(|line| line.trim() == "alive-42 WATCHED-42"), "the same shell, with its variable: {:?}", typed.printed);

    let mut read = None;
    for _ in 0..100 {
        let seen = rig.fleet.list_watches(None, Some(rig.session.clone()), true).await.unwrap().watches.remove(0);
        if seen.last.is_some() {
            read = Some(seen);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let read = read.expect("the watch read the terminal");
    assert_eq!((read.state, read.live), (WatchState::Passed, false));
    assert!(matches!(&read.last, Some(WatchReading::Read { says, .. }) if says.contains("WATCHED-42")), "{:?}", read.last);

    ends(&rig, &shell).await;
    assert!(!files.record.exists() && !files.socket.exists(), "the stop that follows takes the record");
}

#[tokio::test]
async fn a_record_whose_keeper_is_gone_or_which_cannot_be_read_is_removed() {
    let rig = rig().await;
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    ends(&rig, &shell).await;

    let folder = keepers_of(&rig);
    let files = Files::of(&folder, rig.session.as_str(), 1);
    let gone = Record {
        job: rig.session.as_str().to_string(),
        number: 1,
        run_id: "session-run-gone".into(),
        dir: "/".into(),
        socket: folder.join("gone.sock"),
        spool: folder.join("gone.spool"),
    };
    gone.written(&files.record).unwrap();
    let garbled = folder.join("garbled.term");
    std::fs::write(&garbled, "this is not a record").unwrap();

    assert_eq!(rig.fleet.reattached_terminals().await, 0);
    assert!(!files.record.exists() && !garbled.exists());
    let listed = rig.fleet.list_terminals(None, ListTerminals { session_id: Some(rig.session.clone()) }).await.unwrap().terminals;
    assert!(listed.is_empty());
}

#[tokio::test]
async fn a_record_whose_job_is_gone_is_removed_and_its_keeper_ended() {
    let rig = rig().await;
    let folder = keepers_of(&rig);
    let files = Files::of(&folder, "nobody", 1);
    let keep = Keep { socket: files.socket.clone(), spool: files.spool.clone(), dir: rig.dir.path().to_path_buf(), cols: 80, rows: 24, alone_for: ALONE_FOR };
    let _stream = Starting::InProcess.start(&keep, &files.log()).expect("a keeper");
    let record = Record { job: "nobody".into(), number: 1, run_id: "session-run-x".into(), dir: "/".into(), socket: files.socket.clone(), spool: files.spool.clone() };
    record.written(&files.record).unwrap();
    let mut watching = Wire::attach(&files.socket, 0);

    assert_eq!(rig.fleet.reattached_terminals().await, 0);
    assert!(!files.record.exists());
    // Told to end, its shell leaves and says so to whoever is attached.
    tokio::task::spawn_blocking(move || watching.until_exit()).await.unwrap();
}

#[test]
fn a_written_input_is_chunked_under_the_frame_limit() {
    let mut out = Vec::new();
    for frame in wire::frames(wire::OUTPUT, &vec![b'x'; wire::MOST * 2 + 1]) {
        out.write_all(&frame).unwrap();
    }
    let mut reading = out.as_slice();
    let mut total = 0;
    while let Some((_, payload)) = wire::read_frame(&mut reading).unwrap() {
        assert!(payload.len() <= wire::MOST);
        total += payload.len();
    }
    assert_eq!(total, wire::MOST * 2 + 1);
}
