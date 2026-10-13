//! Which conversation a message belongs to, and the one reply at a time each
//! conversation writes. `#939`.
//!
//! **A conversation is looked up by a [`ConversationKey`]: a repository's, or a
//! Job's.** A Job's is its Commander's (`#2166`), the second constructor on the
//! key the first version of this note said a topic would be; the table, the
//! thread and the host are keyed by whatever it holds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ipc::{CommanderMessage, EventsSince, HelmAsked, Instant};

use super::asking::{Asks, HelmAskHold};
use super::hosting::Hosting;
use super::thread::Thread;

/// What a Job's key starts with. **A character a repository's key never holds**
/// ([`ConversationKey::of_repository`] writes a `:` as `_`), so the two cannot
/// be the same string whatever a Manifest is called.
const JOB_PREFIX: &str = "commander:";

/// Which conversation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationKey {
    /// What `helm_sessions` is keyed by.
    stored: String,
    job: bool,
}

impl ConversationKey {
    /// A repository's conversation: Helm's. **A `:` in the Manifest's id is
    /// written as `_`**, which is what keeps this from ever equalling
    /// [`ConversationKey::of_job`]'s.
    pub fn of_repository(manifest: &core_model::ManifestId) -> ConversationKey {
        ConversationKey {
            stored: manifest.as_str().replace(':', "_"),
            job: false,
        }
    }

    /// A Job's conversation: its Commander's. Prefixed, so it cannot collide
    /// with a repository's.
    pub fn of_job(job: &core_model::JobId) -> ConversationKey {
        ConversationKey {
            stored: format!("{JOB_PREFIX}{}", job.as_str()),
            job: true,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.stored
    }

    /// Where its thread is kept. A character a file name should not carry is
    /// written as `_`, so no key can name a path outside the directory. **A
    /// Job's lives in a directory of its own**, because `commander:x` and
    /// `commander_x` both become `commander_x` and one directory would let a
    /// repository called the second read the first's thread.
    pub(crate) fn thread_in(&self, records_root: &str) -> PathBuf {
        let name: String = self
            .stored
            .chars()
            .map(
                |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    true => c,
                    false => '_',
                },
            )
            .collect();
        Path::new(records_root)
            .join(if self.job { "commander" } else { "helm" })
            .join(format!("{name}.jsonl"))
    }
}

/// Every conversation Fleet hosts, and the host that carries their messages.
pub struct Conversations {
    host: Arc<dyn Hosting>,
    open: Mutex<BTreeMap<ConversationKey, Arc<Conversation>>>,
    /// Every Job's Commander that has been written to or woken this run.
    commanders: Mutex<BTreeMap<ConversationKey, Arc<Commanded>>>,
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
            commanders: Mutex::new(BTreeMap::new()),
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
            })
        });
        Arc::clone(conversation)
    }
}

impl Conversations {
    /// The Commander conversation `key` names, its thread read from under
    /// `records_root` the first time it is asked for.
    pub(crate) fn commanded(&self, key: &ConversationKey, records_root: &str) -> Arc<Commanded> {
        let mut open = self
            .commanders
            .lock()
            .expect("the commanders are not held across a panic");
        let commanded = open.entry(key.clone()).or_insert_with(|| {
            Arc::new(Commanded {
                turn: tokio::sync::Mutex::new(()),
                waiting: Mutex::new(Waiting::default()),
                thread: Thread::opened(key.thread_in(records_root)),
            })
        });
        Arc::clone(commanded)
    }
}

/// Why a turn is wanted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The owner's words.
    Owner(String),
    /// Fleet saw something on the Job, said in a line. **Sent by
    /// `crate::commanding_wakes`**, which decides what is worth a wake.
    Wake(String),
}

/// What is wanted of a Commander and has not been written yet.
#[derive(Default)]
struct Waiting {
    /// A turn is being written, or about to be. **While it is set a new input
    /// only joins the queue**: the turn after it takes everything waiting.
    running: bool,
    inputs: Vec<Input>,
}

/// One Job's Commander conversation.
///
/// **One turn at a time, and an input never interrupts one.** The queue is the
/// whole mechanism: an input that arrives mid-turn waits, and the next turn
/// takes every input waiting as one, so ten wakes in a row are one turn and not
/// ten. [`Commanded::take`] and [`Commanded::next`] are the only two doors.
pub struct Commanded {
    /// Held while a turn is written, so a start-fresh (when there is one) and a
    /// turn never meet. The queue, and not this, is what orders the turns.
    pub(crate) turn: tokio::sync::Mutex<()>,
    waiting: Mutex<Waiting>,
    pub(crate) thread: Thread<CommanderMessage>,
}

impl Commanded {
    /// Put an input on the queue. `true` where the caller must start the turns
    /// running; `false` where a run already is and will take this one.
    pub fn take(&self, input: Input) -> bool {
        let mut waiting = self.queue();
        waiting.inputs.push(input);
        !std::mem::replace(&mut waiting.running, true)
    }

    /// Everything waiting, for one turn. `None` where nothing is, and then
    /// nothing is running either: **both are decided under one lock**, so an
    /// input cannot arrive between the last turn finding nothing and the run
    /// ending, and be left until the next input.
    ///
    /// `on_idle` runs under that lock when nothing is waiting, so the word that
    /// the Commander is idle cannot be published after a later input has
    /// already started it working again.
    pub fn next(&self, on_idle: impl FnOnce()) -> Option<Vec<Input>> {
        let mut waiting = self.queue();
        if waiting.inputs.is_empty() {
            waiting.running = false;
            on_idle();
            return None;
        }
        Some(std::mem::take(&mut waiting.inputs))
    }

    /// The owner's words: **on the thread now**, so a message sent mid-turn is
    /// seen at once and answered after, and then on the queue. `true` as
    /// [`Commanded::take`] answers it.
    pub fn owner_wrote(&self, at: Instant, text: &str) -> bool {
        self.thread.say(CommanderMessage::Asked(HelmAsked {
            ts: at,
            text: text.to_string(),
        }));
        self.take(Input::Owner(text.to_string()))
    }

    /// Everything said in this conversation so far, oldest first.
    pub fn said(&self) -> Vec<CommanderMessage> {
        self.thread.taken().0
    }

    /// A turn is being written, or an input is waiting for one.
    pub fn replying(&self) -> bool {
        let waiting = self.queue();
        waiting.running || !waiting.inputs.is_empty()
    }

    fn queue(&self) -> std::sync::MutexGuard<'_, Waiting> {
        self.waiting
            .lock()
            .expect("the queue is not held across a panic")
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

    fn held_poll(&self) -> std::sync::MutexGuard<'_, Option<EventsSince>> {
        self.polled
            .lock()
            .expect("the last poll is not held across a panic")
    }
}
