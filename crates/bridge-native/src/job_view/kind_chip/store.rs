//! How his messages were sorted and the asks they may join, held for every Job the view has shown. The thread draws from
//! `&self`, so each Job's two answers are read once when the Job comes on screen, off the main thread, and again where its
//! record changes, since `session.story` is not routed to views: `FleetConnection` takes no such event. A change he makes is
//! sent, and both are read again so the chip is what Fleet kept. With no Fleet behind the view, `mock` answers.

use std::collections::HashMap;

use gpui::{App, AppContext as _, Context, Entity, Subscription, Task};
use ipc::{Ask, JobAsks, MessageSorting, MessageSortings, SetMessageKind};

use super::mock;
use crate::fleet::FleetConnection;
use crate::job_view::story::{stamp_of, Stamp};
use crate::job_view::JobView;
use crate::settings::wire::{body, request, Ask as Route, Refused};

/// One Job's sortings, oldest first, and its asks.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Held {
    pub sortings: Vec<MessageSorting>,
    pub asks: Vec<Ask>,
}

struct Store {
    jobs: HashMap<String, Held>,
    stamps: HashMap<String, Stamp>,
    work: HashMap<String, Task<()>>,
    sends: Vec<Task<()>>,
    fleet: Option<Entity<FleetConnection>>,
    watch: Option<Subscription>,
}

struct Kinds(Entity<Store>);

impl gpui::Global for Kinds {}

/// `GET /sessions/message_kinds?session_id=`. `crates/ipc/operations/list_message_kinds.toml`.
fn read_sortings(port: u16, job: &str, cx: &App) -> Task<Result<MessageSortings, Refused>> {
    request(port, Route::Get(format!("/sessions/message_kinds?session_id={}", crate::link::percent_encoded(job))), "MessageSortings", cx)
}

/// `GET /sessions/asks?session_id=`. `crates/ipc/operations/list_job_asks.toml`.
fn read_asks(port: u16, job: &str, cx: &App) -> Task<Result<JobAsks, Refused>> {
    request(port, Route::Get(format!("/sessions/asks?session_id={}", crate::link::percent_encoded(job))), "JobAsks", cx)
}

/// `POST /sessions/message_kind/set`. `crates/ipc/operations/set_message_kind.toml`.
fn post_kind(port: u16, set: &SetMessageKind, cx: &App) -> Task<Result<MessageSorting, Refused>> {
    request(port, Route::Post("/sessions/message_kind/set".into(), body(set)), "MessageSorting", cx)
}

fn store(cx: &mut App) -> Entity<Store> {
    match cx.try_global::<Kinds>() {
        Some(Kinds(store)) => store.clone(),
        None => {
            let store = cx.new(|_| Store { jobs: HashMap::new(), stamps: HashMap::new(), work: HashMap::new(), sends: Vec::new(), fleet: None, watch: None });
            cx.set_global(Kinds(store.clone()));
            store
        }
    }
}

fn fleet_now(cx: &App) -> Option<Entity<FleetConnection>> {
    JobView::live(cx).and_then(|view| view.read(cx).fleet.clone())
}

/// The thread draws again to show what was read or kept. Deferred, so it never updates the view from inside one of its own
/// updates, which a press is.
fn redraw(cx: &mut App) {
    cx.defer(|cx| {
        if let Some(view) = JobView::live(cx) {
            view.update(cx, |_, cx| cx.notify());
        }
    });
}

/// A Job came on screen: its sortings and asks are read, once.
pub(crate) fn shown(job: &str, cx: &mut App) {
    let fleet = fleet_now(cx);
    let store = store(cx);
    let job = job.to_string();
    store.update(cx, |store, cx| store.ensure(job, fleet, cx));
}

/// How the message `row` of Job `job` was sorted; none where it is not sorted, or nobody asked for the Job.
pub(crate) fn sorting(job: &str, row: &str, cx: &App) -> Option<MessageSorting> {
    let Kinds(store) = cx.try_global::<Kinds>()?;
    store.read(cx).jobs.get(job)?.sortings.iter().find(|one| one.row_id == row).cloned()
}

/// The sortings of Job `job`, oldest first.
pub(crate) fn sortings(job: &str, cx: &App) -> Vec<MessageSorting> {
    cx.try_global::<Kinds>().and_then(|Kinds(store)| store.read(cx).jobs.get(job).map(|held| held.sortings.clone())).unwrap_or_default()
}

/// The asks of Job `job`, oldest first.
pub(crate) fn asks(job: &str, cx: &App) -> Vec<Ask> {
    cx.try_global::<Kinds>().and_then(|Kinds(store)| store.read(cx).jobs.get(job).map(|held| held.asks.clone())).unwrap_or_default()
}

/// He changed a message's kind: it is sent over `fleet`, which the view that sends it holds, since the store may not read the
/// view from inside its update. What Fleet kept is read again. A refusal is said in a toast in Fleet's words.
pub(crate) fn send(job: &str, set: SetMessageKind, fleet: Option<Entity<FleetConnection>>, cx: &mut App) {
    let store = store(cx);
    let job = job.to_string();
    store.update(cx, |store, cx| store.send(job, set, fleet, cx));
}

/// Hold `held` for Job `job` as if it had been read.
#[cfg(test)]
pub(crate) fn hold(job: &str, held: Held, cx: &mut App) {
    let store = store(cx);
    store.update(cx, |store, _| {
        store.jobs.insert(job.to_string(), held);
    });
}

impl Store {
    fn ensure(&mut self, job: String, fleet: Option<Entity<FleetConnection>>, cx: &mut Context<Self>) {
        if self.watch.is_none() {
            if let Some(fleet) = &fleet {
                self.watch = Some(cx.observe(fleet, |this, _, cx| this.moved(cx)));
            }
        }
        self.fleet = fleet;
        if !self.jobs.contains_key(&job) {
            self.read(job, cx);
        }
    }

    /// A Job's record moved: each Job held is read again where its record says more may have happened.
    fn moved(&mut self, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let stale: Vec<String> = fleet.read(cx).held().jobs.iter().filter(|record| self.stamps.get(record.id.as_str()).is_some_and(|before| *before != stamp_of(record))).map(|record| record.id.as_str().to_string()).collect();
        for job in stale {
            self.read(job, cx);
        }
    }

    fn read(&mut self, job: String, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else {
            self.jobs.insert(job.clone(), mock::held(&job));
            redraw(cx);
            return;
        };
        let held = fleet.read(cx).held();
        if let Some(stamp) = held.jobs.iter().find(|record| record.id.as_str() == job).map(stamp_of) {
            self.stamps.insert(job.clone(), stamp);
        }
        let Some(port) = held.port() else { return };
        let (sortings, asks) = (read_sortings(port, &job, cx), read_asks(port, &job, cx));
        let named = job.clone();
        self.work.insert(
            job,
            cx.spawn(async move |this, cx| {
                let (sortings, asks) = (sortings.await, asks.await);
                let _ = this.update(cx, |this, cx| this.took(named, sortings, asks, cx));
            }),
        );
    }

    /// What was read. A read that failed keeps what was held, and a Job with nothing held draws no chip.
    fn took(&mut self, job: String, sortings: Result<MessageSortings, Refused>, asks: Result<JobAsks, Refused>, cx: &mut Context<Self>) {
        let before = self.jobs.get(&job).cloned();
        let mut held = before.clone().unwrap_or_default();
        match sortings {
            Ok(read) => held.sortings = read.messages,
            Err(_) if before.is_none() => return,
            Err(_) => {}
        }
        if let Ok(read) = asks {
            held.asks = read.asks;
        }
        if before.as_ref() != Some(&held) {
            self.jobs.insert(job, held);
            redraw(cx);
        }
    }

    fn send(&mut self, job: String, set: SetMessageKind, fleet: Option<Entity<FleetConnection>>, cx: &mut Context<Self>) {
        let Some(fleet) = fleet else {
            let held = self.jobs.entry(job).or_default();
            match mock::apply(held, &set) {
                Ok(_) => redraw(cx),
                Err(why) => told(why.said, cx),
            }
            return;
        };
        let Some(port) = fleet.read(cx).held().port() else {
            told("Fleet is not connected".into(), cx);
            return;
        };
        self.fleet = Some(fleet);
        let sent = post_kind(port, &set, cx);
        self.sends.retain(|task| !task.is_ready());
        self.sends.push(cx.spawn(async move |this, cx| {
            let _ = match sent.await {
                Ok(_) => this.update(cx, |this, cx| this.read(job, cx)),
                Err(why) => this.update(cx, |_, cx| told(why.said, cx)),
            };
        }));
    }
}

/// Fleet's own words, in a toast.
fn told(said: String, cx: &mut App) {
    #[cfg(test)]
    cx.default_global::<Told>().0.push(said.clone());
    cx.defer(move |cx| crate::toasts::tell(said, cx));
}

/// What was told, since the toasts hold their sentences to themselves.
#[cfg(test)]
#[derive(Default)]
struct Told(Vec<String>);

#[cfg(test)]
impl gpui::Global for Told {}

#[cfg(test)]
pub(crate) fn told_in_tests(cx: &mut gpui::VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| cx.default_global::<Told>().0.clone())
}
