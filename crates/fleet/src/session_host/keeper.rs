//! The process that holds a hosted session's agent so Fleet can restart under
//! it. `armada session-keep`, started by Fleet and by nothing else.
//!
//! The agent's stdin and stdout are the keeper's pipes, not Fleet's: it appends
//! every line the agent writes to a spool file and serves a unix socket Fleet
//! attaches to, so the agent and its background subagents outlive a Fleet
//! restart. `docs/concepts/session.md`, *What a restart does to a session*, has
//! the wire and the rules for when a keeper ends itself.
//!
//! One client at a time, the newest. The keeper holds the acknowledged offset,
//! so a Fleet that died with lines untaken is replayed exactly those.

use std::io::{self, Write};
use std::num::NonZeroU32;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
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
    let _ = std::fs::remove_file(&keeper.socket);
    let listener = UnixListener::bind(&keeper.socket)?;
    std::fs::set_permissions(&keeper.socket, std::fs::Permissions::from_mode(0o600))?;
    // A new file, not the old one truncated: a keeper that is ending may still
    // hold the old one and remove its path.
    let _ = std::fs::remove_file(&keeper.spool);
    let mut spool = tokio::fs::File::create(&keeper.spool).await?;
    let mine = (inode(&keeper.socket), inode(&keeper.spool));

    let mut agent = Detached::program(&keeper.program)
        .args(&keeper.args)
        .piping_input()
        .capturing_output();
    if let Some(directory) = &keeper.directory {
        agent = agent.in_directory(directory);
    }
    let mut child = agent.spawn()?;
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
        match tokio::fs::OpenOptions::new().append(true).create(true).open(&log).await {
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
        loop {
            tokio::select! {
                line = reading.next_line() => match line {
                    Ok(Some(line)) => {
                        let whole = line + "\n";
                        if spool.write_all(whole.as_bytes()).await.is_err() {
                            break;
                        }
                        written += whole.len() as u64;
                        speaking.busy.store(!ends_turn(&whole), Ordering::SeqCst);
                        speaking.progress.send_modify(|p| p.written = written);
                    }
                    _ => break,
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
        if let Ok(status) = child.wait().await {
            // Its last words are in the log before anyone is told it is over.
            let _ = complaints.await;
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(&log) {
                let _ = writeln!(file, "the agent exited: {status}");
            }
        }
        speaking.progress.send_modify(|p| p.over = true);
    });

    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                if let Ok((stream, _)) = accepted {
                    tokio::spawn(client(Arc::clone(&shared), stream));
                }
            }
            _ = tick.tick() => {
                let over = shared.progress.borrow().over;
                let alone = shared.clients.load(Ordering::SeqCst) == 0
                    && shared
                        .alone_since
                        .lock()
                        .map_or(false, |at| at.elapsed() >= keeper.alone_for);
                let ended = shared.ended.load(Ordering::SeqCst);
                if over && (shared.told_gone.load(Ordering::SeqCst) || alone || ended) {
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
                shared.busy.store(true, Ordering::SeqCst);
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
