//! The Logbook, as the fake answers it: an empty one, and each act answered with what it was given.

use ipc::{
    EditLogbookEntry, Instant, Logbook as LogbookRead, LogbookEntry, ForgottenLogbookEntry, LogbookKind, LogbookSection,
    LogbookState, LockLogbookEntry, TeachLogbookEntry, TidiedLogbook, TidyLogbook,
};

use super::FakeDaemon;
use crate::{Logbook, Refusal};

fn entry(id: &str, section: LogbookSection, kind: LogbookKind, text: &str, paths: Vec<String>) -> LogbookEntry {
    LogbookEntry {
        id: id.to_string(),
        section,
        repository: None,
        kind,
        text: text.to_string(),
        paths,
        evidence: Vec::new(),
        confirmed_by: 0,
        first_seen: Instant::carried("2000-01-01T00:00:00.000Z"),
        last_confirmed: Instant::carried("2000-01-01T00:00:00.000Z"),
        last_served: None,
        last_tripped: None,
        state: LogbookState::Standing,
        locked: false,
        edited: false,
        history: Vec::new(),
    }
}

impl Logbook for FakeDaemon {
    async fn get_logbook(
        &self,
        _repository: Option<String>,
        _section: Option<LogbookSection>,
        _state: Option<LogbookState>,
    ) -> Result<LogbookRead, Refusal> {
        Ok(LogbookRead { entries: Vec::new() })
    }

    async fn edit_logbook_entry(&self, entry_id: String, edit: EditLogbookEntry) -> Result<LogbookEntry, Refusal> {
        Ok(LogbookEntry { edited: true, ..entry(&entry_id, LogbookSection::Habits, LogbookKind::Habit, &edit.text, Vec::new()) })
    }

    async fn lock_logbook_entry(&self, entry_id: String, lock: LockLogbookEntry) -> Result<LogbookEntry, Refusal> {
        Ok(LogbookEntry { locked: lock.locked, ..entry(&entry_id, LogbookSection::Habits, LogbookKind::Habit, "", Vec::new()) })
    }

    async fn forget_logbook_entry(&self, entry_id: String) -> Result<ForgottenLogbookEntry, Refusal> {
        Ok(ForgottenLogbookEntry { id: entry_id })
    }

    async fn tidy_logbook(&self, tidy: TidyLogbook) -> Result<TidiedLogbook, Refusal> {
        Ok(TidiedLogbook { merged: tidy.repository.map_or(0, |root| root.len() as u32) })
    }

    async fn teach_logbook_entry(&self, teach: TeachLogbookEntry) -> Result<LogbookEntry, Refusal> {
        Ok(entry("1", teach.section, teach.kind, &teach.text, teach.paths))
    }
}
