//! A Job's Commander's thread: what was said, kept in a file under the
//! repository's records and offered to whoever is watching. `#939`. (Helm's thread is a Session's
//! now, and is kept in the store with its rows.)
//!
//! **The file is the thread; the session is not.** The agent CLI keeps its own
//! transcript, in its own format and on its own retention, and a Bridge opened
//! after a restart reads the thread from here.
//!
//! **Appended as it is said, synchronously.** A reply is a handful of lines, and
//! a queue in front of them could leave a thread short when Fleet restarts
//! straight after a reply.

use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use api::{ConversationFeed, ConversationWatch, Spoken};
use serde::de::DeserializeOwned;

/// How many messages a viewer is sent when it opens. What is left out is
/// counted.
pub const THREAD: usize = 512;

/// What was said in the Helm Session's thread: the person's asks and the session's prose, in
/// order. `#1293`.
///
/// **Fleet's own read and never a tool.** `get_helm_session` refuses every agent, so a scout
/// reaching a thread through a tool would be that door opened; a read-in hands over this text
/// instead.
///
/// Everything else in the thread is the session's working (its calls, the asks it put to the
/// person, where it sent words), which is not what was argued.
pub fn what_was_said(rows: &[ipc::SessionRow]) -> String {
    let mut said: Vec<String> = Vec::new();
    for row in rows {
        let ipc::SessionRow::Message { from, text, .. } = row else {
            continue;
        };
        let who = match from {
            ipc::SessionVoice::You => "The person",
            ipc::SessionVoice::Agent => "Helm",
            ipc::SessionVoice::Session { title, .. } => title.as_str(),
        };
        if !text.trim().is_empty() {
            said.push(format!("{who}: {}", text.trim()));
        }
    }
    said.join("\n\n")
}

/// A conversation's thread. **Generic over what is said**, so a Job's
/// Commander, whose conversation is Helm's host keyed by the Job, is kept in
/// the same file format and offered on the same channel and not a copy of them.
pub(crate) struct Thread<M> {
    feed: ConversationFeed<M>,
    held: Mutex<Held<M>>,
    file: PathBuf,
}

struct Held<M> {
    messages: VecDeque<M>,
    skipped: u64,
}

impl<M> Default for Held<M> {
    fn default() -> Held<M> {
        Held {
            messages: VecDeque::new(),
            skipped: 0,
        }
    }
}

impl<M> Held<M> {
    fn push(&mut self, message: M) {
        if self.messages.len() == THREAD {
            self.messages.pop_front();
            self.skipped += 1;
        }
        self.messages.push_back(message);
    }
}

impl<M: Spoken + DeserializeOwned> Thread<M> {
    /// The thread `file` holds. **A line that will not decode is counted**, for
    /// the transcript backfill's reason: it was a message.
    pub(crate) fn opened(file: PathBuf) -> Thread<M> {
        let mut held = Held::default();
        if let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines().filter(|line| !line.trim().is_empty()) {
                match ipc::decode::<M>("a conversation's thread line", line.as_bytes()) {
                    // Live only; a hand-edited line that says so is not a thread message.
                    Ok(message) if message.live_only() => {}
                    Ok(message) => held.push(message),
                    Err(_) => held.skipped += 1,
                }
            }
        }
        Thread {
            feed: ConversationFeed::new(),
            held: Mutex::new(held),
            file,
        }
    }

    /// Say something into the thread: held, offered, written.
    ///
    /// **A file that will not take the line loses it on a restart and nowhere
    /// else** — viewers already have it, and the session remembers regardless.
    pub(crate) fn say(&self, message: M) {
        if message.live_only() {
            return self.offered_only(message);
        }
        let mut held = self.held();
        if let Ok(line) = ipc::encode(&message) {
            let _ = self.appended(&line);
        }
        held.push(message.clone());
        self.feed.offer(message);
    }

    /// Offer a message to whoever is watching and keep it nowhere: not in the backfill and not in
    /// the file. What the message box runs on is read again from the next `opened`, so a viewer
    /// that missed one loses nothing a reload would not give back.
    pub(crate) fn offered_only(&self, message: M) {
        self.feed.offer(message);
    }

    /// The subscription, then the thread, under one lock so nothing said
    /// between the two is lost or sent twice.
    pub(crate) fn watched(&self) -> (ConversationWatch<M>, Vec<M>, u64) {
        let held = self.held();
        let watch = self.feed.watch();
        (watch, held.messages.iter().cloned().collect(), held.skipped)
    }

    /// The thread as it stands, and how many older lines it has dropped.
    /// **No subscription**, unlike [`Thread::watched`]: a record is taken
    /// once and nothing follows it.
    pub(crate) fn taken(&self) -> (Vec<M>, u64) {
        let held = self.held();
        (held.messages.iter().cloned().collect(), held.skipped)
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Held<M>> {
        self.held
            .lock()
            .expect("the thread is not held across a panic")
    }

    fn appended(&self, line: &str) -> std::io::Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)?;
        writeln!(out, "{line}")
    }
}
