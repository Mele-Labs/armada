//! Which conversation a message belongs to, and the one reply at a time each
//! conversation writes. `#939`.
//!
//! **A conversation is looked up by a [`ConversationKey`], and a key is a
//! repository today.** A session per topic is a second constructor on the key;
//! the table, the thread and the host are keyed by whatever it holds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ipc::EventsSince;

use super::asking::{Asks, HelmAskHold};
use super::hosting::Hosting;
use super::thread::Thread;

/// Which conversation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationKey(String);

impl ConversationKey {
    /// A repository's conversation. **The one constructor**, so a key per topic
    /// is a second one here and nothing else.
    pub fn of_repository(manifest: &core_model::ManifestId) -> ConversationKey {
        ConversationKey(manifest.as_str().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Where what its messages carried is kept, under `attachments_dir`, named as the thread's
    /// file is so that no key can name a path outside it.
    pub(crate) fn uploads_in(&self, attachments_dir: &str) -> PathBuf {
        Path::new(attachments_dir).join("helm").join(self.safe_name())
    }

    fn safe_name(&self) -> String {
        self.0
            .chars()
            .map(
                |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    true => c,
                    false => '_',
                },
            )
            .collect()
    }

    /// Where its thread is kept. A character a file name should not carry is
    /// written as `_`, so no key can name a path outside the directory.
    pub(crate) fn thread_in(&self, records_root: &str) -> PathBuf {
        let name = self.safe_name();
        Path::new(records_root)
            .join("helm")
            .join(format!("{name}.jsonl"))
    }
}

/// Every conversation Fleet hosts, and the host that carries their messages.
pub struct Conversations {
    host: Arc<dyn Hosting>,
    open: Mutex<BTreeMap<ConversationKey, Arc<Conversation>>>,
    /// Every call held open for a person, across every conversation. **Here
    /// rather than on a [`Conversation`]** because the dock is one list: what
    /// is waiting on a person is waiting on them wherever it came from.
    asks: Asks,
}

impl Conversations {
    pub fn hosted_by(host: Arc<dyn Hosting>, hold: HelmAskHold) -> Conversations {
        Conversations {
            host,
            open: Mutex::new(BTreeMap::new()),
            asks: Asks::holding_for(hold),
        }
    }

    pub(crate) fn host(&self) -> &dyn Hosting {
        self.host.as_ref()
    }

    pub(crate) fn asks(&self) -> &Asks {
        &self.asks
    }

    /// The hold this Fleet was assembled with, so a Fleet rebuilt around a
    /// different host keeps the bound the composition root resolved.
    pub fn hold(&self) -> HelmAskHold {
        self.asks.held_for()
    }

    /// The conversation `key` names, its thread read from under `records_root`
    /// the first time it is asked for.
    pub(crate) fn open(&self, key: &ConversationKey, records_root: &str) -> Arc<Conversation> {
        let mut open = self
            .open
            .lock()
            .expect("the conversations are not held across a panic");
        let conversation = open.entry(key.clone()).or_insert_with(|| {
            Arc::new(Conversation {
                turn: tokio::sync::Mutex::new(()),
                waiting: AtomicUsize::new(0),
                thread: Thread::opened(key.thread_in(records_root)),
                polled: Mutex::new(None),
                commands: Mutex::new(Vec::new()),
                tuned: Mutex::new((None, None)),
            })
        });
        Arc::clone(conversation)
    }
}

/// One conversation.
pub(crate) struct Conversation {
    /// Held while a reply is written. **One at a time**: a second message
    /// resumes the session the first reply ran in, so it waits for that one.
    pub(crate) turn: tokio::sync::Mutex<()>,
    /// Messages taken and not yet answered.
    waiting: AtomicUsize,
    pub(crate) thread: Thread,
    /// What this conversation's last `get_events_since` was answered. **In
    /// memory for this run of Fleet** — the record says so rather than
    /// claiming the session never polled. `#1367`.
    polled: Mutex<Option<EventsSince>>,
    /// The commands Helm's own session last said it has. **In memory**: empty until it has
    /// spoken in this run of Fleet, and `/` offers the Sessions' list meanwhile.
    commands: Mutex<Vec<String>>,
    /// The model and effort chosen for this conversation as last read or set, so what a
    /// listener publishes does not need the store. The store is the record.
    tuned: Mutex<(Option<String>, Option<String>)>,
}

impl Conversation {
    pub(crate) fn taken(&self) {
        self.waiting.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn answered(&self) {
        self.waiting.fetch_sub(1, Ordering::SeqCst);
    }

    pub(crate) fn replying(&self) -> bool {
        self.waiting.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn polled(&self, counted: EventsSince) {
        *self.held_poll() = Some(counted);
    }

    pub(crate) fn last_poll(&self) -> Option<EventsSince> {
        self.held_poll().clone()
    }

    /// What Helm's session said it has. `true` where that is a change.
    pub(crate) fn heard_commands(&self, names: Vec<String>) -> bool {
        let mut held = self
            .commands
            .lock()
            .expect("the commands are not held across a panic");
        let changed = *held != names;
        *held = names;
        changed
    }

    pub(crate) fn commands(&self) -> Vec<String> {
        self.commands
            .lock()
            .expect("the commands are not held across a panic")
            .clone()
    }

    pub(crate) fn tuned_to(&self, model: Option<String>, effort: Option<String>) {
        *self
            .tuned
            .lock()
            .expect("the tuning is not held across a panic") = (model, effort);
    }

    pub(crate) fn tuning(&self) -> (Option<String>, Option<String>) {
        self.tuned
            .lock()
            .expect("the tuning is not held across a panic")
            .clone()
    }

    fn held_poll(&self) -> std::sync::MutexGuard<'_, Option<EventsSince>> {
        self.polled
            .lock()
            .expect("the last poll is not held across a panic")
    }
}
