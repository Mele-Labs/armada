//! [`Trackers`] scripted by a test: which tracker and link reader each provider
//! gets, and every choice Fleet asked with. #91.
//!
//! **Keyed by provider**, so one Fleet over two repositories can be given two
//! different fakes and a test can read which Manifest was answered by which.
//! A provider nothing was planted for gets what the absent provider gets, and
//! that is an empty tracker and a link reader that resolves nothing until a
//! test says otherwise.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{Backlog, LinkLookup, ReadOnly, TicketState, Tool, ToolInstalled, Tracker, TrackerChoice, TrackerWrites, Trackers};

use crate::{FakeBeads, FakeLinkLookup, FakeTracker};

type Provider = Option<String>;

pub struct FakeTrackers {
    trackers: Mutex<BTreeMap<Provider, Arc<dyn Tracker + Send + Sync>>>,
    links: Mutex<BTreeMap<Provider, Arc<dyn LinkLookup + Send + Sync>>>,
    /// What writes to each provider's issues: the tracker planted for it, which records them.
    writers: Mutex<BTreeMap<Provider, Arc<dyn TrackerWrites + Send + Sync>>>,
    /// Only the providers a test planted a backlog for have one, as only beads does.
    backlogs: Mutex<BTreeMap<Provider, Arc<dyn Backlog + Send + Sync>>>,
    /// The words a provider declares for the ticket's states. None for the ones a test did not say.
    ticket_words: Mutex<BTreeMap<Provider, &'static [(TicketState, &'static str)]>>,
    asked: Mutex<Vec<TrackerChoice>>,
    installed: Mutex<Vec<&'static str>>,
    installed_in: Mutex<Vec<String>>,
}

impl FakeTrackers {
    /// Lists nothing, takes every write and resolves no link, for every provider.
    pub fn empty() -> FakeTrackers {
        let empty = Arc::new(FakeTracker::empty());
        FakeTrackers {
            trackers: Mutex::new(BTreeMap::from([(None, Arc::clone(&empty) as Arc<dyn Tracker + Send + Sync>)])),
            writers: Mutex::new(BTreeMap::from([(None, empty as Arc<dyn TrackerWrites + Send + Sync>)])),
            links: Mutex::new(BTreeMap::from([(
                None,
                Arc::new(FakeLinkLookup::resolving_nothing()) as Arc<dyn LinkLookup + Send + Sync>,
            )])),
            backlogs: Mutex::new(BTreeMap::new()),
            ticket_words: Mutex::new(BTreeMap::new()),
            asked: Mutex::new(Vec::new()),
            installed: Mutex::new(Vec::new()),
            installed_in: Mutex::new(Vec::new()),
        }
    }

    /// This tracker for a Manifest that names no provider, and so for any provider nothing was planted for.
    pub fn tracker(self, tracker: Arc<FakeTracker>) -> FakeTrackers {
        self.tracker_for(None, tracker)
    }

    /// This tracker for the Manifests that name `provider`.
    pub fn tracker_named(self, provider: &str, tracker: Arc<FakeTracker>) -> FakeTrackers {
        self.tracker_for(Some(provider.to_string()), tracker)
    }

    /// This link reader for a Manifest that names no provider, and so for any provider nothing was planted for.
    pub fn links(self, links: Arc<FakeLinkLookup>) -> FakeTrackers {
        self.links_for(None, links)
    }

    /// This link reader for the Manifests that name `provider`.
    pub fn links_named(self, provider: &str, links: Arc<FakeLinkLookup>) -> FakeTrackers {
        self.links_for(Some(provider.to_string()), links)
    }

    /// This copy of a backlog, as the tracker and the backlog of the Manifests that name `provider`.
    pub fn beads_named(self, provider: &str, beads: Arc<FakeBeads>) -> FakeTrackers {
        let provider = Some(provider.to_string());
        self.trackers.lock().expect("not poisoned").insert(provider.clone(), Arc::clone(&beads) as Arc<dyn Tracker + Send + Sync>);
        // Beads is written through its backlog, and a saved rule's writes are not planned against it here.
        self.writers.lock().expect("not poisoned").insert(provider.clone(), Arc::new(ReadOnly));
        self.backlogs.lock().expect("not poisoned").insert(provider, beads);
        self
    }

    /// These words for the ticket's states, declared by the Manifests that name `provider`.
    pub fn ticket_words_named(self, provider: &str, words: &'static [(TicketState, &'static str)]) -> FakeTrackers {
        self.ticket_words.lock().expect("not poisoned").insert(Some(provider.to_string()), words);
        self
    }

    /// Every tool Fleet was asked to install, in order. Each is answered as installed.
    pub fn installs(&self) -> Vec<&'static str> {
        self.installed.lock().expect("not poisoned").clone()
    }

    /// The repository each install was asked for, in order.
    pub fn installs_in(&self) -> Vec<String> {
        self.installed_in.lock().expect("not poisoned").clone()
    }

    /// Every choice Fleet resolved a tracker or a link reader for, in order.
    pub fn choices_asked(&self) -> Vec<TrackerChoice> {
        self.asked.lock().expect("not poisoned").clone()
    }

    fn tracker_for(self, provider: Provider, tracker: Arc<FakeTracker>) -> FakeTrackers {
        self.writers.lock().expect("not poisoned").insert(provider.clone(), Arc::clone(&tracker) as Arc<dyn TrackerWrites + Send + Sync>);
        self.trackers
            .lock()
            .expect("not poisoned")
            .insert(provider, tracker);
        self
    }

    fn links_for(self, provider: Provider, links: Arc<FakeLinkLookup>) -> FakeTrackers {
        self.links
            .lock()
            .expect("not poisoned")
            .insert(provider, links);
        self
    }
}

impl Trackers for FakeTrackers {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync> {
        self.asked.lock().expect("not poisoned").push(choice.clone());
        let held = self.trackers.lock().expect("not poisoned");
        let provider = choice.provider().map(String::from);
        Arc::clone(held.get(&provider).unwrap_or_else(|| &held[&None]))
    }

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync> {
        self.asked.lock().expect("not poisoned").push(choice.clone());
        let held = self.links.lock().expect("not poisoned");
        let provider = choice.provider().map(String::from);
        Arc::clone(held.get(&provider).unwrap_or_else(|| &held[&None]))
    }

    fn backlog(&self, choice: &TrackerChoice) -> Option<Arc<dyn Backlog + Send + Sync>> {
        let provider = choice.provider().map(String::from);
        self.backlogs.lock().expect("not poisoned").get(&provider).cloned()
    }

    fn ticket_words(&self, choice: &TrackerChoice) -> &'static [(TicketState, &'static str)] {
        let provider = choice.provider().map(String::from);
        self.ticket_words.lock().expect("not poisoned").get(&provider).copied().unwrap_or(&[])
    }

    fn writer(&self, choice: &TrackerChoice) -> Arc<dyn TrackerWrites + Send + Sync> {
        let held = self.writers.lock().expect("not poisoned");
        let provider = choice.provider().map(String::from);
        Arc::clone(held.get(&provider).unwrap_or_else(|| &held[&None]))
    }

    fn install(&self, tool: Tool, in_repo: &str) -> ToolInstalled {
        self.installed.lock().expect("not poisoned").push(tool.name());
        self.installed_in.lock().expect("not poisoned").push(in_repo.to_string());
        ToolInstalled { done: true, said: format!("`{}` is installed", tool.name()) }
    }
}
