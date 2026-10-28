//! The Logbook as the wire has it: read as entries of every state, and a person's acts on one.
//! `get_logbook`, `edit_logbook_entry`, `lock_logbook_entry`, `forget_logbook_entry` and
//! `teach_logbook_entry`, and `tidy_logbook`. `docs/concepts/logbook.md`.
//!
//! **The `From` step is where a store row becomes a wire entry.** An entry the machine note keeps is
//! read in beside the learnt ones with an id of a word and a hash, and no act takes it.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Logbook, Refusal};
use ipc::{
    EditLogbookEntry, Instant, LogbookEarlier, LogbookEntry, LogbookEvidence, ForgottenLogbookEntry, LogbookKind,
    LogbookReplacedBy, LogbookSection, LogbookState, LockLogbookEntry, TeachLogbookEntry, TidiedLogbook, TidyLogbook, WireError,
};
use store::{logbook_words, LogEntry, LogEvidence, LogHistory, LogHow, LogKind, LogRewrite, LogSection, LogState};

use super::answer::MOST_CHARS;
use super::{Logged, Source};
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::run_memory::places::key_of;

/// An id that names no entry. A 404.
const NO_SUCH_ENTRY: &str = "fleet.no_such_logbook_entry";
/// An entry the machine note keeps, which no act takes. A 422.
const NOT_KEPT: &str = "fleet.logbook_entry_not_kept";
/// Text that is empty or over [`MOST_CHARS`]. A 422.
const TEXT_REFUSED: &str = "fleet.logbook_text_refused";
/// Text the person forgot. A 422.
const TEXT_FORGOTTEN: &str = "fleet.logbook_text_forgotten";
/// A `repo` entry that names no repository. A 422.
const REPOSITORY_MISSING: &str = "fleet.logbook_repository_missing";
/// The store would not read or keep it. A 500.
const FAILED: &str = "fleet.logbook_failed";

fn section_of(section: LogSection) -> LogbookSection {
    match section {
        LogSection::Repo => LogbookSection::Repo,
        LogSection::Habits => LogbookSection::Habits,
        LogSection::Machine => LogbookSection::Machine,
    }
}

fn section_in(section: LogbookSection) -> LogSection {
    match section {
        LogbookSection::Repo => LogSection::Repo,
        LogbookSection::Habits => LogSection::Habits,
        LogbookSection::Machine => LogSection::Machine,
    }
}

fn kind_of(kind: LogKind) -> LogbookKind {
    match kind {
        LogKind::Command => LogbookKind::Command,
        LogKind::Gotcha => LogbookKind::Gotcha,
        LogKind::Convention => LogbookKind::Convention,
        LogKind::Environment => LogbookKind::Environment,
        LogKind::Habit => LogbookKind::Habit,
    }
}

fn kind_in(kind: LogbookKind) -> LogKind {
    match kind {
        LogbookKind::Command => LogKind::Command,
        LogbookKind::Gotcha => LogKind::Gotcha,
        LogbookKind::Convention => LogKind::Convention,
        LogbookKind::Environment => LogKind::Environment,
        LogbookKind::Habit => LogKind::Habit,
    }
}

fn state_of(state: LogState) -> LogbookState {
    match state {
        LogState::Candidate => LogbookState::Candidate,
        LogState::Standing => LogbookState::Standing,
        LogState::Faded => LogbookState::Faded,
    }
}

fn instant(millis: u64) -> Instant {
    Instant::carried(rfc3339_utc(i64::try_from(millis).unwrap_or(i64::MAX)))
}

/// FNV-1a over a text's words, so an entry folded in from elsewhere keeps its id between reads.
fn hashed(text: &str) -> u64 {
    logbook_words(text).bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// What an id is for an entry: its number where Fleet keeps it, a word and a hash where the
/// machine note does.
fn id_of(logged: &Logged) -> String {
    match (logged.from, logged.entry.id) {
        (Source::Learnt, Some(id)) => id.to_string(),
        (from, _) => {
            let word = match from {
                Source::Learnt => "learnt",
                Source::Machine => "machine",
            };
            format!("{word}-{:016x}", hashed(&logged.entry.text))
        }
    }
}

/// Whether an id is one [`id_of`] gives an entry Fleet does not keep.
fn is_folded(id: &str) -> bool {
    id.starts_with("machine-")
}

/// A store row as the wire reads it. `root` is the repository's root, where a `repo` entry's is
/// known; `titles` holds the Jobs' titles by id.
fn wired(logged: &Logged, root: Option<&str>, titles: &HashMap<String, String>) -> LogbookEntry {
    let entry = &logged.entry;
    let learnt = logged.from == Source::Learnt;
    let evidence = entry
        .evidence
        .iter()
        .filter(|one| one.how != LogHow::Contradicted)
        .flat_map(|one| {
            let cites = if one.cites.is_empty() { vec![String::new()] } else { one.cites.clone() };
            cites.into_iter().map(move |cite| (one, cite))
        })
        .map(|(one, cite)| LogbookEvidence {
            job_id: one.session_id.clone(),
            job_title: titles.get(&one.session_id).cloned().unwrap_or_else(|| one.session_id.clone()),
            cite,
            says: None,
            at: instant(one.at),
        })
        .collect();
    LogbookEntry {
        id: id_of(logged),
        section: section_of(entry.section),
        repository: (entry.section == LogSection::Repo)
            .then(|| root.map(str::to_string).unwrap_or_else(|| entry.repository.clone()))
            .filter(|repository| !repository.is_empty()),
        kind: kind_of(entry.kind),
        title: entry.title.clone(),
        text: entry.text.clone(),
        paths: entry.paths.clone(),
        evidence,
        confirmed_by: if learnt { entry.confirmations() as u32 } else { 1 },
        first_seen: instant(entry.first_seen),
        last_confirmed: instant(entry.last_confirmed),
        last_served: entry.last_served.map(instant),
        last_tripped: entry.evidence.iter().filter(|one| one.how == LogHow::Tripped).map(|one| one.at).max().map(instant),
        state: state_of(entry.state),
        locked: entry.locked,
        edited: entry.edited_by_person,
        history: entry
            .history
            .iter()
            .rev()
            .map(|was| LogbookEarlier {
                text: was.text.clone(),
                until: instant(was.replaced_at),
                by: match was.why {
                    LogRewrite::Person => LogbookReplacedBy::Person,
                    LogRewrite::Contradicted | LogRewrite::Tripped => LogbookReplacedBy::Distil,
                    LogRewrite::Merged => LogbookReplacedBy::Merged,
                },
            })
            .collect(),
    }
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
    fn logbook_refused(&self, code: &'static str, said: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, said, self.run_id()))
    }

    fn logbook_missing(&self, said: String) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_ENTRY, said, self.run_id()))
    }

    fn logbook_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(FAILED, why.to_string(), self.run_id()))
    }

    fn logbook_now(&self) -> u64 {
        self.now().epoch_millis().unwrap_or(0).max(0) as u64
    }

    /// The root of the repository a key names, where Fleet holds one.
    fn logbook_root_of(&self, key: &str) -> Option<String> {
        if key.is_empty() {
            return None;
        }
        self.repositories().every().iter().map(|one| one.root().to_string()).find(|root| key_of(root) == key)
    }

    /// The repository a root names: any Fleet holds, with a Manifest or without, since a Job runs on either
    /// and `/repositories` lists both.
    fn logbook_repository(&self, repository: &str) -> Result<String, Refusal> {
        match self.repositories().at(repository) {
            Some(held) => Ok(held.root().to_string()),
            None => Err(Refusal::NotFound(WireError::raised(
                crate::run_memory::NOT_SERVED,
                format!("Fleet does not serve {repository}"),
                self.run_id(),
            ))),
        }
    }

    /// Text as an entry keeps it: trimmed, and neither empty nor past [`MOST_CHARS`].
    fn logbook_text(&self, text: &str) -> Result<String, Refusal> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MOST_CHARS {
            return Err(self.logbook_refused(
                TEXT_REFUSED,
                format!("An entry says one or two lines, up to {MOST_CHARS} characters. Shorten the text and send it again."),
            ));
        }
        Ok(text.to_string())
    }

    /// The entry an id names, as the store keeps it.
    async fn logbook_held(&self, id: &str) -> Result<LogEntry, Refusal> {
        if is_folded(id) {
            return Err(self.logbook_refused(
                NOT_KEPT,
                format!("{id} comes from what Jobs ran or what this Mac lacks, and Armada rebuilds it on every read. Drop a run from the repository's runs instead."),
            ));
        }
        let missing = || self.logbook_missing(format!("No entry has the id {id}. It may have been forgotten."));
        let number: i64 = id.parse().map_err(|_| missing())?;
        self.store().lock().await.logbook_entry(number).map_err(|why| self.logbook_fault(why))?.ok_or_else(missing)
    }

    /// An entry as the wire reads it after an act.
    async fn logbook_entry_wired(&self, entry: LogEntry) -> LogbookEntry {
        let root = self.logbook_root_of(&entry.repository);
        let titles = self.logbook_titles(std::slice::from_ref(&entry)).await;
        wired(&Logged { entry, from: Source::Learnt }, root.as_deref(), &titles)
    }

    /// The Jobs' titles that entries cite, by id.
    async fn logbook_titles(&self, entries: &[LogEntry]) -> HashMap<String, String> {
        let mut titles = HashMap::new();
        let store = self.store().lock().await;
        for one in entries.iter().flat_map(|entry| entry.evidence.iter()) {
            if titles.contains_key(&one.session_id) {
                continue;
            }
            if let Some(title) = store.session(&one.session_id).ok().flatten().and_then(|session| session.title) {
                titles.insert(one.session_id.clone(), title);
            }
        }
        titles
    }

    /// Write the files after an act, as a Session's distil does. The habits and this Mac's go out
    /// beside a served repository's, any one when the entry is not a repository's.
    async fn logbook_mirrored(&self, key: &str) {
        let served: Vec<String> = self.repositories().served().iter().map(|one| one.root().to_string()).collect();
        let root = served.iter().find(|root| !key.is_empty() && key_of(root) == key).or(served.first());
        if let Some(root) = root {
            self.logbook_written(root).await;
        }
    }
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
    /// Whether a tidy is running, and what the last finished one did.
    fn logbook_tidy_state(&self) -> ipc::LogbookTidy {
        let distils = self.logbook_distils();
        let last = *distils.tidy_last.lock().expect("the last tidy is not held across a panic");
        ipc::LogbookTidy {
            running: distils.tidying.load(std::sync::atomic::Ordering::Acquire),
            last: last.map(|(merged, at, failed)| ipc::LogbookTidyLast { merged, at: instant(at), failed }),
        }
    }
}

impl<H, V, W> Logbook for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_logbook(
        &self,
        repository: Option<String>,
        section: Option<LogbookSection>,
        state: Option<LogbookState>,
    ) -> Result<ipc::Logbook, Refusal> {
        let root = repository.map(|repository| self.logbook_repository(&repository)).transpose()?;
        let mut all = self.logbook_read_in(root.as_deref()).await;
        all.retain(|one| section.is_none_or(|asked| section_of(one.entry.section) == asked));
        all.retain(|one| state.is_none_or(|asked| state_of(one.entry.state) == asked));
        let learnt: Vec<LogEntry> = all.iter().filter(|one| one.from == Source::Learnt).map(|one| one.entry.clone()).collect();
        let titles = self.logbook_titles(&learnt).await;
        Ok(ipc::Logbook { entries: all.iter().map(|one| wired(one, root.as_deref(), &titles)).collect(), tidy: self.logbook_tidy_state() })
    }

    async fn edit_logbook_entry(&self, entry_id: String, edit: EditLogbookEntry) -> Result<LogbookEntry, Refusal> {
        let text = self.logbook_text(&edit.text)?;
        let mut entry = self.logbook_held(&entry_id).await?;
        if entry.text != text {
            let was = std::mem::replace(&mut entry.text, text);
            entry.history.push(LogHistory { text: was, replaced_at: self.logbook_now(), why: LogRewrite::Person });
        }
        entry.edited_by_person = true;
        let kept = self.store().lock().await.save_logbook_entry(&entry).map_err(|why| self.logbook_fault(why))?;
        if kept.is_none() {
            return Err(self.logbook_refused(
                TEXT_FORGOTTEN,
                "You forgot that text. Teach it to bring it back.".to_string(),
            ));
        }
        self.logbook_mirrored(&entry.repository).await;
        Ok(self.logbook_entry_wired(entry).await)
    }

    async fn lock_logbook_entry(&self, entry_id: String, lock: LockLogbookEntry) -> Result<LogbookEntry, Refusal> {
        let mut entry = self.logbook_held(&entry_id).await?;
        let number = entry.id.unwrap_or_default();
        self.store().lock().await.lock_logbook_entry(number, lock.locked).map_err(|why| self.logbook_fault(why))?;
        entry.locked = lock.locked;
        self.logbook_mirrored(&entry.repository).await;
        Ok(self.logbook_entry_wired(entry).await)
    }

    async fn forget_logbook_entry(&self, entry_id: String) -> Result<ForgottenLogbookEntry, Refusal> {
        let entry = self.logbook_held(&entry_id).await?;
        let now = self.logbook_now();
        let taken = {
            let mut store = self.store().lock().await;
            let forgot = store
                .forget_logbook_entry(entry.id.unwrap_or_default(), now)
                .map_err(|why| self.logbook_fault(why))?;
            if !forgot {
                return Err(self.logbook_missing(format!("No entry has the id {entry_id}. It may have been forgotten.")));
            }
            // The mods a teaching made from this entry go with it, and under the same lock.
            crate::teaching::forget_effects(&self.mods_dir(), &entry_id)
        };
        if !taken.is_empty() {
            let _ = self.mods_announced().await;
        }
        self.logbook_mirrored(&entry.repository).await;
        Ok(ForgottenLogbookEntry { id: entry_id })
    }

    async fn tidy_logbook(self: Arc<Self>, tidy: TidyLogbook) -> Result<TidiedLogbook, Refusal> {
        self.logbook_tidy_started(tidy.repository).map_err(|why| self.logbook_fault(why))?;
        Ok(TidiedLogbook { running: true })
    }

    async fn teach_logbook_entry(&self, teach: TeachLogbookEntry) -> Result<LogbookEntry, Refusal> {
        let text = self.logbook_text(&teach.text)?;
        let section = section_in(teach.section);
        let root = match section {
            LogSection::Repo => {
                let named = teach.repository.ok_or_else(|| {
                    self.logbook_refused(REPOSITORY_MISSING, "A repository entry needs the repository it is for.".to_string())
                })?;
                Some(self.logbook_repository(&named)?)
            }
            LogSection::Habits | LogSection::Machine => None,
        };
        let key = root.as_deref().map(key_of).unwrap_or_default();
        let now = self.logbook_now();
        let evidence = teach.source.map(|from| LogEvidence {
            session_id: from.session_id,
            cites: vec![from.cite],
            how: LogHow::First,
            hard: false,
            at: now,
        });
        let entry = {
            let mut store = self.store().lock().await;
            let words = logbook_words(&text);
            let said = store
                .logbook_of(&key)
                .map_err(|why| self.logbook_fault(why))?
                .into_iter()
                .find(|one| one.section == section && one.repository == key && logbook_words(&one.text) == words);
            // Teaching what an entry already says stands that entry, as a new one would stand.
            let id = match said {
                Some(mut one) => {
                    one.state = LogState::Standing;
                    one.last_confirmed = now;
                    store.save_logbook_entry(&one).map_err(|why| self.logbook_fault(why))?;
                    one.id
                }
                None => Some(
                    store
                        .teach_logbook_entry(section, &key, kind_in(teach.kind), &text, &teach.paths, evidence, now)
                        .map_err(|why| self.logbook_fault(why))?,
                ),
            };
            store
                .logbook_entry(id.unwrap_or_default())
                .map_err(|why| self.logbook_fault(why))?
                .ok_or_else(|| self.logbook_fault("the entry was not kept"))?
        };
        self.logbook_mirrored(&key).await;
        Ok(self.logbook_entry_wired(entry).await)
    }
}
