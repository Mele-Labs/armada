//! A Check that fails while others still run is put to the base at once, so
//! `--status` can say so while the turn goes on. `docs/capabilities/merge-line.md`,
//! *A failed Check is told at once*.

use std::path::Path;

use super::batch::tell;
use super::caches::checks_on_the_base;
use super::dir::StateDir;
use super::env::Env;
use super::outcome::{CheckRun, OutcomePatch, OutcomeState};
use super::queue::QueueEntry;
use super::stop::Stopped;

/// What the base said of each Check asked about mid-turn, so the end of the
/// turn asks it only of the rest. A timeout is kept here though the cache
/// does not keep it: it is the base's for this turn.
#[derive(Default)]
pub struct Asked {
    pub names: Vec<String>,
    pub already: Vec<String>,
    pub timed_out: Vec<String>,
    /// Failed here and green on the base: the branch's.
    pub own: Vec<String>,
}

impl Asked {
    /// Ask the base about `name`, which just failed, and say what it answered.
    #[allow(clippy::too_many_arguments)]
    pub fn ask(
        &mut self,
        repo: &Path,
        state: &StateDir,
        env: &Env,
        group: &[QueueEntry],
        base: &str,
        logs: &Path,
        name: &str,
        runs: &[CheckRun],
        log_paths: &[String],
    ) -> Result<(), Stopped> {
        let patch = |own: &[String], already: Option<Vec<String>>| OutcomePatch {
            logs: Some(log_paths.to_vec()),
            checks: Some(runs.to_vec()),
            own_failures: Some(own.to_vec()),
            already,
            ..OutcomePatch::default()
        };
        tell(
            state,
            group,
            OutcomeState::Gating,
            format!("{name} failed; asking whether {} fails it too", env.base),
            patch(&self.own, None),
        )?;
        let on_base = checks_on_the_base(repo, state, base, &[name.to_string()], env, logs)?;
        self.names.push(name.to_string());
        let detail = if on_base.already.is_empty() {
            self.own.push(name.to_string());
            format!("{name} failed and {} is green for it", env.base)
        } else {
            self.already.extend(on_base.already);
            self.timed_out.extend(on_base.timed_out);
            format!("{name} failed and fails on {} too", env.base)
        };
        tell(
            state,
            group,
            OutcomeState::Gating,
            detail,
            patch(&self.own, Some(self.already.clone())),
        )
    }
}
