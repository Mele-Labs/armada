//! The Commands a repository's `when_base_moves:` names, run in its own checkout each time that
//! checkout stands at a new commit of the base. `docs/concepts/manifest.md`, *When the base moves*.
//!
//! **Off the reading of main**, beside the catch-up that moves the checkout, so a merge by
//! anyone is seen and not only one of Fleet's own. **Only where the checkout is at the tip**:
//! a checkout held on other work runs nothing, and the next reading tries again.
//!
//! **One run at a time per repository.** A move seen during a run queues one more, and every
//! later move before it starts is folded into that one, since the run reads the checkout as it
//! stands and not the commit that asked for it. It waits for a place behind every Check and
//! runs at utility priority. A failure is said on Fleet's console and ends the run; no Job
//! waits on it.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Priority, Writing};
use core_model::{FieldValue, Level};
use verification::Exit;

use crate::daemon::Fleet;
use crate::places::Asking;
use crate::repositories::Served;

/// How much of a failed Command's output the console line carries: its last lines.
const SAID_TAIL: usize = 2000;

/// Each repository's runs, by Manifest id. Never written down: a Fleet that starts runs once at
/// the first tip it sees, which is what a move made while it was down needs.
#[derive(Default)]
pub struct BaseMoves(Mutex<BTreeMap<String, Following>>);

#[derive(Default)]
struct Following {
    /// The last tip a run was started or queued for.
    seen: Option<String>,
    running: bool,
    /// A move came while a run was going.
    again: bool,
    /// A run is owed and not yet started.
    due: bool,
}

impl BaseMoves {
    /// The checkout of `repository` stands at `tip`. Starts a run where none is going and queues
    /// one where one is; a tip already seen does neither.
    pub fn moved(&self, repository: &str, tip: &str) {
        let mut all = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let one = all.entry(repository.to_string()).or_default();
        if one.seen.as_deref() == Some(tip) {
            return;
        }
        one.seen = Some(tip.to_string());
        match one.running {
            true => one.again = true,
            false => {
                one.running = true;
                one.due = true;
            }
        }
    }

    /// Whether `tip` is the last tip `repository` ran or queued for, so a reading of an unmoved
    /// base asks nothing of git.
    pub fn seen(&self, repository: &str, tip: &str) -> bool {
        let all = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        all.get(repository).is_some_and(|one| one.seen.as_deref() == Some(tip))
    }

    /// The repositories whose run is owed, each handed out once.
    pub fn due(&self) -> Vec<String> {
        let mut all = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        all.iter_mut()
            .filter_map(|(repository, one)| std::mem::take(&mut one.due).then(|| repository.clone()))
            .collect()
    }

    /// A run in `repository` ended. Whether to run once more, for moves seen while it went.
    pub fn ended(&self, repository: &str) -> bool {
        let mut all = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(one) = all.get_mut(repository) else {
            return false;
        };
        let again = std::mem::take(&mut one.again);
        one.running = again;
        again
    }
}

impl Fleet
{
    /// Main's tip on the forge is `head`, read after the catch-up. Noted where the repository
    /// names something to run and its checkout's `HEAD` is that commit.
    pub async fn base_move_seen(&self, served: &Served, head: &str) {
        let repository = served.manifest().id().as_str().to_string();
        if served.manifest().when_base_moves().is_empty() || self.base_moves().seen(&repository, head) {
            return;
        }
        let root = served.root().to_string();
        let at = tokio::task::spawn_blocking(move || adapters::territory::head_of(Path::new(&root), "HEAD").ok())
            .await
            .ok()
            .flatten();
        if at.as_deref() == Some(head) {
            self.base_moves().moved(&repository, head);
        }
    }

    /// Start each run owed, on a task of its own, so the reading of main goes on.
    pub fn base_moves_started(self: &Arc<Self>) {
        for repository in self.base_moves().due() {
            let fleet = Arc::clone(self);
            tokio::spawn(async move {
                loop {
                    fleet.base_move_ran(&repository).await;
                    if !fleet.base_moves().ended(&repository) {
                        break;
                    }
                }
            });
        }
    }

    /// One run: every Command named, in order, in the checkout, stopping at the first that fails.
    async fn base_move_ran(&self, repository: &str) {
        let Some(served) = self.repositories().serving(repository) else { return };
        let _place = self.room_asking(Asking::BaseMoved).place().await;
        let id = served.manifest().id().clone();
        for step in served.manifest().when_base_moves() {
            let attempt = checks_runner::run_until(
                step.run(),
                Path::new(served.root()),
                self.budget().duration(),
                Writing::Nowhere,
                &[],
                std::future::pending(),
                Priority::Low,
            )
            .await;
            if attempt.exit == Exit::Code(0) {
                continue;
            }
            let printed = format!("{}{}", attempt.output.stdout, attempt.output.stderr);
            self.said(
                Level::Warn,
                "a Command the base's move runs failed",
                &id,
                [
                    ("command", FieldValue::Str(step.name().to_string())),
                    ("ended", FieldValue::Str(format!("{:?}", attempt.exit))),
                    ("said", FieldValue::Str(tail(&printed).to_string())),
                ],
            );
            return;
        }
    }
}

/// The last [`SAID_TAIL`] bytes of `text`, from a character boundary.
fn tail(text: &str) -> &str {
    let mut from = text.len().saturating_sub(SAID_TAIL);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    &text[from..]
}
