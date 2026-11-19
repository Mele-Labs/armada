//! A Job's tokens from its start to its end: what the Tokens tab draws full size and the Context card draws small. `model.rs` works
//! out the graph from Fleet's `JobTimeline` and touches no screen; `chart.rs` paints it; `read.rs` is the one read of a Job's timeline
//! that the card and the tab share; `mock.rs` stands in for Fleet in a walk.
//!
//! Fleet works out what is shown (the buckets, the pauses and their words, the figures); this draws it and measures where a label fits.

mod chart;
mod mock;
mod model;
mod read;
pub mod walk;
pub mod wire;

#[cfg(test)]
mod tests;

pub use chart::{bones, chart, margins};
pub use model::Graph;
pub use read::{Held, TimelineRead};

use std::rc::Rc;

use gpui::{App, Entity, Global, Window};

/// The Job whose Tokens tab a press left to open. The Job view takes it when it draws that Job and puts the tab in front.
#[derive(Default)]
struct Pending(Option<String>);

impl Global for Pending {}

/// Leave the Job whose tab should open, in place of any that nobody took.
pub fn put(job: &str, cx: &mut App) {
    cx.default_global::<Pending>().0 = Some(job.to_string());
}

/// Whether the ask left is for this Job.
pub fn asks(job: &str, cx: &App) -> bool {
    cx.try_global::<Pending>().is_some_and(|pending| pending.0.as_deref() == Some(job))
}

/// Take the ask for this Job, which leaves none.
pub fn take(job: &str, cx: &mut App) -> bool {
    let held = cx.default_global::<Pending>();
    if held.0.as_deref() == Some(job) {
        held.0 = None;
        true
    } else {
        false
    }
}

/// Open a Job's Tokens tab from anywhere: the ask is left, then the Job opens as any screen opens one.
pub fn open(job: &str, cx: &mut App) {
    put(job, cx);
    if let Some(sink) = cx.try_global::<crate::titlebar::AskedSink>().cloned() {
        (sink.0)(&crate::titlebar::Asked::OpenJob(job.to_string()), cx);
    }
}

/// What a Job's head hands the Context card for the graph under its rows: the Job's reading, and what a press on Open Tokens tab does.
#[derive(Clone)]
pub struct Hook {
    pub read: Entity<TimelineRead>,
    pub open: Rc<dyn Fn(&mut Window, &mut App)>,
}

/// The mock Job's timeline and an empty one, for the tab's tests.
#[cfg(test)]
pub fn mock_timeline() -> ipc::JobTimeline {
    mock::timeline()
}

#[cfg(test)]
pub fn mock_empty() -> ipc::JobTimeline {
    mock::empty()
}
