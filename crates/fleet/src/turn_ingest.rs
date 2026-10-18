//! Turn usage read from the transcripts on disk, so nothing has to send it. `docs/concepts/session.md`,
//! *Where the tokens went*.
//!
//! **Two passes over one reader.** At start, the files written to in the last [`BACKFILL_DAYS`] days
//! are read once, one at a time, with a pause between reads; on the notice loop, the files of every
//! live session are read from where the last read left off. Either way a file's offset is kept in
//! the store with the turns it gave, so a restart never parses a whole file again.
//!
//! **A session Fleet does not know gets its rows too**: the transcripts are the record, and
//! `get_session_usage` still refuses a session it has no ledger row for.
//!
//! **Bounded on purpose.** This machine has about 1,600 sessions a week, and a burst of parsing has
//! frozen it twice. One file at a time, [`CHUNK`] bytes at a time, [`PAUSE`] between reads, the
//! parse off the runtime's threads, and a file whose offset is its size costs only the `stat` that
//! found it.

use std::time::{Duration, SystemTime};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::turn_lines::{self, TranscriptFile};
use core_model::{Component, Envelope, FieldValue, Level};
use store::{SessionSearch, SessionState};

use crate::daemon::Fleet;
use crate::turn_usage_keeping::kept_form;

/// How far back the start-up pass looks, by when a file was last written.
pub(crate) const BACKFILL_DAYS: u64 = 7;
/// How much of a file one read takes in.
const CHUNK: u64 = 8 * 1024 * 1024;
/// How often the notice loop reads the live sessions' transcripts.
pub(crate) const EVERY: Duration = Duration::from_secs(5);
/// How long after start the week's backfill waits, so it does not compete with Fleet coming up.
pub(crate) const BACKFILL_AFTER: Duration = Duration::from_secs(30);
/// How long the reader rests between reads.
const PAUSE: Duration = Duration::from_millis(40);

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
    /// Read every transcript written to in the last week that has more in it than was read. How
    /// many turns changed the store. `from_start` ignores the offsets kept, which a test uses to
    /// show that reading again changes nothing.
    pub(crate) async fn turns_backfilled(&self, from_start: bool) -> usize {
        let home = self.host().home.clone();
        let since = SystemTime::now() - Duration::from_secs(BACKFILL_DAYS * 86_400);
        let Ok(files) = tokio::task::spawn_blocking(move || turn_lines::transcripts_since(&home, since)).await else {
            return 0;
        };
        let mut changed = 0;
        let count = files.len();
        self.backfill_said("turn usage backfill started", count, 0);
        for file in files {
            changed += self.transcript_taken_in(&file, from_start).await;
            tokio::time::sleep(PAUSE).await;
        }
        self.backfill_said("turn usage backfill finished", count, changed);
        changed
    }

    /// The backfill's start and end, in the envelope on Fleet's standard error: how many transcript
    /// files it covers, and how many turn rows it has changed.
    fn backfill_said(&self, msg: &str, files: usize, rows: usize) {
        let envelope = Envelope::new(self.now(), Level::Info, Component::Fleet, self.run().clone(), msg)
            .with_field("files", FieldValue::Int(files as i64))
            .with_field("rows", FieldValue::Int(rows as i64));
        crate::transcript::note_aloud(&envelope);
    }

    /// Read what the live sessions' transcripts gained since they were last read. How many turns
    /// changed the store. A transcript that has not grown costs a `stat`.
    pub(crate) async fn turns_followed(&self) -> usize {
        let live = {
            let store = self.store().lock().await;
            store
                .find_sessions(&SessionSearch { state: Some(SessionState::Live), ..Default::default() })
                .unwrap_or_default()
        };
        if live.is_empty() {
            return 0;
        }
        let home = self.host().home.clone();
        let ids: Vec<String> = live.into_iter().map(|one| one.id).collect();
        let Ok(files) = tokio::task::spawn_blocking(move || {
            ids.iter().flat_map(|id| turn_lines::transcripts_of(&home, id)).collect::<Vec<_>>()
        })
        .await
        else {
            return 0;
        };
        let mut changed = 0;
        for file in files {
            changed += self.transcript_taken_in(&file, false).await;
        }
        changed
    }

    /// One transcript, read from its kept offset to its end in bounded reads.
    async fn transcript_taken_in(&self, file: &TranscriptFile, from_start: bool) -> usize {
        let key = file.path.to_string_lossy().to_string();
        let (mut changed, mut first) = (0, true);
        loop {
            let from = if from_start && first {
                0
            } else {
                self.store().lock().await.transcript_offset(&key).ok().flatten().unwrap_or(0)
            };
            first = false;
            if !from_start && from == file.size {
                return changed;
            }
            let path = file.path.clone();
            let subagent = file.subagent;
            let Ok(Ok(read)) =
                tokio::task::spawn_blocking(move || turn_lines::read_turns(&path, from, subagent, CHUNK)).await
            else {
                return changed;
            };
            let turns: Vec<_> = read.turns.iter().map(kept_form).collect();
            match self.store().lock().await.keep_turns_read(&file.session_id, &turns, &key, read.next) {
                Ok(more) => changed += more,
                Err(_) => return changed,
            }
            if read.next == from || read.next >= file.size {
                return changed;
            }
            tokio::time::sleep(PAUSE).await;
        }
    }
}
