//! What the stuck band's three acts ask: **Queue again** puts each pull request back in the merge queue,
//! **Tell Jobs** sends the Job that holds one the facts, and **Repair Job** opens the dispatch
//! sheet drafted from them. The strip emits; the wiring in `live.rs` does it.

use gpui::{Context, SharedString};
use ipc::{PullView, QueueStuck, QueueStuckCause};

use super::{Asked, MergeStrip};

const LIMITS: &str = "Fleet put it back as often as Settings › Limits allows.";

/// What stopped the queue, as the band says it, for a sentence of its own.
fn cause_said(stuck: &QueueStuck) -> String {
    match (&stuck.cause, stuck.job.as_deref()) {
        (QueueStuckCause::NoRunner, Some(job)) => format!("{job} got no runner"),
        (QueueStuckCause::NoRunner, None) => "a job got no runner".into(),
        (QueueStuckCause::TimedOut, Some(job)) => format!("{job} ran past its time limit"),
        (QueueStuckCause::TimedOut, None) => "a job ran past its time limit".into(),
        (QueueStuckCause::PastEstimate, _) => "waiting past twice the forge's estimate".into(),
    }
}

fn since_said(stuck: &QueueStuck) -> Option<String> {
    crate::settings::time::stamp_at(stuck.since.as_str())
}

/// One message for each Job that holds a stuck pull request, in the order of the queue: the facts, and the log
/// where Fleet has its page.
pub(super) fn tells_of(stuck: &QueueStuck, pulls: &[PullView]) -> Vec<(SharedString, String)> {
    let mut held: Vec<(String, Vec<u64>)> = Vec::new();
    for number in &stuck.pulls {
        let Some(job) = pulls.iter().find(|view| view.pull.number == *number).and_then(|view| view.held_by.as_ref()) else { continue };
        match held.iter_mut().find(|(id, _)| *id == job.id) {
            Some((_, numbers)) => numbers.push(*number),
            None => held.push((job.id.clone(), vec![*number])),
        }
    }
    let since = since_said(stuck).map(|at| format!(", since {at}")).unwrap_or_default();
    let log = stuck.log_url.as_ref().map(|url| format!(" Log: {url}")).unwrap_or_default();
    held.into_iter()
        .map(|(id, numbers)| {
            let list = numbers.iter().map(|number| format!("#{number}")).collect::<Vec<_>>().join(" ");
            let (verb, limits) = match numbers.as_slice() {
                [_] => ("is", LIMITS.to_string()),
                _ => ("are", LIMITS.replace("put it back", "put each back")),
            };
            (id.into(), format!("{list} {verb} stuck in the merge queue: {}{since}. {limits}{log}", cause_said(stuck)))
        })
        .collect()
}

/// The brief a repair Job starts from: bare facts, there to be edited.
pub(super) fn brief_of(stuck: &QueueStuck) -> String {
    let since = since_said(stuck).map(|at| format!(" since {at}")).unwrap_or_default();
    let mut lines = vec![format!("The merge queue on main is stuck: {}{since}.", cause_said(stuck))];
    if !stuck.pulls.is_empty() {
        lines.push(format!("Pull requests held: {}.", stuck.pulls.iter().map(|number| format!("#{number}")).collect::<Vec<_>>().join(" ")));
    }
    lines.push(LIMITS.replace("put it back", "put each back"));
    if let Some(url) = &stuck.log_url {
        lines.push(format!("Failed job: {url}."));
    }
    lines.push("Find why, and clear it.".into());
    lines.join(" ")
}

/// Whether any Job holds one of the stuck pull requests.
pub(super) fn held_by_a_job(stuck: &QueueStuck, pulls: &[PullView]) -> bool {
    stuck.pulls.iter().any(|number| pulls.iter().any(|view| view.pull.number == *number && view.held_by.is_some()))
}

impl MergeStrip {
    pub(super) fn stuck_requeued(&mut self, line: usize, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(stuck) = &one.view.queue_stuck else { return };
        cx.emit(Asked::QueueAgain { root: one.view.root.clone(), numbers: stuck.pulls.clone() });
    }

    pub(super) fn stuck_told(&mut self, line: usize, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(stuck) = &one.view.queue_stuck else { return };
        let root = one.view.root.clone();
        for (session, message) in tells_of(stuck, &one.view.pulls) {
            cx.emit(Asked::SendBack { root: root.clone(), session, message });
        }
    }

    pub(super) fn stuck_repair(&mut self, line: usize, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(stuck) = &one.view.queue_stuck else { return };
        cx.emit(Asked::DispatchFix { root: one.view.root.clone(), request: brief_of(stuck) });
    }
}
