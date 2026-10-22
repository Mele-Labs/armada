//! What a watch tells its Job, once per change, and its end with the Job.
//!
//! **Once per (watch, state, failing parts)**, kept in the store as `pull_notices` keeps a pull
//! request's, so a restart repeats nothing. `waiting` and `running` tell nothing; a weighing tells
//! nobody, since the agent gave it. A Job that cannot take a notice now is offered it again a
//! minute later, for as long as the watch owes it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{ReadingState, WatchState};
use store::{KeptWatchTaken, KeptWatchWhole, Store};

use super::{domain, moved};
use crate::daemon::Fleet;

/// The most of a broken command's output a notice quotes, from its end.
const OUTPUT_TOLD: usize = 1_200;
/// The most lines of it.
const OUTPUT_LINES: usize = 15;

/// The counted parts of `watch`'s last reading that failed, in the order the reading named them.
fn failing(watch: &core_model::Watch) -> Vec<String> {
    let Some(last) = &watch.last else { return Vec::new() };
    last.parts
        .iter()
        .filter(|part| part.state == ReadingState::Failed)
        .filter(|part| !watch.set_aside.iter().any(|aside| aside.part == part.name))
        .map(|part| part.name.clone())
        .collect()
}

/// The notice `watch` owes its Job in the state it is in, as the key a store row keeps once.
/// `None` for `waiting`, `running`, a pass not yet finished, and `stopped`.
pub(crate) fn owed(watch: &core_model::Watch) -> Option<String> {
    match watch.state {
        WatchState::Failed => {
            let mut parts = failing(watch);
            parts.sort();
            Some(format!("failed:{}", parts.join(",")))
        }
        WatchState::Passed if !watch.live => Some(String::from("passed")),
        WatchState::GaveUp => Some(String::from("gave_up")),
        WatchState::Broken => Some(String::from("broken")),
        _ => None,
    }
}

/// `owed`, less what was already told of it.
pub(crate) fn unsaid(store: &Store, id: &str, watch: &core_model::Watch) -> Option<String> {
    owed(watch).filter(|key| !store.watch_notice_kept(id, key).unwrap_or(true))
}

/// A span as a recipe writes it: `2h`, `90m`, `45s`.
fn span(seconds: u32) -> String {
    match seconds {
        s if s % 3_600 == 0 => format!("{}h", s / 3_600),
        s if s % 60 == 0 => format!("{}m", s / 60),
        s => format!("{s}s"),
    }
}

/// The end of `output`: its last lines, within [`OUTPUT_TOLD`] characters.
fn trimmed(output: &str) -> String {
    let lines: Vec<&str> = output.trim().lines().collect();
    let kept = lines[lines.len().saturating_sub(OUTPUT_LINES)..].join("\n");
    let chars: Vec<char> = kept.chars().collect();
    chars[chars.len().saturating_sub(OUTPUT_TOLD)..].iter().collect()
}

/// What the Job is told of `kept` in the state `watch` is in. `None` where it owes nothing.
pub(crate) fn notice(kept: &KeptWatchWhole, watch: &core_model::Watch) -> Option<String> {
    let name = &kept.watch.name;
    let said = match watch.state {
        WatchState::Failed => {
            let last = watch.last.as_ref()?;
            let failing = failing(watch);
            let mut said = match failing.is_empty() {
                true => format!("{name} failed: {}", last.says.trim()),
                false => format!("{name} failed: {} failed.", failing.join(", ")),
            };
            if let Some(link) = &last.link {
                said.push(' ');
                said.push_str(link);
            }
            said.push_str(&format!(
                "\nWeigh it with weigh_watch on watch {}: ours ends it, and not_ours sets those parts aside and it carries on.",
                kept.watch.id
            ));
            said
        }
        WatchState::Passed if !watch.live => format!("{name} passed."),
        WatchState::GaveUp => match &watch.last {
            Some(last) => format!(
                "{name} gave up after {}. It last said: {}",
                span(kept.watch.gives_up_after_seconds),
                last.says.trim()
            ),
            None => format!("{name} gave up after {} without a reading.", span(kept.watch.gives_up_after_seconds)),
        },
        WatchState::Broken => {
            let last = kept.readings.iter().rev().find_map(|one| match &one.taken {
                KeptWatchTaken::Broken { why, output } => {
                    Some(output.as_deref().filter(|out| !out.trim().is_empty()).map_or_else(|| why.clone(), trimmed))
                }
                KeptWatchTaken::Read { .. } => None,
            });
            format!("{name} stopped: its command failed three times. {}", last.unwrap_or_default()).trim_end().to_string()
        }
        _ => return None,
    };
    Some(said)
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
    /// Offer every notice a watch still owes, each a minute apart at most.
    pub(crate) async fn watches_told_when_due(self: &Arc<Self>) {
        let Ok(unsaid) = self.store().lock().await.unsaid_watches() else { return };
        for kept in unsaid {
            self.watch_told(&kept.watch.id).await;
        }
    }

    /// Tell `id`'s Job what it owes, unless it was told or was offered it within the minute.
    pub(crate) async fn watch_told(self: &Arc<Self>, id: &str) {
        let Ok(Some(kept)) = self.store().lock().await.watch_whole(id) else { return };
        let Some(key) = kept.watch.unsaid.clone() else { return };
        if !self.watches().may_offer(&format!("{id}/{key}"), &self.now()) {
            return;
        }
        let watch = domain(&kept);
        let text = notice(&kept, &watch).filter(|_| super::owed(&watch).as_deref() == Some(key.as_str()));
        let told = match &text {
            Some(text) => matches!(self.told_by_fleet(&kept.watch.session_id, text).await, Ok(true)),
            // What it owed is no longer what it is, so there is nothing true to tell.
            None => true,
        };
        if !told {
            return;
        }
        let now = self.now();
        let mut store = self.store().lock().await;
        if text.is_some() {
            let _ = store.keep_watch_notice(id, &key, now.as_str());
        }
        if let Ok(Some(fresh)) = store.watch_whole(id) {
            let mut next = fresh.watch;
            if next.unsaid.as_deref() == Some(key.as_str()) {
                next.unsaid = None;
                let _ = store.keep_watch(&next);
            }
        }
    }

    /// End every live watch of `job` `stopped`, and take back what it still owed: the Job is gone,
    /// so there is nobody to wake. `crate::session_host::ending`'s wind up calls it.
    pub(crate) async fn watches_ended_with(&self, job: &str) {
        let now = self.now();
        let ended: Vec<String> = {
            let mut store = self.store().lock().await;
            let Ok(watches) = store.watches_of(job) else { return };
            let mut ended = Vec::new();
            for kept in watches.iter().filter(|one| one.watch.live || one.watch.unsaid.is_some()) {
                let mut watch = domain(kept);
                watch.end(WatchState::Stopped);
                let mut next = moved(&kept.watch, &watch, &now);
                next.unsaid = None;
                if store.keep_watch(&next).is_ok() {
                    ended.push(kept.watch.id.clone());
                }
            }
            ended
        };
        if ended.is_empty() {
            return;
        }
        for id in &ended {
            self.watches().stop(id);
        }
        self.watches_published(job).await;
        self.watching_row_published(job).await;
    }
}
