//! The Fleet connection, on a thread and a tokio runtime of its own. gpui's main thread never
//! waits on it: everything it learns crosses one channel to [`crate::fleet::FleetConnection`].
//!
//! Mirrors `apps/desktop/src/main/socket.ts`: read the runtime file, check the pid and the
//! protocol, connect, and keep trying on a drop. Reconnects back off from one second to ten and
//! start over after a connection that said something. A socket that accepts and stays silent is
//! `Starting` while the Fleet process is younger than the boot window and `Unreachable` after.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use async_channel::Sender;
use chrono::Local;
use tokio::time::{sleep, Instant};

use ipc::{
    Event, FleetBuildReport, MergeLineViews, ModChecked, ModKind, ModList, ModSummary, Preferences, Projects, ProtocolId, RepositoryList, SessionList,
    SessionRecord, SettingsList, StandingCalls, StreamMessage,
};

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

/// How often the build is read while a connection stands. No event says the build moved: a fetch in
/// Fleet moves its position, and a restart moves the build with the socket down
/// (`apps/desktop/src/main/fleet-build.ts`).
const BUILD_WATCH: Duration = Duration::from_secs(60);

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
    /// What the runtime file said on this attempt, with the file's path: the title bar's words for a Fleet down read it.
    Presence(Presence, String),
    State(LinkState),
    /// The Jobs, fetched once on connect, as `GET /sessions` answered.
    Jobs(Vec<SessionRecord>),
    JobsFailed(String),
    /// `GET /preferences`, once per connection. Nothing but a save changes them.
    Preferences(Preferences),
    PreferencesFailed(String),
    /// `GET /mods`, once per connection; `mods.changed` carries the list after that.
    Mods(ModList),
    ModsFailed(String),
    /// Fleet's check of one theme mod, with the `changed_at` of the row it was asked for. Bridge
    /// draws the sheet Fleet checked and no other.
    ModChecked { checked: ModChecked, changed_at: Option<ipc::Instant> },
    /// `GET /fleet/build/offers`, on connect and after a `Missed`; `preview_offers.changed` carries it whole after that.
    Offers(ipc::PreviewOffers),
    OffersFailed(String),
    /// `GET /queued`, on connect and after a `Missed`; `queued.changed` carries it whole after that.
    Queued(ipc::QueuedList),
    QueuedFailed(String),
    /// `GET /fleet/build`, on connect and every [`BUILD_WATCH`] after.
    Build(FleetBuildReport),
    BuildFailed(String),
    /// `GET /calls`, once per connection; `calls.changed` carries the list after that.
    Calls(StandingCalls),
    CallsFailed(String),
    /// `GET /settings`, once per connection; `settings.changed` carries the list after that.
    Settings(SettingsList),
    SettingsFailed(String),
    /// `GET /repositories`, once per connection; `repositories.changed` carries the list after that.
    Repositories(RepositoryList),
    RepositoriesFailed(String),
    /// `GET /projects`, once per connection; `projects.changed` carries all of them after that.
    Projects(Projects),
    ProjectsFailed(String),
    /// `GET /workflows`, once per connection: the head's picker and the steps a Job's workflow names.
    Workflows(Vec<ipc::WorkflowSummary>),
    WorkflowsFailed(String),
    /// `GET /merge_lines/view`, on connect and after a `Missed`; `merge_line_view.changed` carries it after that.
    MergeLines(MergeLineViews),
    MergeLinesFailed(String),
    /// `GET /plan_usage`, once per connection; `plan_usage.changed` carries it after that.
    PlanUsage(ipc::PlanUsage),
    PlanUsageFailed(String),
    /// `GET /model_downshift`, once per connection; `model_downshift.changed` carries it after that.
    Downshift(ipc::ModelDownshiftState),
    DownshiftFailed(String),
    /// `GET /clashes/all`, on connect and after a `Missed`; `write_clashes.changed` carries it after that.
    Clashes(ipc::AllWriteClashes),
    ClashesFailed(String),
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
    let said = path.display().to_string();
    // The probe shells out to `ps`, so it runs where it cannot hold this runtime.
    let presence = tokio::task::spawn_blocking(move || {
        runtime_file::classify(std::fs::read(&path), runtime_file::holder_of)
    })
    .await
    .unwrap_or_else(|why| Presence::Refused(format!("the runtime file read failed: {why}")));
    if send(tx, Signal::Presence(presence.clone(), said)).await.is_err() {
        return Ended::NobodyListening;
    }

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
    let mut _build_watch: Option<AbortOnDrop> = None;
    // What has been sent to Fleet to check, by mod and the `changed_at` it was asked at.
    let mut asked: HashMap<String, Option<ipc::Instant>> = HashMap::new();
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
                    tokio::spawn(fetch_preferences(tx.clone(), fleet.port));
                    tokio::spawn(fetch_mods(tx.clone(), fleet.port));
                    tokio::spawn(fetch_calls(tx.clone(), fleet.port));
                    tokio::spawn(fetch_settings(tx.clone(), fleet.port));
                    tokio::spawn(fetch_repositories(tx.clone(), fleet.port));
                    tokio::spawn(fetch_projects(tx.clone(), fleet.port));
                    tokio::spawn(fetch_workflows(tx.clone(), fleet.port));
                    tokio::spawn(fetch_merge_lines(tx.clone(), fleet.port));
                    tokio::spawn(fetch_plan(tx.clone(), fleet.port));
                    tokio::spawn(fetch_offers(tx.clone(), fleet.port));
                    tokio::spawn(fetch_queued(tx.clone(), fleet.port));
                    tokio::spawn(fetch_clashes(tx.clone(), fleet.port));
                    // Stops with this connection: the guard aborts the task when `follow` returns.
                    _build_watch = Some(AbortOnDrop(tokio::spawn(watch_build(tx.clone(), fleet.port))));
                }
                if let StreamMessage::Event(delivered) = &message {
                    if let Event::ModsChanged(list) = &delivered.event {
                        check_themes(tx, fleet.port, &list.mods, &mut asked);
                    }
                }
                // A view sent whole is read again after events were lost, or the window would hold a line it never heard.
                if let StreamMessage::Missed(_) = &message {
                    tokio::spawn(fetch_merge_lines(tx.clone(), fleet.port));
                    tokio::spawn(fetch_clashes(tx.clone(), fleet.port));
                    tokio::spawn(fetch_offers(tx.clone(), fleet.port));
                    tokio::spawn(fetch_queued(tx.clone(), fleet.port));
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

/// Ends a task with the scope that holds it.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// `GET /fleet/build` now and every [`BUILD_WATCH`], for as long as the connection that started it stands.
/// A read that failed says so and leaves the report held: the commonest cause is a restart under
/// way, and the item going dark in the middle of one would say it had finished.
async fn watch_build(tx: Sender<Signal>, port: u16) {
    loop {
        let signal = match read_with_retries::<FleetBuildReport>(port, "/fleet/build", "build report").await {
            Ok(report) => Signal::Build(report),
            Err(why) => Signal::BuildFailed(why),
        };
        if tx.send(signal).await.is_err() {
            return;
        }
        sleep(BUILD_WATCH).await;
    }
}

/// `GET /preferences`, once per connection, retried as the Jobs are.
async fn fetch_preferences(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<Preferences>(port, "/preferences", "preferences").await {
        Ok(preferences) => Signal::Preferences(preferences),
        Err(why) => Signal::PreferencesFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /calls`, once per connection, retried as the Jobs are.
async fn fetch_calls(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<StandingCalls>(port, "/calls", "calls").await {
        Ok(calls) => Signal::Calls(calls),
        Err(why) => Signal::CallsFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /settings`, once per connection, retried as the Jobs are.
async fn fetch_settings(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<SettingsList>(port, "/settings", "settings").await {
        Ok(settings) => Signal::Settings(settings),
        Err(why) => Signal::SettingsFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /repositories`, once per connection, retried as the Jobs are.
async fn fetch_repositories(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<RepositoryList>(port, "/repositories", "repository list").await {
        Ok(list) => Signal::Repositories(list),
        Err(why) => Signal::RepositoriesFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /projects`, once per connection, retried as the Jobs are.
async fn fetch_projects(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<Projects>(port, "/projects", "projects").await {
        Ok(projects) => Signal::Projects(projects),
        Err(why) => Signal::ProjectsFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /workflows`, once per connection, retried as the Jobs are.
async fn fetch_workflows(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<Vec<ipc::WorkflowSummary>>(port, "/workflows", "workflows").await {
        Ok(list) => Signal::Workflows(list),
        Err(why) => Signal::WorkflowsFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /plan_usage` and `GET /model_downshift`, once per connection, retried as the Jobs are.
async fn fetch_plan(tx: Sender<Signal>, port: u16) {
    let usage = match read_with_retries::<ipc::PlanUsage>(port, "/plan_usage", "plan usage").await {
        Ok(usage) => Signal::PlanUsage(usage),
        Err(why) => Signal::PlanUsageFailed(why),
    };
    if tx.send(usage).await.is_err() {
        return;
    }
    let downshift = match read_with_retries::<ipc::ModelDownshiftState>(port, "/model_downshift", "model downshift").await {
        Ok(state) => Signal::Downshift(state),
        Err(why) => Signal::DownshiftFailed(why),
    };
    let _ = tx.send(downshift).await;
}

/// `GET /fleet/build/offers`, retried as the Jobs are.
async fn fetch_offers(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<ipc::PreviewOffers>(port, "/fleet/build/offers", "preview offers").await {
        Ok(offers) => Signal::Offers(offers),
        Err(why) => Signal::OffersFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /queued`, retried as the Jobs are.
async fn fetch_queued(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<ipc::QueuedList>(port, "/queued", "sends waiting for another Mac").await {
        Ok(list) => Signal::Queued(list),
        Err(why) => Signal::QueuedFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /merge_lines/view`, retried as the Jobs are.
async fn fetch_merge_lines(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<MergeLineViews>(port, "/merge_lines/view", "merge line view").await {
        Ok(view) => Signal::MergeLines(view),
        Err(why) => Signal::MergeLinesFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /clashes/all`, retried as the Jobs are.
async fn fetch_clashes(tx: Sender<Signal>, port: u16) {
    let signal = match read_with_retries::<ipc::AllWriteClashes>(port, "/clashes/all", "write clashes").await {
        Ok(all) => Signal::Clashes(all),
        Err(why) => Signal::ClashesFailed(why),
    };
    let _ = tx.send(signal).await;
}

/// `GET /mods`, once per connection; then every theme mod on it is asked for.
async fn fetch_mods(tx: Sender<Signal>, port: u16) {
    match read_with_retries::<ModList>(port, "/mods", "mod list").await {
        Ok(list) => {
            let mods = list.mods.clone();
            if tx.send(Signal::Mods(list)).await.is_ok() {
                // A list read at connect starts the checks; none has been asked yet on this connection.
                check_themes(&tx, port, &mods, &mut HashMap::new());
            }
        }
        Err(why) => {
            let _ = tx.send(Signal::ModsFailed(why)).await;
        }
    }
}

/// Ask Fleet to check each theme mod that is on and valid, once for each version of it. A mod that
/// is off or broken is not asked about: it cannot be drawn, whatever its sheet says.
fn check_themes(
    tx: &Sender<Signal>,
    port: u16,
    mods: &[ModSummary],
    asked: &mut HashMap<String, Option<ipc::Instant>>,
) {
    for row in mods {
        let is_theme = row.kind.is_none_or(|kind| kind == ModKind::Theme);
        if !(is_theme && row.enabled && row.valid) || asked.get(&row.name) == Some(&row.changed_at) {
            continue;
        }
        asked.insert(row.name.clone(), row.changed_at.clone());
        tokio::spawn(check_theme(tx.clone(), port, row.name.clone(), row.changed_at.clone()));
    }
}

/// `GET /mods/validate?name=`: the stylesheet as Fleet checked it.
async fn check_theme(tx: Sender<Signal>, port: u16, name: String, changed_at: Option<ipc::Instant>) {
    let path = format!("/mods/validate?name={}", percent_encoded(&name));
    match read_with_retries::<ModChecked>(port, &path, "mod check").await {
        Ok(checked) => {
            let _ = tx.send(Signal::ModChecked { checked, changed_at }).await;
        }
        Err(why) => {
            crate::log::Note::warn("a mod could not be checked").field("mod", name).field("why", why).emit();
        }
    }
}

/// A GET that decodes into `T`, tried a few times: a read that times out on a busy machine must not
/// leave the window without it.
async fn read_with_retries<T: serde::de::DeserializeOwned>(
    port: u16,
    path: &str,
    expected: &'static str,
) -> Result<T, String> {
    let mut last = String::new();
    for attempt in 0..JOBS_TRIES {
        if attempt > 0 {
            sleep(BACKOFF_FIRST * attempt).await;
        }
        match transport::get(port, path).await {
            Ok((200, body)) => match ipc::decode::<T>(expected, &body) {
                Ok(read) => return Ok(read),
                Err(why) => last = why.to_string(),
            },
            Ok((status, _)) => last = format!("Fleet answered {status}"),
            Err(why) => last = why,
        }
    }
    Err(last)
}

/// A query value with everything but unreserved characters escaped.
pub(crate) fn percent_encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mod_name_with_a_colon_is_escaped_in_the_query() {
        assert_eq!(percent_encoded("built-in:calm"), "built-in%3Acalm");
        assert_eq!(percent_encoded("dusk"), "dusk");
        assert_eq!(percent_encoded("a b"), "a%20b");
    }
}
