//! What a done card leads with: one line an ask of the Job, open ones first, which Fleet writes into `JobStory.lead`. The
//! card reads them here. Each Job's story is read once, when the Job comes on screen, and again where Fleet says it moved
//! (`session.story`) or its record changes, so the card, which draws from `&self`, finds the answer held.

use std::collections::{HashMap, HashSet};

use gpui::{App, AppContext as _, Context, Entity, Subscription, Task};

use super::model::{self, Stand};
use super::{mock, read_story, stamp_of, Stamp};
use crate::fleet::{FleetConnection, StoryMoved};
use crate::job_view::JobView;

/// The lead lines of one Job.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lines {
    Reading,
    Read(Vec<(Stand, String)>),
}

struct Store {
    lines: HashMap<String, Lines>,
    /// The Jobs a card asked for, so a word that a story moved is read for those alone.
    asked: HashSet<String>,
    stamps: HashMap<String, Stamp>,
    work: HashMap<String, Task<()>>,
    fleet: Option<Entity<FleetConnection>>,
    watch: Option<Subscription>,
    told: Option<Subscription>,
}

struct Leads(Entity<Store>);

impl gpui::Global for Leads {}

/// A Job came on screen: its story is read for the done card, once.
pub(crate) fn shown(job: &str, cx: &mut App) {
    let fleet = JobView::live(cx).and_then(|view| view.read(cx).fleet.clone());
    shown_on(job, fleet, cx);
}

/// A Job came on screen over `fleet`: its story is read for the done card, once.
pub(crate) fn shown_on(job: &str, fleet: Option<Entity<FleetConnection>>, cx: &mut App) {
    let store = match cx.try_global::<Leads>() {
        Some(Leads(store)) => store.clone(),
        None => {
            let store = cx.new(|_| Store { lines: HashMap::new(), asked: HashSet::new(), stamps: HashMap::new(), work: HashMap::new(), fleet: None, watch: None, told: None });
            cx.set_global(Leads(store.clone()));
            store
        }
    };
    let job = job.to_string();
    store.update(cx, |store, cx| store.ensure(job, fleet, cx));
}

/// The lines held for Job `job`, or none where nobody asked for them.
#[allow(dead_code)]
pub(crate) fn lines(job: &str, cx: &App) -> Option<Lines> {
    cx.try_global::<Leads>().and_then(|Leads(store)| store.read(cx).lines.get(job).cloned())
}

impl Store {
    fn ensure(&mut self, job: String, fleet: Option<Entity<FleetConnection>>, cx: &mut Context<Self>) {
        if self.watch.is_none() {
            if let Some(fleet) = &fleet {
                self.watch = Some(cx.observe(fleet, |this, _, cx| this.moved(cx)));
                self.told = Some(cx.subscribe(fleet, |this, _, moved: &StoryMoved, cx| this.story_moved(moved, cx)));
            }
        }
        self.asked.insert(job.clone());
        if fleet.is_some() {
            self.fleet = fleet;
        }
        if !self.lines.contains_key(&job) {
            self.read(job, cx);
        }
    }

    /// A Job's record moved: the lines of each Job that held some are read again where its record says more may have happened.
    fn moved(&mut self, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let stale: Vec<String> = fleet.read(cx).held().jobs.iter().filter(|record| self.stamps.get(record.id.as_str()).is_some_and(|before| *before != stamp_of(record))).map(|record| record.id.as_str().to_string()).collect();
        for job in stale {
            self.read(job, cx);
        }
    }

    /// Fleet said a story moved: it is read again for each Job named that a card asked for.
    fn story_moved(&mut self, moved: &StoryMoved, cx: &mut Context<Self>) {
        for one in &moved.0 {
            if self.asked.contains(one.session_id.as_str()) {
                self.read(one.session_id.as_str().to_string(), cx);
            }
        }
    }

    fn read(&mut self, job: String, cx: &mut Context<Self>) {
        #[cfg(test)]
        cx.default_global::<Reads>().0.push(job.clone());
        let Some(fleet) = self.fleet.clone() else {
            self.lines.insert(job.clone(), Lines::Read(model::lead(&mock::story(&job))));
            return;
        };
        let held = fleet.read(cx).held();
        if let Some(stamp) = held.jobs.iter().find(|record| record.id.as_str() == job).map(stamp_of) {
            self.stamps.insert(job.clone(), stamp);
        }
        let Some(port) = held.port() else { return };
        self.lines.entry(job.clone()).or_insert(Lines::Reading);
        let story = read_story(port, &job, cx);
        let named = job.clone();
        self.work.insert(
            job,
            cx.spawn(async move |this, cx| {
                // A story that cannot be read leads with nothing.
                let lines = Lines::Read(story.await.map(|story| model::lead(&story)).unwrap_or_default());
                let _ = this.update(cx, |this, cx| {
                    if this.lines.get(&named) != Some(&lines) {
                        this.lines.insert(named, lines);
                        // The card is drawn from the Job view, which draws again to show them.
                        if let Some(view) = JobView::live(cx) {
                            view.update(cx, |_, cx| cx.notify());
                        }
                    }
                });
            }),
        );
    }
}

/// The Jobs whose story was asked for, in order, for a test of what makes it read again.
#[cfg(test)]
#[derive(Default)]
struct Reads(Vec<String>);

#[cfg(test)]
impl gpui::Global for Reads {}

#[cfg(test)]
pub(crate) fn reads_in_tests(cx: &App) -> Vec<String> {
    cx.try_global::<Reads>().map(|reads| reads.0.clone()).unwrap_or_default()
}
