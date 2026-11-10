//! A repair's process: started for one turn, read to its end, stopped when the Job moves on or
//! the time is up. **Beside a scout's host, and one that writes**: no worktree of its own and no
//! place under the concurrency cap, since it works in a checkout that already exists.
//!
//! **A stop is an interrupt first**, as a scout's: the agent ends its turn and reports, and one that
//! has not ended after [`GRACE`] has its group ended. Never written down: a repair does not outlive
//! the Fleet reading it.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, McpConfig, Model, Speaker};
use adapters::{HeadlessAgent, Repairing};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::Notify;

use crate::daemon::Host;
use crate::detach::Detached;
use crate::drone::{environment, HostPaths};
use crate::scout::{Turn, COMPLAINT, GRACE};

/// How long a repair runs before it is stopped. Twenty minutes, and nothing measured it.
pub(crate) const REPAIR_LIMIT: Duration = Duration::from_secs(20 * 60);

/// The server file a repair is started against, beside a scout's own.
const SERVERS_FILE: &str = "repair-mcp.json";

/// How a repair's process ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Finish {
    /// It ended its turn itself.
    Ended,
    /// It was still going at [`REPAIR_LIMIT`].
    TimedOut,
    /// The Job moved on, and it was told to stop.
    Stopped,
    /// It would not start, or ended without finishing its turn.
    Failed(String),
}

/// What a repair said last and how it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepairEnded {
    pub(crate) said: Option<String>,
    pub(crate) finish: Finish,
}

/// The agent CLI, one process per repair.
pub struct RepairHost {
    agent: HeadlessAgent,
    model: String,
    servers: PathBuf,
    path: String,
    home: String,
    user: String,
    width: checks_runner::CheckWidth,
    grace: Duration,
    limit: Duration,
}

impl RepairHost {
    pub fn new(agent: HeadlessAgent, model: &str, servers: PathBuf, host: HostPaths<'_>) -> RepairHost {
        RepairHost {
            agent,
            model: model.to_string(),
            servers,
            path: host.path.to_string(),
            home: host.home.to_string(),
            user: host.user.to_string(),
            width: host.width,
            grace: GRACE,
            limit: REPAIR_LIMIT,
        }
    }

    /// The CLI and the model a hosted Job's host runs.
    pub(crate) fn on_this_machine(host: &Host, width: checks_runner::CheckWidth) -> RepairHost {
        RepairHost::new(
            HeadlessAgent::at(host.agent_binary.clone()),
            HeadlessAgent::default_model(),
            Path::new(&host.mcp_config).with_file_name(SERVERS_FILE),
            HostPaths { path: &host.path, home: &host.home, user: &host.user, width },
        )
    }

    /// The same host, stopping a repair after `limit`: a test's, in place of twenty minutes.
    #[cfg(test)]
    pub(crate) fn stopping_after(mut self, limit: Duration, grace: Duration) -> RepairHost {
        (self.limit, self.grace) = (limit, grace);
        self
    }

    /// Run one repair in `directory`, told `told` on its only turn, until it ends or is stopped.
    pub(crate) async fn run(&self, directory: &str, told: &str, running: &Running) -> RepairEnded {
        match self.start(directory, told).await {
            Ok(started) => self.read(started, running).await,
            Err(why) => RepairEnded { said: None, finish: Finish::Failed(why) },
        }
    }

    async fn start(&self, directory: &str, told: &str) -> Result<Started, String> {
        adapters::no_servers(&self.servers).map_err(|why| format!("the repair's server file would not be written: {why}"))?;
        let model = Model::named(&self.model).map_err(|why| why.said())?;
        let servers = McpConfig::only_these(&self.servers.to_string_lossy()).map_err(|why| why.said())?;
        let paths = HostPaths { path: &self.path, home: &self.home, user: &self.user, width: self.width };
        let env = environment(paths, &[]).map_err(|why| why.said())?;
        let repairing = Repairing::in_checkout(directory, model, servers, env).map_err(|why| why.to_string())?;
        let launch = self.agent.render_repair(&repairing).map_err(|why| why.to_string())?;
        let mut child = Detached::launching(&launch)
            .piping_input()
            .capturing_output()
            .spawn()
            .map_err(|why| format!("{} would not start: {why}", launch.program()))?;
        let pid = child.id().and_then(NonZeroU32::new);
        let turn = ipc::encode(&Turn::of(told)).map_err(|why| why.to_string())? + "\n";
        if let Some(mut input) = child.stdin.take() {
            // Closed behind the brief: one turn, and the process exits after it.
            let _ = input.write_all(turn.as_bytes()).await;
        }
        Ok(Started { child, pid })
    }

    /// Read a started repair to its end, keeping what it said last. Interrupted when `running` is
    /// stopped or the limit is reached, and its group ended if it has not ended [`GRACE`] after.
    async fn read(&self, started: Started, running: &Running) -> RepairEnded {
        let Started { mut child, pid } = started;
        let (Some(output), Some(mut complaints)) = (child.stdout.take(), child.stderr.take()) else {
            let _ = child.wait().await;
            return RepairEnded { said: None, finish: Finish::Failed("the repair was gone before Fleet could read it".to_string()) };
        };
        let complained = tokio::spawn(async move {
            let mut kept = Vec::new();
            let _ = (&mut complaints).take(COMPLAINT).read_to_end(&mut kept).await;
            let _ = tokio::io::copy(&mut complaints, &mut tokio::io::sink()).await;
            String::from_utf8_lossy(&kept).trim().to_string()
        });
        let mut lines = BufReader::new(output).lines();
        let mut said: Option<String> = None;
        let mut ended_itself = false;
        let mut interrupted = false;
        let mut timed_out = false;
        let mut deadline: Option<tokio::time::Instant> = None;
        let stop = running.stop.notified();
        tokio::pin!(stop);
        let limit = tokio::time::sleep(self.limit);
        tokio::pin!(limit);
        loop {
            let ending = async {
                match deadline {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                line = lines.next_line() => {
                    let Ok(Some(line)) = line else { break };
                    for event in AgentHarness::read(&self.agent, &line) {
                        match event {
                            DroneEvent::Said { text, by: Speaker::Drone } => said = Some(text),
                            DroneEvent::Ended { .. } => ended_itself = true,
                            _ => {}
                        }
                    }
                }
                _ = &mut stop, if !interrupted => {
                    interrupted = true;
                    if let Some(group) = pid {
                        crate::group::interrupt_the_group(group);
                    }
                    deadline = Some(tokio::time::Instant::now() + self.grace);
                }
                _ = &mut limit, if !interrupted => {
                    interrupted = true;
                    timed_out = true;
                    if let Some(group) = pid {
                        crate::group::interrupt_the_group(group);
                    }
                    deadline = Some(tokio::time::Instant::now() + self.grace);
                }
                _ = ending => {
                    deadline = None;
                    if let Some(group) = pid {
                        crate::group::end_the_group(group);
                    }
                }
            }
        }
        let status = child.wait().await;
        let finish = if timed_out {
            Finish::TimedOut
        } else if interrupted || running.stopping() {
            Finish::Stopped
        } else if ended_itself {
            Finish::Ended
        } else {
            let complaint = complained.await.unwrap_or_default();
            let exited = status.map(|status| status.to_string()).unwrap_or_default();
            Finish::Failed(format!("the repair ended without finishing its turn ({exited}): {complaint}"))
        };
        RepairEnded { said, finish }
    }
}

/// A repair's process, started and not yet read.
struct Started {
    child: Child,
    pid: Option<NonZeroU32>,
}

/// One repair running, as its stop reaches it.
#[derive(Default)]
pub(crate) struct Running {
    stop: Notify,
    stopping: AtomicBool,
}

impl Running {
    /// Ask it to stop. **The reading task sends the signal**, since it holds the uncollected child.
    fn stop(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.stop.notify_one();
    }

    fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }
}

/// Every repair running, by the Job it repairs, and the host that starts them. **One a Job.**
pub struct Repairs {
    host: Arc<RepairHost>,
    running: Mutex<BTreeMap<String, Arc<Running>>>,
}

impl Repairs {
    pub fn hosted_by(host: Arc<RepairHost>) -> Repairs {
        Repairs { host, running: Mutex::new(BTreeMap::new()) }
    }

    pub(crate) fn host(&self) -> Arc<RepairHost> {
        Arc::clone(&self.host)
    }

    /// Listed as running for `job`, until [`Repairs::ended`]. `None` where one already is.
    pub(crate) fn listed(&self, job: &str) -> Option<Arc<Running>> {
        let mut running = self.running.lock().expect("the repairs are not held across a panic");
        if running.contains_key(job) {
            return None;
        }
        let one = Arc::new(Running::default());
        running.insert(job.to_string(), Arc::clone(&one));
        Some(one)
    }

    pub(crate) fn ended(&self, job: &str) {
        self.running.lock().expect("the repairs are not held across a panic").remove(job);
    }

    /// Whether a repair is running for `job`.
    #[cfg(test)]
    pub(crate) fn running_for(&self, job: &str) -> bool {
        self.running.lock().expect("the repairs are not held across a panic").contains_key(job)
    }

    /// Whether any repair is running: Fleet starts one at a time for a dead Job.
    pub(crate) fn running_any(&self) -> bool {
        !self.running.lock().expect("the repairs are not held across a panic").is_empty()
    }

    /// Stop the repair running for `job`. `false` where none is.
    pub(crate) fn stop(&self, job: &str) -> bool {
        let one = self.running.lock().expect("the repairs are not held across a panic").get(job).cloned();
        match one {
            Some(one) => {
                one.stop();
                true
            }
            None => false,
        }
    }
}
