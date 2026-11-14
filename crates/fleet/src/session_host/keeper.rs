//! The process that holds a hosted session's agent so Fleet can restart under
//! it. `armada session-keep`, started by Fleet and by nothing else.
//!
//! The agent's stdin and stdout are the keeper's pipes, not Fleet's: it appends
//! every line the agent writes to a spool file and serves a unix socket Fleet
//! attaches to, so the agent and its background subagents outlive a Fleet
//! restart. `docs/concepts/session/a-job-fleet-hosts.md`, *What a restart does
//! to a job*, has the wire and the rules for when a keeper ends itself.
//!
//! One client at a time, the newest. The keeper holds the acknowledged offset,
//! so a Fleet that died with lines untaken is replayed exactly those.
//!
//! **One keeper per Job**, by an `flock` on `<id>.lock` the keeper holds for its
//! life: a second keeper of the same Job ends before it starts an agent. Two
//! copies of one Job ran side by side for a day when a keeper whose socket
//! Fleet had removed was not found (14 Nov 2026).

use std::io::{self, Read, Write};
use std::num::NonZeroU32;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, DroneEvent};
use adapters::HeadlessAgent;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, watch, Notify};

use crate::detach::Detached;

/// How long a keeper runs an agent no Fleet is attached to.
pub const ALONE_FOR: Duration = Duration::from_secs(30 * 60);

/// What a keeper is started with.
pub struct Keeper {
    pub socket: PathBuf,
    pub spool: PathBuf,
    /// Where the agent's stderr and how it ended are written. Fleet reads the
    /// tail when the agent is gone, so a death says why.
    pub log: PathBuf,
    pub program: String,
    pub args: Vec<String>,
    /// Where the agent runs. `None` is the keeper's own, which Fleet set.
    pub directory: Option<PathBuf>,
    pub alone_for: Duration,
    /// Whether an agent line ends a turn, so a Fleet that attaches mid-turn is
    /// told so. Supplied, because this file parses no agent line.
    pub ends_turn: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    /// Whether `SIGTERM` ends the agent and then the keeper: the process's own
    /// keeper only, since a test's keeper shares the test's process.
    pub on_terminate: bool,
}

/// The file a Job's keeper holds locked for its life, beside its socket.
pub fn lock_of(socket: &Path) -> PathBuf {
    socket.with_extension("lock")
}

/// The pid of the keeper holding `lock`, or `None` where no keeper holds it.
/// **The lock is the proof the keeper lives**: the kernel lets it go however
/// the keeper ends, so a pid read from a lock found held is still the keeper's.
pub fn holder_of(lock: &Path) -> Option<u32> {
    let mut file = std::fs::OpenOptions::new().read(true).open(lock).ok()?;
    match file.try_lock() {
        // Free: closing the file lets it go again.
        Ok(()) => None,
        Err(std::fs::TryLockError::WouldBlock) => {
            let mut said = String::new();
            let _ = file.read_to_string(&mut said);
            // Held by a keeper that has not written its pid yet: alive, and unnamed.
            Some(said.trim().parse().unwrap_or(0))
        }
        Err(std::fs::TryLockError::Error(_)) => None,
    }
}

/// What Fleet does before it starts a Job's keeper: **find any keeper the Job
/// already has, and adopt it or stop it.** One whose socket answers is handed
/// back to be attached to. One that holds the lock and does not answer has
/// lost its socket (Fleet takes it away when it ends a process) and is asked to
/// stop with `SIGTERM`, then waited for. One that will not stop within `within`
/// is an error, and nothing is started beside it.
pub fn make_way(socket: &Path, within: Duration) -> Result<Option<std::os::unix::net::UnixStream>, String> {
    let lock = lock_of(socket);
    let Some(pid) = holder_of(&lock) else {
        return Ok(None);
    };
    if let Some(stream) = super::kept::connect(socket) {
        return Ok(Some(stream));
    }
    // The keeper leads its own group (`crate::detach`), and its agent leads
    // another, which the keeper ends itself on the signal.
    if let Some(keeper) = NonZeroU32::new(pid) {
        crate::group::ask_the_group_to_end(keeper);
    }
    let asked = Instant::now();
    while holder_of(&lock).is_some() {
        if asked.elapsed() > within {
            return Err(format!(
                "the job's last agent (keeper pid {pid}) has not stopped, so a second one was not started"
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(None)
}

/// Take `lock` for this keeper's life and write its pid in it, or refuse
/// where another keeper of the Job holds it. Opened close-on-exec, as the
/// standard library opens every file, so the agent does not inherit it and
/// hold it after the keeper is gone.
fn taken(lock: &Path) -> io::Result<std::fs::File> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            let mut said = String::new();
            let _ = file.read_to_string(&mut said);
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("a keeper of this job already runs (pid {}), so no second agent was started", said.trim()),
            ));
        }
        Err(std::fs::TryLockError::Error(why)) => return Err(why),
    }
    file.set_len(0)?;
    write!(file, "{}", std::process::id())?;
    file.flush()?;
    Ok(file)
}

#[derive(Clone, Copy)]
struct Progress {
    written: u64,
    over: bool,
}

struct Shared {
    spool: PathBuf,
    progress: watch::Sender<Progress>,
    acked: AtomicU64,
    busy: AtomicBool,
    /// The newest attach's number; a pump holding an older one is superseded.
    attaches: AtomicU64,
    latest: watch::Sender<u64>,
    clients: AtomicUsize,
    alone_since: Mutex<Instant>,
    told_gone: AtomicBool,
    pid: u32,
    stdin: mpsc::UnboundedSender<String>,
    end: Notify,
    /// Fleet ordered the end, and has taken the socket and spool away itself:
    /// the next keeper of this session may already be using those paths.
    ended: AtomicBool,
}

/// Run until the agent is gone and its last word has been taken.
pub async fn keep(keeper: Keeper) -> io::Result<()> {
    // First, before the socket is touched: a second keeper must not take the
    // path of a first that is still running its agent.
    let _held = taken(&lock_of(&keeper.socket))?;
    let _ = std::fs::remove_file(&keeper.socket);
    // Bound beside the path and moved onto it once listening: a Fleet that
    // connects between the bind and the listen is refused, takes the socket
    // for a dead keeper's and removes it, and this keeper is never reached.
    let binding = keeper.socket.with_extension("bind");
    let _ = std::fs::remove_file(&binding);
    let listener = UnixListener::bind(&binding)?;
    std::fs::set_permissions(&binding, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&binding, &keeper.socket)?;
    // A new file, not the old one truncated: a keeper that is ending may still
    // hold the old one and remove its path.
    let _ = std::fs::remove_file(&keeper.spool);
    let mut spool = tokio::fs::File::create(&keeper.spool).await?;
    let mine = (inode(&keeper.socket), inode(&keeper.spool));

    let mut child = Detached::agent(&keeper.program, &keeper.args, keeper.directory.as_deref())
        .piping_input()
        .capturing_output()
        .spawn()?;
    let pid = child.id().unwrap_or(0);
    let (Some(mut stdin), Some(stdout), Some(mut stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        return Err(io::Error::other("the agent's pipes were lost"));
    };

    let (to_stdin, mut lines) = mpsc::unbounded_channel::<String>();
    let shared = Arc::new(Shared {
        spool: keeper.spool.clone(),
        progress: watch::channel(Progress {
            written: 0,
            over: false,
        })
        .0,
        acked: AtomicU64::new(0),
        busy: AtomicBool::new(false),
        attaches: AtomicU64::new(0),
        latest: watch::channel(0).0,
        clients: AtomicUsize::new(0),
        alone_since: Mutex::new(Instant::now()),
        told_gone: AtomicBool::new(false),
        pid,
        stdin: to_stdin,
        end: Notify::new(),
        ended: AtomicBool::new(false),
    });

    tokio::spawn(async move {
        while let Some(line) = lines.recv().await {
            let line = line + "\n";
            if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                break;
            }
        }
    });
    let log = keeper.log.clone();
    let complaints = tokio::spawn(async move {
        // Appended, as the keeper's own stderr is: both go to the one file.
        match tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log)
            .await
        {
            Ok(mut file) => {
                let _ = tokio::io::copy(&mut stderr, &mut file).await;
            }
            Err(_) => {
                let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
            }
        }
    });
    let log = keeper.log.clone();
    let speaking = Arc::clone(&shared);
    let ends_turn = Arc::clone(&keeper.ends_turn);
    tokio::spawn(async move {
        let mut reading = BufReader::new(stdout).lines();
        let mut written = 0u64;
        // **An agent nobody can hear is ended**, not left running: a keeper that
        // stopped reading on a full disk went on to ignore `END` and its own
        // timer, and its agent ran unseen beside the next one (14 Nov 2026).
        let unheard = |why: String| {
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(&log) {
                let _ = writeln!(file, "the keeper could not pass on what the agent said ({why}), so it ended the agent");
            }
            if let Some(group) = NonZeroU32::new(pid) {
                crate::group::end_the_group(group);
            }
        };
        loop {
            tokio::select! {
                line = reading.next_line() => match line {
                    Ok(Some(line)) => {
                        let whole = line + "\n";
                        // `write_all` returns once the bytes are queued: the
                        // write itself finishes on another thread. The flush
                        // waits for it, so a pump told of this line finds it
                        // in the spool and not a short read it then waits on.
                        let spooled = match spool.write_all(whole.as_bytes()).await {
                            Ok(()) => spool.flush().await,
                            failed => failed,
                        };
                        if let Err(why) = spooled {
                            unheard(format!("the spool: {why}"));
                            break;
                        }
                        written += whole.len() as u64;
                        // Only a turn's end clears it: a line after the result
                        // (a rate-limit notice, a hook) is not a new turn, and
                        // setting busy on it left a finished agent Working.
                        if ends_turn(&whole) {
                            speaking.busy.store(false, Ordering::SeqCst);
                        }
                        speaking.progress.send_modify(|p| p.written = written);
                    }
                    Ok(None) => break,
                    Err(why) => {
                        unheard(format!("its output: {why}"));
                        break;
                    }
                },
                () = speaking.end.notified() => {
                    // The child is uncollected until the wait below, so the
                    // group is still this agent's.
                    if let Some(group) = NonZeroU32::new(pid) {
                        crate::group::end_the_group(group);
                    }
                    break;
                }
            }
        }
        // An agent that closed its output may still be running: the end is
        // still heard while it is waited on.
        let status = tokio::select! {
            status = child.wait() => status,
            () = speaking.end.notified() => {
                if let Some(group) = NonZeroU32::new(pid) {
                    crate::group::end_the_group(group);
                }
                child.wait().await
            }
        };
        if let Ok(status) = status {
            // Its last words are in the log before anyone is told it is over.
            let _ = complaints.await;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&log)
            {
                let _ = writeln!(file, "the agent exited: {status}");
            }
        }
        speaking.progress.send_modify(|p| p.over = true);
    });

    // A Fleet that finds this keeper locked but unreachable asks it to stop this way.
    let mut terminated = match keeper.on_terminate {
        true => Some(tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?),
        false => None,
    };
    let mut stopped = false;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                if let Ok((stream, _)) = accepted {
                    tokio::spawn(client(Arc::clone(&shared), stream));
                }
            }
            Some(()) = async { terminated.as_mut()?.recv().await } => {
                stopped = true;
                shared.end.notify_one();
            }
            _ = tick.tick() => {
                let over = shared.progress.borrow().over;
                let alone = shared.clients.load(Ordering::SeqCst) == 0
                    && shared
                        .alone_since
                        .lock()
                        .map_or(false, |at| at.elapsed() >= keeper.alone_for);
                let ended = shared.ended.load(Ordering::SeqCst);
                if over && (shared.told_gone.load(Ordering::SeqCst) || alone || ended || stopped) {
                    break;
                }
                if alone && !over {
                    shared.end.notify_one();
                }
            }
        }
    }
    // The log stays: Fleet reads it after the keeper is gone.
    if !shared.ended.load(Ordering::SeqCst) {
        // Only what is still this keeper's: a replacement may have bound the
        // same paths while this one was ending.
        if inode(&keeper.socket) == mine.0 {
            let _ = std::fs::remove_file(&keeper.socket);
        }
        if inode(&keeper.spool) == mine.1 {
            let _ = std::fs::remove_file(&keeper.spool);
        }
    }
    Ok(())
}

fn inode(path: &std::path::Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|meta| meta.ino())
}

async fn client(shared: Arc<Shared>, stream: UnixStream) {
    shared.clients.fetch_add(1, Ordering::SeqCst);
    let (read, write) = stream.into_split();
    let mut write = Some(write);
    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let (word, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match word {
            "ATTACH" => {
                if let Some(write) = write.take() {
                    let mine = shared.attaches.fetch_add(1, Ordering::SeqCst) + 1;
                    shared.latest.send_replace(mine);
                    tokio::spawn(pump(Arc::clone(&shared), write, mine));
                }
            }
            "IN" => {
                // A control line asks the agent something beside its turn, and starts none.
                if !rest.starts_with(r#"{"type":"control_request""#) {
                    shared.busy.store(true, Ordering::SeqCst);
                }
                let _ = shared.stdin.send(rest.to_string());
            }
            "ACK" => {
                if let Ok(offset) = rest.parse::<u64>() {
                    shared.acked.fetch_max(offset, Ordering::SeqCst);
                }
            }
            "END" => {
                shared.ended.store(true, Ordering::SeqCst);
                shared.end.notify_one();
            }
            _ => {}
        }
    }
    if shared.clients.fetch_sub(1, Ordering::SeqCst) == 1 {
        if let Ok(mut at) = shared.alone_since.lock() {
            *at = Instant::now();
        }
    }
}

/// Send one client everything from the acknowledged offset on, then whatever
/// comes, until it is superseded or the agent is gone.
async fn pump(shared: Arc<Shared>, mut write: OwnedWriteHalf, mine: u64) {
    let mut offset = shared.acked.load(Ordering::SeqCst);
    let Ok(file) = tokio::fs::File::open(&shared.spool).await else {
        return;
    };
    let mut file = BufReader::new(file);
    if file.seek(io::SeekFrom::Start(offset)).await.is_err() {
        return;
    }
    let ready = format!(
        "READY {} {}\n",
        shared.pid,
        u8::from(shared.busy.load(Ordering::SeqCst))
    );
    if write.write_all(ready.as_bytes()).await.is_err() {
        return;
    }
    let mut progress = shared.progress.subscribe();
    let mut latest = shared.latest.subscribe();
    loop {
        if *latest.borrow_and_update() != mine {
            return;
        }
        let now = *progress.borrow_and_update();
        while offset < now.written {
            let mut line = String::new();
            match file.read_line(&mut line).await {
                Ok(n) if n > 0 => {
                    offset += n as u64;
                    let out = format!("OUT {offset} {}\n", line.trim_end_matches('\n'));
                    if write.write_all(out.as_bytes()).await.is_err() {
                        return;
                    }
                }
                _ => break,
            }
        }
        if now.over && offset >= now.written {
            if write.write_all(b"GONE\n").await.is_ok() && write.flush().await.is_ok() {
                shared.told_gone.store(true, Ordering::SeqCst);
            }
            return;
        }
        tokio::select! {
            changed = progress.changed() => if changed.is_err() { return },
            changed = latest.changed() => if changed.is_err() { return },
        }
    }
}

/// `armada session-keep <socket> <spool> <alone-secs> -- <program> <args>…`.
/// The agent's directory and environment are the keeper's own.
pub async fn run(args: Vec<String>) -> Result<(), String> {
    let Some(split) = args.iter().position(|arg| arg == "--") else {
        return Err(String::from(
            "session-keep: no `--` before the agent's command",
        ));
    };
    let (own, agent) = (&args[..split], &args[split + 1..]);
    let ([socket, spool, alone], [program, rest @ ..]) = (own, agent) else {
        return Err(String::from(
            "session-keep: expected <socket> <spool> <alone-secs> -- <program> [args…]",
        ));
    };
    let alone_for = alone
        .parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|why| format!("session-keep: alone-secs: {why}"))?;
    let reading = HeadlessAgent::at(program.clone());
    keep(Keeper {
        socket: PathBuf::from(socket),
        spool: PathBuf::from(spool),
        log: PathBuf::from(spool).with_extension("log"),
        program: program.clone(),
        args: rest.to_vec(),
        directory: None,
        alone_for,
        on_terminate: true,
        ends_turn: Arc::new(move |line| {
            reading
                .read(line)
                .iter()
                .any(|event| matches!(event, DroneEvent::Ended { .. }))
        }),
    })
    .await
    .map_err(|why| format!("session-keep: {why}"))
}
