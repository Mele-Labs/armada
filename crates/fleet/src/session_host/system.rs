//! What a Job costs the machine, and ending one process it started: `get_session_system` and
//! `stop_process`. **Ownership is [`Fleet::started_by`]'s**, through [`Fleet::processes_of`], so a pid
//! the Job did not start is never signalled, and one it started before its agent was replaced still is.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestId, ProcessStopped, SessionId, SessionSystem, StopProcess, SystemLog, SystemProcess, SystemProcessState,
    SystemWorktree,
};

use super::processes::GRACE;
use crate::daemon::Fleet;

const PROCESS_UNNAMED: &str = "fleet.process_unnamed";
const PROCESS_NOT_THE_JOBS: &str = "fleet.process_not_the_jobs";
const SYSTEM_READ_FAILED: &str = "fleet.system_read_failed";

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_session_system`: the Job's processes and what they and its slot take, its worktrees and logs.
    /// Refused for a Job Fleet does not host. The readings run on a blocking thread.
    pub(crate) async fn system_of(self: Arc<Self>, session_id: SessionId) -> Result<SessionSystem, Refusal> {
        let id = session_id.as_str().trim().to_string();
        let (_, hosting) = self.session_and_hosting(&id).await?;
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let fleet = Arc::clone(&self);
        tokio::task::spawn_blocking(move || {
            let held = fleet.process_directories(&id, &served, &hosting);
            let processes = fleet.processes_of(&id, &held);
            let slot = Self::directory_of(&served, &hosting);
            let load = fleet.load_of(&processes, &slot);
            let (memory_of_bytes, disk_of_bytes) = Self::machine_of(&slot);
            let pool = crate::leasing::pool_of(&served);
            let worktrees = fleet
                .vcs()
                .slot_pool(&pool)
                .into_iter()
                .filter(|reading| held.contains(&reading.path))
                .map(|reading| SystemWorktree { slot: reading.slot, path: reading.path, branch: reading.branch })
                .collect();
            let logs = fleet.logs_of(&id);
            SessionSystem {
                session_id: SessionId::carried(id),
                processes: processes
                    .into_iter()
                    .map(|one| SystemProcess {
                        pid: one.pid,
                        command: one.command,
                        port: one.port,
                        state: SystemProcessState::Running,
                    })
                    .collect(),
                cpu_percent: load.cpu_percent,
                memory_bytes: load.memory_bytes,
                memory_of_bytes,
                disk_bytes: load.disk_bytes,
                disk_of_bytes,
                worktrees,
                logs,
            }
        })
        .await
        .map_err(|_| self.hosted_fault(SYSTEM_READ_FAILED, "the readings of this job's processes did not finish"))
    }

    /// The files Fleet keeps for the Job: the keeper's log of the agent's error output, and the agent's own
    /// transcript. A subagent's transcript is left out, and so is a file that is not there.
    fn logs_of(&self, id: &str) -> Vec<SystemLog> {
        let keeper = std::path::Path::new(&self.host().keepers_dir).join(format!("{id}.log"));
        let kept = std::fs::metadata(&keeper).ok().map(|file| SystemLog { name: String::from("session.log"), path: keeper.to_string_lossy().into_owned(), bytes: file.len() });
        let transcripts = adapters::turn_lines::transcripts_of(&self.host().home, id)
            .into_iter()
            .filter(|file| !file.subagent)
            .filter_map(|file| {
                let name = file.path.file_name()?.to_string_lossy().into_owned();
                Some(SystemLog { name, path: file.path.to_string_lossy().into_owned(), bytes: file.size })
            });
        kept.into_iter().chain(transcripts).collect()
    }

    /// `stop_process`: end one process the Job started. **Refused unless the Job started it**, which is
    /// [`Fleet::started_by`]'s word, and a port is resolved to a pid from the same listing. The ending
    /// runs on a blocking thread and is not waited for.
    pub(crate) async fn process_stopped(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        stop: StopProcess,
    ) -> Result<ProcessStopped, Refusal> {
        let id = self.waiting_session(caller, stop.session_id.as_ref())?;
        if stop.pid.is_some() == stop.port.is_some() {
            return Err(self.hosted_refusal(PROCESS_UNNAMED, "pid or port, one of them, to say which process to end"));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let fleet = Arc::clone(&self);
        let found = tokio::task::spawn_blocking(move || {
            let held = fleet.process_directories(&id, &served, &hosting);
            fleet
                .processes_of(&id, &held)
                .into_iter()
                .find(|one| stop.pid == Some(one.pid) || (stop.port.is_some() && stop.port == one.port))
        })
        .await
        .map_err(|_| self.hosted_fault(SYSTEM_READ_FAILED, "the readings of this job's processes did not finish"))?;
        let Some(process) = found else {
            let named = match (stop.pid, stop.port) {
                (Some(pid), _) => format!("pid {pid}"),
                (None, Some(port)) => format!("port {port}"),
                (None, None) => String::new(),
            };
            return Err(self.hosted_refusal(PROCESS_NOT_THE_JOBS, &format!("{named} is not a process this job started, so it is left running")));
        };
        let pid = process.pid;
        tokio::task::spawn_blocking(move || adapters::leasing::end_pids(&[pid], GRACE));
        Ok(ProcessStopped { pid, command: process.command })
    }
}
