//! What a batch of Checks knows about each run as it ends, handed to the
//! writer to be appended to `check_runs` later. Only reads and notes: nothing
//! here can fail or delay a Check.

use std::time::Duration;

use checks_runner::CheckWidth;
use core_model::ResolvedCheck;
use store::{CheckOutcome, CheckRun};
use tokio::time::Instant;
use verification::{Exit, Observed};

use crate::underway::Announcing;

/// One batch's readings: the load and cores once at its start, and for each
/// Check when it asked for a place, how long it waited and when it began.
pub(crate) struct Ledger {
    load: Option<f64>,
    cores: u32,
    asked: Option<Instant>,
    waited: Vec<Duration>,
    began: Vec<Option<core_model::Timestamp>>,
}

impl Ledger {
    pub(crate) fn begin(checks: usize) -> Ledger {
        Ledger {
            load: one_minute_load(),
            cores: std::thread::available_parallelism()
                .map_or(1, |cores| u32::try_from(cores.get()).unwrap_or(u32::MAX)),
            asked: None,
            waited: vec![Duration::ZERO; checks],
            began: vec![None; checks],
        }
    }

    /// The batch asks for a place for its next Check.
    pub(crate) fn asking(&mut self) {
        self.asked = Some(Instant::now());
    }

    /// The Check at `at` was given its place and is about to start.
    pub(crate) fn granted(&mut self, at: usize, announcing: &Announcing) {
        self.waited[at] = self
            .asked
            .take()
            .map_or(Duration::ZERO, |asked| asked.elapsed());
        self.began[at] = announcing.stamp();
    }

    /// The Check at `at` ended with `observed`. Only an exit code or a timeout
    /// is a run; a signal or a command that never started is not.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ran(
        &self,
        at: usize,
        check: &ResolvedCheck,
        observed: &Observed,
        narrowed_to: Option<&str>,
        took: Duration,
        width: CheckWidth,
        announcing: &Announcing,
    ) {
        let outcome = match observed {
            Observed::Command(Exit::TimedOut { .. }) => CheckOutcome::TimedOut,
            Observed::Command(Exit::Code(_)) if crate::checking::advances(check, observed) => {
                CheckOutcome::Passed
            }
            Observed::Command(Exit::Code(_)) => CheckOutcome::Failed,
            _ => return,
        };
        let Some(started_at) = self.began[at].clone() else {
            return;
        };
        announcing.ran(CheckRun {
            check: check.key().to_string(),
            started_at,
            outcome,
            narrowed_to: narrowed_to.map(str::to_string),
            queue_wait: self.waited[at],
            took,
            // The runner's summary is not read where a Check ends.
            tests_run: None,
            tests_failed: None,
            load_avg_at_start: self.load,
            cores: self.cores,
            width: width.get(),
            places_held: check.places().get(),
        });
    }
}

/// The 1-minute load average, read once per batch. `None` where the machine will not say.
///
/// **A `sysctl` process, not a libc call**: the call needs the gate's list of files that may
/// speak the unsafe keyword, which is the owner's to extend, not for a number read once.
fn one_minute_load() -> Option<f64> {
    let said = std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()?;
    // `{ 18.19 12.71 13.90 }`
    String::from_utf8(said.stdout)
        .ok()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
