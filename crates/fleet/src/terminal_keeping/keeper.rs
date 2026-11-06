//! The process that holds a Job's shell so Fleet can restart under it. `armada terminal-keep`,
//! started by Fleet and by nothing else.
//!
//! It opens the pseudo-terminal, starts `$SHELL -l` on it, appends every byte the shell prints to
//! the spool and serves the socket. **One client at a time, the newest**: an attach is sent the
//! tail of the spool and then what comes after, under the lock output is appended under, so it
//! loses none and reads none twice. **The spool is cut back to its tail** once it passes
//! [`SPOOL_MOST`], since only a tail is ever replayed.
//!
//! It ends the shell's group on `END` and after [`ALONE_FOR`] with no Fleet attached, with a
//! hangup first, which a shell leaves on, and a kill after [`GRACE`]. It ends itself once the
//! shell is gone and its exit has been taken, or there is no one left to take it.

use std::io::{self, Read, Seek, Write};
use std::num::NonZeroU32;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use super::{wire, Keep};

pub use crate::session_host::keeper::ALONE_FOR;

/// The spool's size at which it is cut back to the [`super::TAIL`].
const SPOOL_MOST: u64 = 1024 * 1024;
/// How long a stopped shell's group has to leave before it is killed.
const GRACE: Duration = Duration::from_secs(3);
/// How long output still in the terminal is read for after the shell has exited.
const DRAIN: Duration = Duration::from_millis(300);
/// How often the keeper looks at its listener and the clock.
const TICK: Duration = Duration::from_millis(20);
/// How often it looks at the foreground.
const FOREGROUND_EVERY: Duration = Duration::from_millis(100);

/// What the output lock guards.
struct Out {
    spool: std::fs::File,
    /// The spool's inode, which a cut changes: the paths are removed only while they are still ours.
    spool_inode: Option<u64>,
    size: u64,
    /// Where the newest client's frames go.
    sink: Option<mpsc::Sender<Vec<u8>>>,
    foreground: bool,
    /// `Some` once the shell is gone and its output read: its code.
    exit: Option<Option<i32>>,
}

impl Out {
    /// Hand `frames` to the client, if there is one; one that cannot take them is let go.
    fn send(&mut self, frames: impl Iterator<Item = Vec<u8>>) {
        let Some(sink) = &self.sink else {
            return;
        };
        for frame in frames {
            if sink.send(frame).is_err() {
                self.sink = None;
                return;
            }
        }
    }
}

struct Shared {
    out: Mutex<Out>,
    spool: PathBuf,
    pid: u32,
    keys: mpsc::Sender<Vec<u8>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    /// Fleet ordered the end.
    ended: AtomicBool,
    clients: AtomicUsize,
    alone_since: Mutex<Instant>,
    /// The exit was written to a client, so no one is owed it.
    told: AtomicBool,
}

impl Shared {
    fn print(&self, bytes: &[u8]) {
        let mut out = self.out.lock().expect("not poisoned");
        if out.spool.write_all(bytes).is_ok() {
            out.size += bytes.len() as u64;
        }
        if out.size > SPOOL_MOST {
            self.cut(&mut out);
        }
        out.send(wire::frames(wire::OUTPUT, bytes));
    }

    /// Keep the spool's tail and drop the rest, unless the path is no longer this keeper's.
    fn cut(&self, out: &mut Out) {
        if inode(&self.spool) != out.spool_inode {
            return;
        }
        let Ok(tail) = tail_of(&self.spool, super::TAIL) else {
            return;
        };
        let next = self.spool.with_extension("cut");
        if std::fs::write(&next, &tail).and_then(|()| std::fs::rename(&next, &self.spool)).is_err() {
            return;
        }
        if let Ok(file) = std::fs::OpenOptions::new().append(true).open(&self.spool) {
            out.spool = file;
            out.size = tail.len() as u64;
            out.spool_inode = inode(&self.spool);
        }
    }

    fn exited(&self, code: Option<i32>) {
        let mut out = self.out.lock().expect("not poisoned");
        out.exit = Some(code);
        out.send(std::iter::once(exit_frame(code)));
    }

    /// A client's frames, starting with the tail it asked for, ready first. Replaces the one before.
    fn attach(&self, tail: u64) -> mpsc::Receiver<Vec<u8>> {
        let (sink, frames) = mpsc::channel();
        let mut out = self.out.lock().expect("not poisoned");
        out.foreground = self.foreground_taken();
        let mut ready = self.pid.to_be_bytes().to_vec();
        ready.push(u8::from(out.foreground));
        let _ = sink.send(wire::frame(wire::READY, &ready));
        if let Ok(replay) = tail_of(&self.spool, tail.min(super::TAIL)) {
            for frame in wire::frames(wire::OUTPUT, &replay) {
                let _ = sink.send(frame);
            }
        }
        if let Some(code) = out.exit {
            let _ = sink.send(exit_frame(code));
        }
        out.sink = Some(sink);
        frames
    }

    fn foreground_taken(&self) -> bool {
        let master = self.master.lock().expect("not poisoned");
        master.process_group_leader().is_some_and(|leader| leader != self.pid as i32)
    }

    /// Tell the client when the foreground changes.
    fn foreground_watched(&self) {
        let now = self.foreground_taken();
        let mut out = self.out.lock().expect("not poisoned");
        if out.foreground != now {
            out.foreground = now;
            out.send(std::iter::once(wire::frame(wire::FOREGROUND, &[u8::from(now)])));
        }
    }
}

fn exit_frame(code: Option<i32>) -> Vec<u8> {
    match code {
        Some(code) => {
            let mut payload = vec![1];
            payload.extend_from_slice(&code.to_be_bytes());
            wire::frame(wire::EXIT, &payload)
        }
        None => wire::frame(wire::EXIT, &[0]),
    }
}

/// The last `most` bytes of the spool, starting at a character where it was cut mid-way.
fn tail_of(spool: &Path, most: u64) -> io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(spool)?;
    let length = file.metadata()?.len();
    let from = length.saturating_sub(most);
    file.seek(io::SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if from > 0 {
        let continuing = bytes.iter().take(3).take_while(|byte| *byte & 0xC0 == 0x80).count();
        bytes.drain(..continuing);
    }
    Ok(bytes)
}

fn inode(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.ino())
}

/// A keeper with its shell started and its socket bound, not yet serving.
pub struct Bound {
    shared: Arc<Shared>,
    listener: UnixListener,
    socket: PathBuf,
    mine: Option<u64>,
    alone_for: Duration,
}

/// Start the shell and bind the socket. **The socket appears last**, bound beside its path and
/// moved onto it once the shell is running, so a Fleet that finds it finds a keeper with a shell.
pub fn bind(keep: &Keep) -> io::Result<Bound> {
    let binding = keep.socket.with_extension("bind");
    let _ = std::fs::remove_file(&binding);
    let listener = UnixListener::bind(&binding)?;
    let bound = bound_beside(keep, listener, &binding);
    if bound.is_err() {
        let _ = std::fs::remove_file(&binding);
    }
    bound
}

fn bound_beside(keep: &Keep, listener: UnixListener, binding: &Path) -> io::Result<Bound> {
    std::fs::set_permissions(binding, std::fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    // A new file, not the old one truncated: a keeper that is ending may still hold the old one.
    let _ = std::fs::remove_file(&keep.spool);
    let spool = std::fs::OpenOptions::new().create(true).append(true).open(&keep.spool)?;

    let pair = native_pty_system().openpty(PtySize { rows: keep.rows, cols: keep.cols, pixel_width: 0, pixel_height: 0 }).map_err(io::Error::other)?;
    let mut shell = CommandBuilder::new(crate::session_running::shell());
    shell.arg("-l");
    shell.cwd(&keep.dir);
    shell.env("TERM", "xterm-256color");
    let child = pair.slave.spawn_command(shell).map_err(io::Error::other)?;
    // With our slave end open the reader would never be told the terminal had closed.
    drop(pair.slave);
    let pid = child.process_id().ok_or_else(|| io::Error::other("the shell has no process id"))?;
    let reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
    let writer = pair.master.take_writer().map_err(io::Error::other)?;

    let _ = std::fs::remove_file(&keep.socket);
    std::fs::rename(binding, &keep.socket)?;

    let (keys, typed) = mpsc::channel::<Vec<u8>>();
    let shared = Arc::new(Shared {
        out: Mutex::new(Out { spool_inode: inode(&keep.spool), spool, size: 0, sink: None, foreground: false, exit: None }),
        spool: keep.spool.clone(),
        pid,
        keys,
        master: Mutex::new(pair.master),
        ended: AtomicBool::new(false),
        clients: AtomicUsize::new(0),
        alone_since: Mutex::new(Instant::now()),
        told: AtomicBool::new(false),
    });
    typing(writer, typed);
    let (drained, drain) = mpsc::channel::<()>();
    printing(Arc::clone(&shared), reader, drained);
    waiting(Arc::clone(&shared), child, drain);
    Ok(Bound { shared, listener, socket: keep.socket.clone(), mine: inode(&keep.socket), alone_for: keep.alone_for })
}

/// Write the keys to the terminal, on a thread of their own: a program that is not reading can hold
/// a write back, and must not hold back the next frame, an end among them.
fn typing(mut writer: Box<dyn Write + Send>, typed: mpsc::Receiver<Vec<u8>>) {
    std::thread::spawn(move || {
        for keys in typed {
            if writer.write_all(&keys).and_then(|()| writer.flush()).is_err() {
                break;
            }
        }
    });
}

/// Read the terminal until it closes, spooling and passing on what it prints. A blocking read cannot
/// be cancelled: a grandchild holding the terminal open keeps this thread reading until it goes too.
fn printing(shared: Arc<Shared>, mut reader: Box<dyn Read + Send>, drained: mpsc::Sender<()>) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => shared.print(&buffer[..read]),
                Err(why) if why.kind() == io::ErrorKind::Interrupted => {}
                // Some systems answer a read on a terminal whose far end closed with an error.
                Err(_) => break,
            }
        }
        let _ = drained.send(());
    });
}

/// Wait for the shell, then for its last output no longer than [`DRAIN`], then record its exit.
fn waiting(shared: Arc<Shared>, mut child: Box<dyn Child + Send + Sync>, drain: mpsc::Receiver<()>) {
    std::thread::spawn(move || {
        let status = child.wait();
        let _ = drain.recv_timeout(DRAIN);
        shared.exited(match status {
            Ok(status) if status.signal().is_none() => Some(status.exit_code() as i32),
            _ => None,
        });
    });
}

/// Write one client's frames to its socket, and note when its exit was taken.
fn writing(shared: Arc<Shared>, frames: mpsc::Receiver<Vec<u8>>, mut stream: UnixStream) {
    std::thread::spawn(move || {
        for frame in frames {
            if stream.write_all(&frame).and_then(|()| stream.flush()).is_err() {
                break;
            }
            if frame.first() == Some(&wire::EXIT) {
                shared.told.store(true, Ordering::SeqCst);
            }
        }
    });
}

/// Read one client's frames until it closes.
fn client(shared: Arc<Shared>, mut stream: UnixStream) {
    shared.clients.fetch_add(1, Ordering::SeqCst);
    while let Ok(Some((kind, payload))) = wire::read_frame(&mut stream) {
        match kind {
            wire::ATTACH => {
                let tail = payload.get(..8).and_then(|bytes| bytes.try_into().ok()).map_or(0, u64::from_be_bytes);
                if let Ok(write) = stream.try_clone() {
                    writing(Arc::clone(&shared), shared.attach(tail), write);
                }
            }
            wire::INPUT => {
                let _ = shared.keys.send(payload);
            }
            wire::RESIZE => {
                if let [a, b, c, d, ..] = payload[..] {
                    let size = PtySize { cols: u16::from_be_bytes([a, b]), rows: u16::from_be_bytes([c, d]), pixel_width: 0, pixel_height: 0 };
                    let _ = shared.master.lock().expect("not poisoned").resize(size);
                }
            }
            wire::END => shared.ended.store(true, Ordering::SeqCst),
            _ => {}
        }
    }
    if shared.clients.fetch_sub(1, Ordering::SeqCst) == 1 {
        *shared.alone_since.lock().expect("not poisoned") = Instant::now();
    }
}

/// `SIGHUP` to the shell's group, with `kill`, which takes `-<pgid>` for a whole group.
fn hang_up(group: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-s", "HUP", "--", &format!("-{group}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

impl Bound {
    /// Serve until the shell is gone and its exit has been taken, or no Fleet is left to take it.
    pub fn serve(self) {
        let Bound { shared, listener, socket, mine, alone_for } = self;
        let mut ending: Option<Instant> = None;
        let mut killed = false;
        let mut looked = Instant::now();
        loop {
            if let Ok((stream, _)) = listener.accept() {
                // A socket accepted from a listener that does not block may not block either.
                let _ = stream.set_nonblocking(false);
                let serving = Arc::clone(&shared);
                std::thread::spawn(move || client(serving, stream));
            }
            let exited = shared.out.lock().expect("not poisoned").exit.is_some();
            let alone = shared.clients.load(Ordering::SeqCst) == 0
                && shared.alone_since.lock().expect("not poisoned").elapsed() >= alone_for;
            let ended = shared.ended.load(Ordering::SeqCst);
            if exited && (shared.told.load(Ordering::SeqCst) || alone || ended) {
                break;
            }
            if (ended || alone) && !exited {
                match ending {
                    None => {
                        ending = Some(Instant::now());
                        hang_up(shared.pid);
                    }
                    Some(at) if !killed && at.elapsed() >= GRACE => {
                        killed = true;
                        // The shell is uncollected until its wait returns, so the group is still its.
                        if let Some(group) = NonZeroU32::new(shared.pid) {
                            crate::group::end_the_group(group);
                        }
                    }
                    Some(_) => {}
                }
            }
            if looked.elapsed() >= FOREGROUND_EVERY {
                looked = Instant::now();
                shared.foreground_watched();
            }
            std::thread::sleep(TICK);
        }
        // Only what is still this keeper's: a replacement may have bound the same paths.
        if inode(&socket) == mine {
            let _ = std::fs::remove_file(&socket);
        }
        let spool = shared.out.lock().expect("not poisoned").spool_inode;
        if inode(&shared.spool) == spool {
            let _ = std::fs::remove_file(&shared.spool);
        }
    }
}

/// `armada terminal-keep <socket> <spool> <alone-secs> <cols> <rows> <dir>`: the shell runs in `dir`
/// with the keeper's own environment.
pub fn run(args: Vec<String>) -> Result<(), String> {
    let [socket, spool, alone, cols, rows, dir] = args.as_slice() else {
        return Err(String::from("terminal-keep: expected <socket> <spool> <alone-secs> <cols> <rows> <dir>"));
    };
    let number = |what: &str, text: &str| text.parse::<u64>().map_err(|why| format!("terminal-keep: {what}: {why}"));
    let cell = |what: &str, text: &str| u16::try_from(number(what, text)?).map_err(|why| format!("terminal-keep: {what}: {why}"));
    let keep = Keep {
        socket: PathBuf::from(socket),
        spool: PathBuf::from(spool),
        dir: PathBuf::from(dir),
        cols: cell("cols", cols)?,
        rows: cell("rows", rows)?,
        alone_for: Duration::from_secs(number("alone-secs", alone)?),
    };
    bind(&keep).map_err(|why| format!("terminal-keep: {why}"))?.serve();
    Ok(())
}
