//! The lifecycles, rules and autonomy defaults a Job started with, read back from the copy it
//! froze. **A Job keeps the copy it started with**, so a rule or a lifecycle saved afterwards
//! reaches only the Jobs started after it.
//!
//! A Job that froze none, which is every Job started before this existed, has no rules and reads
//! the lifecycles that apply now ([`Fleet::lifecycles_of_job`]).

use std::path::Path;

use core_model::rule::{ActKind, Autonomy, AutonomyDefaults, Rule};
use core_model::Lifecycle;
use store::{PieceKind, Store};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

/// What a Job froze when it started.
#[derive(Clone, Debug, Default)]
pub struct JobCopy {
    /// The four Armada carries, with a file's in place of the one it replaces and any it adds.
    pub lifecycles: Vec<Lifecycle>,
    pub rules: Vec<Rule>,
    pub defaults: AutonomyDefaults,
}

/// The copy `session` froze, or `None` for a Job that froze none. **A piece that no longer reads is
/// left out**, so a rule written for a version of the file this Fleet cannot read plans nothing
/// and the rest of the copy still stands.
pub fn copy_of(store: &Store, session: &str) -> Option<JobCopy> {
    let kept = store.job_copy_of(session).ok().flatten()?;
    let mut lifecycles = Lifecycle::built_in();
    let mut rules = Vec::new();
    for piece in &kept.pieces {
        let path = Path::new(&piece.path);
        match piece.kind {
            PieceKind::Lifecycle => {
                let Ok(read) = config::parse_lifecycle(path, &piece.text) else { continue };
                match lifecycles.iter_mut().find(|one| one.id() == read.id()) {
                    Some(held) => *held = read,
                    None => lifecycles.push(read),
                }
            }
            PieceKind::Rule => rules.extend(config::parse_rule(path, &piece.text).ok()),
        }
    }
    let defaults = kept.autonomy.iter().fold(AutonomyDefaults::new(), |set, (kind, autonomy)| {
        match (ActKind::from_wire(kind), Autonomy::from_wire(autonomy)) {
            (Some(kind), Some(autonomy)) => set.setting(kind, autonomy),
            _ => set,
        }
    });
    Some(JobCopy { lifecycles, rules, defaults })
}

/// The lifecycles `session` froze, without the rules or anything else of the copy: the four Armada
/// carries with a file's in place of the one it replaces. `None` for a Job that froze none.
pub fn frozen_lifecycles(store: &Store, session: &str) -> Option<Vec<Lifecycle>> {
    let kept = store.job_copy_of(session).ok().flatten()?;
    let mut lifecycles = Lifecycle::built_in();
    for piece in kept.pieces.iter().filter(|piece| piece.kind == PieceKind::Lifecycle) {
        let Ok(read) = config::parse_lifecycle(Path::new(&piece.path), &piece.text) else { continue };
        match lifecycles.iter_mut().find(|one| one.id() == read.id()) {
            Some(held) => *held = read,
            None => lifecycles.push(read),
        }
    }
    Some(lifecycles)
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
    /// The lifecycles `session` reads its instances against: the copy it froze, else the ones that
    /// apply to its repository now. **Every reader of a Job's instances asks here**, so a lifecycle
    /// saved after the Job started changes nothing it holds.
    pub(crate) fn lifecycles_of_job(&self, store: &Store, session: &str, manifest: &str) -> Vec<Lifecycle> {
        frozen_lifecycles(store, session).unwrap_or_else(|| self.lifecycles_that_apply(manifest))
    }
}
