//! What the Logbook serves a repository's next agent: what Curate stood, what run memory and the
//! machine note know, in one order, cut to a budget. `docs/concepts/logbook.md`.

use store::{LogEntry, LogKind, LogSection, LogState};

/// Where an entry served comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// Learnt from Sessions, and curated.
    Learnt,
    /// A run memory fact: read off the stream, so it stands as it is.
    Runs,
    /// A machine note fact, read off the stream likewise.
    Machine,
}

/// One entry of the Logbook as it is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Logged {
    pub entry: LogEntry,
    pub from: Source,
}

impl Logged {
    /// A fact run memory or the machine note keeps, as an entry: standing, with no Session behind it.
    pub(crate) fn folded(from: Source, kind: LogKind, text: String, at: u64) -> Logged {
        let section = match from {
            Source::Machine => LogSection::Machine,
            Source::Learnt | Source::Runs => LogSection::Repo,
        };
        Logged {
            entry: LogEntry {
                id: None,
                section,
                repository: String::new(),
                kind,
                text,
                paths: Vec::new(),
                evidence: Vec::new(),
                first_seen: at,
                last_confirmed: at,
                last_served: None,
                state: LogState::Standing,
                locked: false,
                edited_by_person: false,
                tripped: 0,
                history: Vec::new(),
            },
            from,
        }
    }

    /// How many Sessions bore it out. A folded fact counts as one: the stream showed it.
    pub(crate) fn borne_out(&self) -> usize {
        match self.from {
            Source::Learnt => self.entry.confirmations(),
            Source::Runs | Source::Machine => 1,
        }
    }

    /// Tripped first, then by how many Sessions bore it out, then the most recent.
    fn rank(&self) -> (u32, usize, u64) {
        (self.entry.tripped, self.borne_out(), self.entry.last_confirmed)
    }

    /// What it costs to serve, in tokens: about four characters each, and a few for the line.
    fn tokens(&self) -> usize {
        let paths: usize = self.entry.paths.iter().map(|path| path.len() + 2).sum();
        (self.entry.text.len() + paths).div_ceil(4) + 4
    }
}

/// Entries in the order they are served: [`Logged::rank`], highest first.
pub(crate) fn ordered(mut all: Vec<Logged>) -> Vec<Logged> {
    all.sort_by(|one, other| other.rank().cmp(&one.rank()));
    all
}

/// The entries to serve, in order, until the next would pass `budget` tokens. Only standing ones.
pub(crate) fn served(mut all: Vec<Logged>, budget: usize) -> Vec<Logged> {
    all.retain(|one| one.entry.state == LogState::Standing);
    let mut spent = 0;
    ordered(all)
        .into_iter()
        .take_while(|one| {
            spent += one.tokens();
            spent <= budget
        })
        .collect()
}
