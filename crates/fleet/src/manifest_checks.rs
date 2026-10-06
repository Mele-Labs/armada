//! Every Check run one repository's Jobs asked for or ran, read in one answer.
//! `ipc::ManifestChecks`.
//!
//! **A read, never a record**: the gate's rows (`job_step_checks`) and a Drone's
//! asked runs (`asked_runs`) stay where they are, and this folds them into rows a
//! Checks page draws. Merge-line Checks and checkout runs are not here.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Job, StepCheck};
use ipc::{ManifestCheckLog, ManifestCheckRow, ManifestChecks};
use store::{AskedRun, Attempted};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A log's own file name, which is what `get_check_output` takes.
fn kept(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// One Job's rows: each Check a gate ruled on, and each run a Drone asked for.
pub(crate) fn rows_of(
    job: &Job,
    ran: &[Attempted<Vec<StepCheck>>],
    asked: &[AskedRun],
) -> Vec<ManifestCheckRow> {
    let (job_id, handle) = (ipc::JobId::from(job.id()), job.handle());
    let mut rows = Vec::new();
    for group in ran {
        let step = ipc::StepId::from(&group.step_id);
        for check in &group.record {
            rows.push(ManifestCheckRow {
                source: ipc::SOURCE_GATE.to_string(),
                requester: ipc::Requester::gate(&job_id, &step).with_handle(&handle),
                job_id: job_id.clone(),
                job_handle: handle.clone(),
                job_title: job.title().as_str().to_string(),
                step: step.clone(),
                attempt: group.attempt.number(),
                group: group.group.map(|(group, _)| group.to_string()),
                name: check.name.clone(),
                state: ipc::CheckOutcome::from(check.outcome).as_wire().to_string(),
                started_at: None,
                ended_at: Some(ipc::Instant::from(&group.at)),
                took_ms: None,
                logs: check
                    .output_path
                    .iter()
                    .map(|path| ManifestCheckLog {
                        check: check.name.clone(),
                        kept: kept(path),
                    })
                    .collect(),
                asked_run_id: None,
            });
        }
    }
    for run in asked {
        let wired = crate::dry_run::asked::wired(job, run);
        let took_ms = run
            .finished_at
            .as_ref()
            .map(|ended| crate::converging::elapsed(&run.started_at, ended).as_millis() as u64);
        rows.push(ManifestCheckRow {
            source: ipc::SOURCE_ASKED_RUN.to_string(),
            requester: wired.requester,
            job_id: job_id.clone(),
            job_handle: handle.clone(),
            job_title: job.title().as_str().to_string(),
            step: ipc::StepId::from(&run.step),
            attempt: run.attempt,
            group: None,
            name: run.checks.join(", "),
            state: asked_state(run.state).to_string(),
            started_at: Some(ipc::Instant::from(&run.started_at)),
            ended_at: wired.finished_at,
            took_ms,
            logs: logs_of(run),
            asked_run_id: Some(run.id),
        });
    }
    rows
}

fn asked_state(state: store::AskedState) -> &'static str {
    state.as_str()
}

/// An asked run's logs, each with the Check it belongs to. Stored beside
/// `checks`, one per entry and empty where a Check kept none.
pub(crate) fn logs_of(run: &AskedRun) -> Vec<ManifestCheckLog> {
    run.checks
        .iter()
        .zip(&run.logs)
        .filter(|(_, path)| !path.is_empty())
        .map(|(check, path)| ManifestCheckLog {
            check: check.clone(),
            kept: kept(path),
        })
        .collect()
}

/// The newest `most` rows, newest first, and how many there were.
pub(crate) fn newest(mut rows: Vec<ManifestCheckRow>, most: usize) -> ManifestChecks {
    let at = |row: &ManifestCheckRow| {
        row.ended_at
            .as_ref()
            .or(row.started_at.as_ref())
            .map(|at| at.as_str().to_string())
            .unwrap_or_default()
    };
    rows.sort_by(|a, b| at(b).cmp(&at(a)));
    let total = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    rows.truncate(most);
    ManifestChecks {
        truncated: usize::try_from(total).unwrap_or(usize::MAX) > rows.len(),
        rows,
        total,
    }
}

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
    /// `list_manifest_checks`: every Check run this repository's Jobs asked for
    /// or ran, newest first, cut at [`ManifestChecks::MOST`]. **Absent a name,
    /// every served repository's**, as `list_jobs` is.
    pub(crate) async fn manifest_checks(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestChecks, Refusal> {
        let owned = self.owned_by(manifest_id.as_ref())?;
        let (mut loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        loaded.jobs.retain(|job| owned(job));
        let mut rows = Vec::new();
        for job in &loaded.jobs {
            let store = self.store().lock().await;
            let ran = store
                .step_checks_every_attempt(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let asked = store
                .asked_runs(job.id(), self.run())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            drop(store);
            rows.extend(rows_of(job, &ran, &asked));
        }
        Ok(newest(rows, ManifestChecks::MOST))
    }
}
