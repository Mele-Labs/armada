//! The Fleet connection, on a thread and a tokio runtime of its own. gpui's main thread never
//! waits on it: everything it learns crosses one channel to [`crate::fleet::FleetConnection`].
//!
//! Mirrors `apps/desktop/src/main/socket.ts`: read the runtime file, check the pid and the
//! protocol, connect, and keep trying on a drop. Reconnects back off from one second to ten and
//! start over after a connection that said something. A socket that accepts and stays silent is
//! `Starting` while the Fleet process is younger than the boot window and `Unreachable` after.

use std::path::PathBuf;
use std::time::Duration;

use async_channel::Sender;
use chrono::Local;
use tokio::time::{sleep, Instant};

use ipc::{ProtocolId, SessionList, SessionRecord, StreamMessage};

use crate::frames::{self, Messages};
use crate::runtime_file::{self, Presence, RuntimeFile};
use crate::transport;

/// How long a socket may sit without a first message before this says so.
const ANSWER_GRACE: Duration = Duration::from_millis(1500);

/// How old a Fleet process may be and still be booting: reading every Job takes one to two minutes
/// (measured 6 Oct 2026), and five leaves that room twice over.
const BOOT_WINDOW_MS: i64 = 5 * 60_000;

/// How often a Fleet that is old and silent is re-reported while the socket stays open.
const SILENT_RECHECK: Duration = Duration::from_secs(30);

const BACKOFF_FIRST: Duration = Duration::from_secs(1);
const BACKOFF_CAP: Duration = Duration::from_secs(10);

const JOBS_TRIES: u32 = 3;

/// Where the connection stands. Never leaves stale state reading as live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkState {
    NotRunning(String),
    Refused(String),
    ProtocolMismatch { fleet: ProtocolId, ours: ProtocolId },
    Connecting { port: u16, pid: u32 },
    Starting,
    Unreachable(String),
    Connected,
}

/// What the link tells the entity.
#[derive(Debug)]
pub enum Signal {
    State(LinkState),
    /// The Jobs, fetched once on connect, as `GET /sessions` answered.
    Jobs(Vec<SessionRecord>),
    JobsFailed(String),
    Stream(StreamMessage),
}

/// Run until the receiving end is gone.
pub async fn run(tx: Sender<Signal>, home: Option<String>) {
    let mut wait = BACKOFF_FIRST;
    loop {
        let Some(path) = runtime_file::path(home.as_deref()) else {
            let why = "HOME is not set, so the machine directory cannot be resolved";
            if tell(&tx, LinkState::Refused(why.into())).await.is_err() {
                return;
            }
            sleep(wait).await;
            continue;
        };
        match attach(&tx, path).await {
            Ended::NobodyListening => return,
            Ended::Spoke => wait = BACKOFF_FIRST,
            Ended::Silent => {}
        }
        sleep(wait).await;
        wait = (wait * 2).min(BACKOFF_CAP);
    }
}

enum Ended {
    NobodyListening,
    /// The connection said something before it ended.
    Spoke,
    Silent,
}

async fn tell(tx: &Sender<Signal>, state: LinkState) -> Result<(), ()> {
    send(tx, Signal::State(state)).await
}

async fn send(tx: &Sender<Signal>, signal: Signal) -> Result<(), ()> {
    tx.send(signal).await.map_err(|_| ())
}

macro_rules! tell_or_stop {
    ($tx:expr, $state:expr) => {
        if tell($tx, $state).await.is_err() {
            return Ended::NobodyListening;
        }
    };
}

async fn attach(tx: &Sender<Signal>, path: PathBuf) -> Ended {
    // The probe shells out to `ps`, so it runs where it cannot hold this runtime.
    let presence = tokio::task::spawn_blocking(move || {
        runtime_file::classify(std::fs::read(&path), runtime_file::holder_of)
    })
    .await
    .unwrap_or_else(|why| Presence::Refused(format!("the runtime file read failed: {why}")));

    let fleet = match presence {
        // Neither opens a socket: a stale file's port may not be Fleet's.
        Presence::Absent => {
            tell_or_stop!(tx, LinkState::NotRunning("no runtime file".into()));
            return Ended::Silent;
        }
        Presence::Stale(why) => {
            tell_or_stop!(tx, LinkState::NotRunning(why));
            return Ended::Silent;
        }
        Presence::Refused(why) => {
            tell_or_stop!(tx, LinkState::Refused(why));
            return Ended::Silent;
        }
        Presence::Running(fleet) => fleet,
    };
    if let Err(speaks) = runtime_file::speaks_ours(&fleet) {
        tell_or_stop!(tx, LinkState::ProtocolMismatch { fleet: speaks, ours: ProtocolId::current() });
        return Ended::Silent;
    }
    tell_or_stop!(tx, LinkState::Connecting { port: fleet.port, pid: fleet.pid });
    follow(tx, &fleet).await
}

async fn follow(tx: &Sender<Signal>, fleet: &RuntimeFile) -> Ended {
    let io = match transport::upgrade(fleet.port, "/events").await {
        Ok(io) => io,
        Err(why) => {
            tell_or_stop!(tx, LinkState::Unreachable(why));
            return Ended::Silent;
        }
    };
    let mut messages = Messages::new(io);

    // The first message is awaited under a timer and never cancelled: a frame half-read when the
    // timer fires would lose its bytes, so the same future is polled again.
    let first = {
        let wait = messages.next();
        tokio::pin!(wait);
        let timer = sleep(ANSWER_GRACE);
        tokio::pin!(timer);
        loop {
            tokio::select! {
                read = &mut wait => break read,
                _ = &mut timer => {
                    let age = runtime_file::age_ms(&fleet.started_at, Local::now());
                    // A start time that will not parse says nothing about age, so it cannot earn `Starting`.
                    match age.filter(|age| *age < BOOT_WINDOW_MS) {
                        Some(age) => {
                            tell_or_stop!(tx, LinkState::Starting);
                            let left = Duration::from_millis((BOOT_WINDOW_MS - age) as u64);
                            timer.as_mut().reset(Instant::now() + left);
                        }
                        None => {
                            tell_or_stop!(tx, LinkState::Unreachable("the socket has not answered".into()));
                            timer.as_mut().reset(Instant::now() + SILENT_RECHECK);
                        }
                    }
                }
            }
        }
    };
    let Some(first) = first else {
        tell_or_stop!(tx, LinkState::Unreachable("the connection closed".into()));
        return Ended::Silent;
    };

    // Fleet says what it is on the first message of every connection; a protocol this build will
    // not speak ends the connection rather than reading on.
    let mut text = first;
    let mut jobs_asked = false;
    loop {
        match frames::decode(&text) {
            Ok(StreamMessage::Resync(resync)) if resync.protocol_id != ProtocolId::current() => {
                tell_or_stop!(
                    tx,
                    LinkState::ProtocolMismatch { fleet: resync.protocol_id, ours: ProtocolId::current() }
                );
                return Ended::Silent;
            }
            Ok(message) => {
                if !jobs_asked {
                    jobs_asked = true;
                    tell_or_stop!(tx, LinkState::Connected);
                    tokio::spawn(fetch_jobs(tx.clone(), fleet.port));
                }
                if send(tx, Signal::Stream(message)).await.is_err() {
                    return Ended::NobodyListening;
                }
            }
            Err(why) => {
                // A message this build cannot read is a build that disagrees with Fleet; closing
                // reconnects and reads the runtime file again.
                tell_or_stop!(tx, LinkState::Unreachable(format!("a message could not be read: {why}")));
                return Ended::Silent;
            }
        }
        match messages.next().await {
            Some(next) => text = next,
            None => {
                tell_or_stop!(tx, LinkState::Unreachable("the connection closed".into()));
                return Ended::Spoke;
            }
        }
    }
}

/// `GET /sessions`: the Jobs, once per connection. Retried a few times, because a read that times
/// out on a busy machine must not leave the window without them.
async fn fetch_jobs(tx: Sender<Signal>, port: u16) {
    let mut last = String::new();
    for attempt in 0..JOBS_TRIES {
        if attempt > 0 {
            sleep(BACKOFF_FIRST * attempt).await;
        }
        match transport::get(port, "/sessions").await {
            Ok((200, body)) => match ipc::decode::<SessionList>("job list", &body) {
                Ok(list) => {
                    let _ = tx.send(Signal::Jobs(list.sessions)).await;
                    return;
                }
                Err(why) => last = why.to_string(),
            },
            Ok((status, _)) => last = format!("Fleet answered {status}"),
            Err(why) => last = why,
        }
    }
    let _ = tx.send(Signal::JobsFailed(last)).await;
}
