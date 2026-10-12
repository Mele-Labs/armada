//! Why a Drone's process is gone when it never said its run was over.
//!
//! `Ending::Vanished` escalates a Job as `interrupted` and says nothing else:
//! it is a fold over a transcript with no terminating row, and a process that
//! was killed from outside, one that crashed and one whose pipe was cut all
//! fold to it. The operating system does say which, once, to whoever collects
//! the child, and that is what the Job's log gets here.

use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use core_model::{Component, DroneId, Envelope, FieldValue, JobId, JobStatus, Level, StepId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drone::{Ending, Left};
use crate::working::Working;

/// How the process ended, in words a person can act on.
pub(crate) fn said(ended: Option<ExitStatus>) -> String {
    let Some(status) = ended else {
        return String::from("ended, and the operating system did not say how");
    };
    if let Some(signal) = status.signal() {
        let name = match signal {
            libc::SIGKILL => " (SIGKILL: stopped from outside, or killed for memory)",
            libc::SIGTERM => " (SIGTERM)",
            libc::SIGINT => " (SIGINT)",
            libc::SIGHUP => " (SIGHUP)",
            libc::SIGPIPE => " (SIGPIPE: nothing was reading its output)",
            libc::SIGABRT => " (SIGABRT: it aborted)",
            libc::SIGSEGV => " (SIGSEGV: it crashed)",
            libc::SIGBUS => " (SIGBUS: it crashed)",
            _ => "",
        };
        return format!("killed by signal {signal}{name}");
    }
    match status.code() {
        Some(code) => format!("exited with code {code}"),
        None => String::from("ended, and the operating system did not say how"),
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
    /// Write into the Job's log that a Drone's process ended on its own with
    /// no terminating row, and how. **Before the Job moves**, so the line is
    /// there for whoever reads the escalation it explains.
    ///
    /// Fleet did not signal this one: every road that does says so on its own
    /// line (`the Drone was ended because its step ended`).
    pub(crate) fn noted_vanished(
        &self,
        job: &JobId,
        step: &StepId,
        drone: &DroneId,
        ended: Option<ExitStatus>,
        heard: &[DroneEvent],
        healed: bool,
    ) {
        let calls = heard
            .iter()
            .filter(|event| matches!(event, DroneEvent::Called { .. }))
            .count();
        let last = heard.iter().rev().find_map(|event| match event {
            DroneEvent::Called { tool, .. } => Some(tool.clone()),
            _ => None,
        });
        let envelope = Envelope::new(
            self.now(),
            if healed { Level::Info } else { Level::Warn },
            Component::Fleet,
            self.run().clone(),
            "the Drone's process ended on its own before it reported, and Fleet did not signal it",
        )
        .in_job(job.as_ulid().clone())
        .by_drone(drone.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("ended", FieldValue::Str(said(ended)))
        .with_field("calls", FieldValue::Int(calls as i64))
        .with_field("last_call", FieldValue::Str(last.unwrap_or_default()))
        .with_field("healed", FieldValue::Bool(healed));
        self.noted_in_the_log(job, &envelope);
    }

    /// Put a fresh Drone on the task of a kept Drone whose process vanished,
    /// **once per task**. `true` when it did, and the slot then holds the new
    /// Drone; `false` leaves the slot for `reap` to escalate as `interrupted`.
    ///
    /// Only a task's Drone that has not handed in, on a step Fleet is running
    /// and with nothing at the gate: a Drone that handed in is `tasking`'s to
    /// replace, and one on a stopped Job is a person's. A Drone Fleet adopted
    /// is left too, because nothing says how it went.
    ///
    /// The task goes back to `open` the way a side Drone's does
    /// (`crate::crew`), and the next-task spawn then takes it, from the same
    /// baseline and declared paths as any task's successor.
    pub(crate) async fn healed_task_drone(
        &self,
        working: &mut Option<Working>,
    ) -> Result<bool, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(false);
        };
        let Some(task) = at_work.task() else {
            return Ok(false);
        };
        let heard = at_work.heard();
        if at_work.has_handed_in()
            || at_work.session().unheard()
            || !matches!(Ending::of(&heard), Ending::Vanished)
        {
            return Ok(false);
        }
        let (job_id, step, drone) = at_work.drone();
        let ended = at_work.how_it_ended();
        if self.load(&job_id).await?.status() != JobStatus::Running
            || self.left(&job_id) != Left::Nothing
            || !self.first_vanish_of(&job_id, task)
        {
            return Ok(false);
        }
        self.noted_vanished(&job_id, &step, &drone, ended, &heard, true);
        self.reopened(&job_id, &step, task).await?;
        self.put_next_task_drone(working).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signal_is_named_and_a_code_is_a_code() {
        assert_eq!(
            said(Some(ExitStatus::from_raw(libc::SIGKILL))),
            "killed by signal 9 (SIGKILL: stopped from outside, or killed for memory)"
        );
        assert_eq!(said(Some(ExitStatus::from_raw(1 << 8))), "exited with code 1");
        assert!(said(None).contains("did not say how"));
    }
}
