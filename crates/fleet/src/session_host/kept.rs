//! Fleet's end of a keeper's socket: a [`Process`] that is a connection.
//! `keeper.rs` holds the other end and the reasons.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::AgentHarness;
use adapters::HeadlessAgent;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use super::process::{Heard, Process, Sink};

/// How long a keeper has to answer an attach.
const READY_WITHIN: Duration = Duration::from_secs(5);

/// A session's agent, reached through its keeper.
struct Kept {
    pid: Arc<AtomicU32>,
    out: mpsc::UnboundedSender<String>,
    reading: tokio::task::AbortHandle,
    socket: PathBuf,
    spool: PathBuf,
}

impl Drop for Kept {
    /// Let the connection go. Lines already queued are still written: the
    /// writer ends when its queue is empty and nothing can add to it.
    fn drop(&mut self) {
        self.reading.abort();
    }
}

impl Process for Kept {
    fn pid(&self) -> Option<u32> {
        Some(self.pid.load(Ordering::SeqCst)).filter(|pid| *pid != 0)
    }

    fn send(&self, line: String) {
        let _ = self.out.send(format!("IN {line}"));
    }

    /// **The keeper's paths go first, here and not in the keeper.** The next
    /// message may start this session's next keeper before this one has read
    /// `END`, and it must not find this one at the socket, or reattach to an
    /// agent that is about to be gone.
    fn end(&self) {
        let _ = std::fs::remove_file(&self.socket);
        let _ = std::fs::remove_file(&self.spool);
        let _ = self.out.send(String::from("END"));
    }

    fn complaint(&self) -> String {
        tail_of(&self.spool.with_extension("log"))
    }
}

/// The last lines of a keeper's log, each on a line of its own after a newline,
/// or nothing where the log is empty or unreadable.
pub(crate) fn tail_of(log: &Path) -> String {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let from = lines.len().saturating_sub(TAIL_LINES);
    match lines[from..].join("\n") {
        tail if tail.is_empty() => tail,
        tail => format!("\n{tail}"),
    }
}

/// How much of a log a row carries.
const TAIL_LINES: usize = 8;

/// Whether a keeper answers at `socket`. A socket nobody answers on is a
/// keeper that is gone, and is removed so the next start can bind there.
pub(crate) fn connect(socket: &Path) -> Option<std::os::unix::net::UnixStream> {
    match std::os::unix::net::UnixStream::connect(socket) {
        Ok(stream) => Some(stream),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => {
            let _ = std::fs::remove_file(socket);
            None
        }
    }
}

/// Attach to a keeper on a connection already made. Returns at once: the
/// handshake and everything after it happen on tasks, and what the agent says
/// reaches `sink` as it would from a process Fleet started.
pub(crate) fn attached(
    stream: std::os::unix::net::UnixStream,
    agent: HeadlessAgent,
    sink: Sink,
    socket: &Path,
    spool: &Path,
) -> Result<Arc<dyn Process>, String> {
    stream
        .set_nonblocking(true)
        .map_err(|why| why.to_string())?;
    let stream = UnixStream::from_std(stream).map_err(|why| why.to_string())?;
    let (read, mut write) = stream.into_split();
    let pid = Arc::new(AtomicU32::new(0));
    let (out, mut outbox) = mpsc::unbounded_channel::<String>();
    let _ = out.send(String::from("ATTACH"));
    tokio::spawn(async move {
        while let Some(line) = outbox.recv().await {
            let line = line + "\n";
            if write.write_all(line.as_bytes()).await.is_err() || write.flush().await.is_err() {
                break;
            }
        }
    });
    let ack = out.clone();
    let known = Arc::clone(&pid);
    let reading = tokio::spawn(async move {
        let mut lines = BufReader::new(read).lines();
        let ready = tokio::time::timeout(READY_WITHIN, lines.next_line()).await;
        let busy = match ready {
            Ok(Ok(Some(line))) => ready_line(&line),
            _ => None,
        };
        let Some((keeper_pid, busy)) = busy else {
            let _ = sink.send(Heard::Gone);
            return;
        };
        known.store(keeper_pid, Ordering::SeqCst);
        let _ = sink.send(Heard::Attached { busy });
        while let Ok(Some(line)) = lines.next_line().await {
            let Some(rest) = line.strip_prefix("OUT ") else {
                break;
            };
            let Some((offset, text)) = rest.split_once(' ') else {
                continue;
            };
            if let Some(names) = adapters::init_commands(text) {
                let _ = sink.send(Heard::Commands(names));
            }
            let _ = sink.send(Heard::Events(agent.read(text)));
            let _ = ack.send(format!("ACK {offset}"));
        }
        let _ = sink.send(Heard::Gone);
    });
    Ok(Arc::new(Kept {
        pid,
        out,
        reading: reading.abort_handle(),
        socket: socket.to_path_buf(),
        spool: spool.to_path_buf(),
    }))
}

/// `READY <pid> <busy>`.
fn ready_line(line: &str) -> Option<(u32, bool)> {
    let mut words = line.strip_prefix("READY ")?.split(' ');
    let pid = words.next()?.parse().ok()?;
    let busy = words.next()? == "1";
    Some((pid, busy))
}
