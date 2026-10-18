//! Telling Fleet that `armada check` ran in a Session's slot. `ipc::session_checks`.
//!
//! **Best effort and silent.** A Check run by a person, with no Fleet up, or in a directory no
//! Session holds, behaves exactly as it did: Fleet answers no run, or nothing answers, and the
//! Check goes on. Fleet places the Session by the directory, so nothing here names one.

use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use checks_runner::{Attempt, Priority, Stop, Writing};
use fleet::runtime;
use ipc::{SessionCheckAct, SessionCheckAnswer, SessionCheckCall, SessionCheckState, SessionCheckStop};

use crate::declared::Ran;
use crate::loopback::Loopback;

const ROUTE: &str = "/sessions/checks";

/// A run Fleet has begun a row for, to be ended with the Check's result.
pub struct Told {
    fleet: Loopback,
    cwd: String,
    name: String,
    run: u64,
    /// Where the run's output is written as it prints, so it can be told to Fleet while it runs.
    live: PathBuf,
    /// How much of `live` Fleet has been told.
    told: AtomicU64,
}

/// How often what a run has printed is told to Fleet. It is also how soon a stop reaches the run.
const EVERY: Duration = Duration::from_millis(250);

fn say(fleet: &Loopback, call: &SessionCheckCall) -> Option<SessionCheckAnswer> {
    let body = ipc::encode(call).ok()?;
    let answer = fleet.post(ROUTE, body.as_bytes()).ok()?;
    (answer.status == 200)
        .then(|| ipc::decode("a Job Check answer", &answer.body).ok())
        .flatten()
}

/// Tell Fleet a Check `name` is starting in `cwd`, with the test filter it was given if any.
/// `None` where no Session holds that slot. A `--changed` run tells its target at the end instead,
/// since what the change reaches is only resolved as the Check runs.
pub fn begin(cwd: &Path, name: &str, filter: Option<&str>) -> Option<Told> {
    let at = runtime::machine_path().ok()?;
    let fleet = Loopback::at(crate::mcp::listening(runtime::read(&at), &at).ok()?);
    let cwd = cwd.to_str()?.to_string();
    let call = SessionCheckCall {
        act: SessionCheckAct::Start,
        cwd: cwd.clone(),
        name: name.to_string(),
        target: filter.map(str::to_string),
        run: None,
        outcome: None,
        took_ms: None,
        log: None,
    };
    let run = say(&fleet, &call)?.run?;
    Some(Told {
        fleet,
        cwd,
        name: name.to_string(),
        run,
        live: std::env::temp_dir().join(format!("armada-check-{}-{run}.log", std::process::id())),
        told: AtomicU64::new(0),
    })
}

impl Told {
    /// `command` in `root`, with its output told to Fleet as it prints and ended by a stop a person
    /// asks for there. **A stop is carried out on the Check's whole process group**: `SIGTERM` and
    /// then `SIGKILL` after the grace period, or `SIGKILL` at once.
    pub async fn run(
        &self,
        command: &str,
        root: &Path,
        budget: Duration,
        env: &[(String, String)],
        priority: Priority,
    ) -> Attempt {
        let stop = async {
            loop {
                tokio::time::sleep(EVERY).await;
                if let Some(stop) = self.chunk() {
                    return stop;
                }
            }
        };
        checks_runner::run_stoppable(
            command,
            root,
            budget,
            Writing::Appending(&self.live),
            env,
            stop,
            priority,
            checks_runner::GRACE,
        )
        .await
    }

    /// Tell Fleet what printed since the last time, and answer the stop it holds for this run.
    /// **Nothing is told twice and nothing is dropped**: the offset moves only where Fleet answered.
    fn chunk(&self) -> Option<Stop> {
        let (text, upto) = self.unsent();
        let call = SessionCheckCall {
            act: SessionCheckAct::Chunk,
            cwd: self.cwd.clone(),
            name: self.name.clone(),
            target: None,
            run: Some(self.run),
            outcome: None,
            took_ms: None,
            log: Some(text),
        };
        let answer = say(&self.fleet, &call)?;
        self.told.store(upto, Ordering::Relaxed);
        answer.stop.map(|stop| match stop {
            SessionCheckStop::Graceful => Stop::Graceful,
            SessionCheckStop::Force => Stop::Force,
        })
    }

    /// The whole characters written since the last chunk, and where they end.
    fn unsent(&self) -> (String, u64) {
        let from = self.told.load(Ordering::Relaxed);
        let mut bytes = Vec::new();
        let read = std::fs::File::open(&self.live).and_then(|mut file| {
            file.seek(SeekFrom::Start(from))?;
            file.read_to_end(&mut bytes)
        });
        if read.is_err() {
            return (String::new(), from);
        }
        // A character cut by the read is left for the next one.
        let cut = match std::str::from_utf8(&bytes) {
            Err(why) if why.error_len().is_none() => why.valid_up_to(),
            _ => bytes.len(),
        };
        (String::from_utf8_lossy(&bytes[..cut]).into_owned(), from + cut as u64)
    }

    /// Tell Fleet how it ended, with what it printed.
    pub fn end(self, ran: &Ran, took: Duration) {
        let output = &ran.attempt.output;
        let log = format!("$ {}\n{}{}", ran.command, output.stdout, output.stderr);
        self.ended(ran.status() == 0, took, log, ran.target.clone());
    }

    /// The Check could not be run at all, and Fleet is told that, so the row does not stay out.
    pub fn refused(self, why: &str, took: Duration) {
        self.ended(false, took, why.to_string(), None);
    }

    fn ended(self, passed: bool, took: Duration, log: String, target: Option<String>) {
        // The last of what printed, so a viewer still following reads to the end.
        self.chunk();
        let _ = std::fs::remove_file(&self.live);
        let call = SessionCheckCall {
            act: SessionCheckAct::End,
            cwd: self.cwd,
            name: self.name,
            target,
            run: Some(self.run),
            outcome: Some(match passed {
                true => SessionCheckState::Passed,
                false => SessionCheckState::Failed,
            }),
            took_ms: Some(u64::try_from(took.as_millis()).unwrap_or(u64::MAX)),
            log: Some(log),
        };
        say(&self.fleet, &call);
    }
}
