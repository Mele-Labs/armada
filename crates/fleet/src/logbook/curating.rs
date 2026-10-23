//! Curate: Distil's answer merged into the Logbook by plain rules, with no model call.
//! `docs/concepts/logbook.md`.
//!
//! **An entry stands on evidence, not on one agent's say-so**: two distinct Sessions, or one where
//! a command failed and a different form then worked. A locked entry is never rewritten and never
//! fades; a person's text is kept until a Session contradicts it.

use std::collections::{BTreeSet, HashSet};

use store::{logbook_words, LogEntry, LogEvidence, LogHistory, LogHow, LogRewrite, LogSection, LogState};

use super::answer::{Claim, Distilled, Learnt};

/// The distinct Sessions that stand an entry.
pub(crate) const STANDS_AT: usize = 2;
/// How long a standing entry may go unconfirmed and unserved before it fades, with
/// [`FADES_AFTER_SESSIONS`] of its repository's Sessions distilled in that time.
pub(crate) const FADES_AFTER_MS: u64 = 30 * 24 * 60 * 60 * 1000;
pub(crate) const FADES_AFTER_SESSIONS: usize = 20;
/// How alike two texts' words must be for a new one to be read as an entry already held.
const ALIKE: f64 = 0.8;

/// The distil being curated: whose, for which repository's key, and when.
pub(crate) struct Distil<'a> {
    pub session: &'a str,
    pub repository: &'a str,
    pub now: u64,
}

/// What Curate asks of the store, so the rules read without one.
pub(crate) trait Held {
    /// Whether a text was forgotten for this section and repository.
    fn denied(&self, section: LogSection, repository: &str, text: &str) -> bool;
    /// How many of the section's Sessions were distilled after `since`.
    fn sessions_since(&self, section: LogSection, since: u64) -> usize;
}

/// The share of words two texts hold in common.
fn alike(one: &str, other: &str) -> f64 {
    let one: HashSet<String> = logbook_words(one).split(' ').map(str::to_string).collect();
    let other: HashSet<String> = logbook_words(other).split(' ').map(str::to_string).collect();
    let all = one.union(&other).count();
    if all == 0 {
        return 0.0;
    }
    one.intersection(&other).count() as f64 / all as f64
}

fn evidence(distil: &Distil<'_>, cites: &[String], how: LogHow, hard: bool) -> LogEvidence {
    LogEvidence { session_id: distil.session.to_string(), cites: cites.to_vec(), how, hard, at: distil.now }
}

/// Replace an entry's text, keeping the one it had. Nothing where the words are the same.
fn rewrite(entry: &mut LogEntry, text: &str, why: LogRewrite, now: u64) {
    if logbook_words(text) == logbook_words(&entry.text) {
        return;
    }
    let was = std::mem::replace(&mut entry.text, text.to_string());
    entry.history.push(LogHistory { text: was, replaced_at: now, why });
}

/// Merge an answer into `entries` and answer the places of those that changed: new ones are pushed
/// on the end, with no id.
pub(crate) fn curate(entries: &mut Vec<LogEntry>, answer: &Distilled, distil: &Distil<'_>, held: &dyn Held) -> BTreeSet<usize> {
    let mut changed = BTreeSet::new();
    let place = |entries: &[LogEntry], id: i64| entries.iter().position(|entry| entry.id == Some(id));
    let mut each = |claims: &[Claim], entries: &mut Vec<LogEntry>, act: &dyn Fn(&mut LogEntry, &Claim)| {
        for claim in claims {
            if let Some(at) = place(entries, claim.id) {
                act(&mut entries[at], claim);
                changed.insert(at);
            }
        }
    };
    each(&answer.confirmed, entries, &|entry, claim| {
        entry.evidence.push(evidence(distil, &claim.cites, LogHow::Confirmed, claim.hard));
        entry.last_confirmed = distil.now;
    });
    // A trip is a Session meeting what the entry said: it bears the entry out, and orders it first.
    each(&answer.tripped, entries, &|entry, claim| {
        entry.evidence.push(evidence(distil, &claim.cites, LogHow::Tripped, claim.hard));
        entry.last_confirmed = distil.now;
        entry.tripped += 1;
        match &claim.text {
            Some(text) if !entry.locked && !entry.edited_by_person => rewrite(entry, text, LogRewrite::Tripped, distil.now),
            _ => {}
        }
    });
    each(&answer.contradicted, entries, &|entry, claim| {
        entry.evidence.push(evidence(distil, &claim.cites, LogHow::Contradicted, claim.hard));
        match &claim.text {
            Some(text) if !entry.locked => {
                rewrite(entry, text, LogRewrite::Contradicted, distil.now);
                entry.edited_by_person = false;
            }
            _ => {}
        }
    });
    for learnt in &answer.learnt {
        if let Some(at) = learnt_into(entries, learnt, distil, held) {
            changed.insert(at);
        }
    }
    for at in changed.iter().copied() {
        stand(&mut entries[at]);
    }
    for (at, entry) in entries.iter_mut().enumerate() {
        if fade(entry, distil.now, held) {
            changed.insert(at);
        }
    }
    changed
}

/// A new entry, or a confirmation of one held that says the same. `None` for a forgotten text.
fn learnt_into(entries: &mut Vec<LogEntry>, learnt: &Learnt, distil: &Distil<'_>, held: &dyn Held) -> Option<usize> {
    let repository = match learnt.section {
        LogSection::Repo => distil.repository,
        LogSection::Machine | LogSection::Habits => "",
    };
    if held.denied(learnt.section, repository, &learnt.text) {
        return None;
    }
    let same = entries.iter().position(|entry| {
        entry.section == learnt.section && entry.repository == repository && alike(&entry.text, &learnt.text) >= ALIKE
    });
    if let Some(at) = same {
        let entry = &mut entries[at];
        entry.evidence.push(evidence(distil, &learnt.cites, LogHow::Confirmed, learnt.hard));
        entry.last_confirmed = distil.now;
        // Learnt again after it faded: it stands again on the rules, as a new one would.
        if entry.state == LogState::Faded {
            entry.state = LogState::Candidate;
        }
        return Some(at);
    }
    entries.push(LogEntry {
        id: None,
        section: learnt.section,
        repository: repository.to_string(),
        kind: learnt.kind,
        text: learnt.text.clone(),
        paths: learnt.paths.clone(),
        evidence: vec![evidence(distil, &learnt.cites, LogHow::First, learnt.hard)],
        first_seen: distil.now,
        last_confirmed: distil.now,
        last_served: None,
        state: LogState::Candidate,
        locked: false,
        edited_by_person: false,
        tripped: 0,
        history: Vec::new(),
    });
    Some(entries.len() - 1)
}

/// A candidate stands at [`STANDS_AT`] distinct Sessions, or on one Session's hard evidence.
pub(crate) fn stand(entry: &mut LogEntry) {
    let hard = entry.evidence.iter().any(|one| one.hard && one.how != LogHow::Contradicted);
    if entry.state == LogState::Candidate && (entry.confirmations() >= STANDS_AT || hard) {
        entry.state = LogState::Standing;
    }
}

/// Whether a standing entry fades now: neither confirmed nor served for [`FADES_AFTER_MS`], and
/// [`FADES_AFTER_SESSIONS`] of its repository's Sessions since. A locked entry never fades.
pub(crate) fn fade(entry: &mut LogEntry, now: u64, held: &dyn Held) -> bool {
    if entry.state != LogState::Standing || entry.locked {
        return false;
    }
    let used = entry.last_confirmed.max(entry.last_served.unwrap_or(0));
    let fades = now.saturating_sub(used) >= FADES_AFTER_MS && held.sessions_since(entry.section, used) >= FADES_AFTER_SESSIONS;
    if fades {
        entry.state = LogState::Faded;
    }
    fades
}
