//! The reader's answer, held to what Bridge can use before a person sees it: a link that could
//! work, and a note that fits the section it names and the reach the person chose.

use ipc::{NoteKind, NoteSection, TeachingLink, TeachingNote, TeachingProposal, TeachingReach};
use serde::Deserialize;

/// The most characters of a note, and its most lines: what the Logbook keeps an entry to.
const MOST_CHARS: usize = 400;
const MOST_LINES: usize = 2;

/// Why an answer is not a proposal.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unfit {
    /// It is not an object with the fields asked for. The call failed.
    Unread(String),
    /// The link cannot work.
    Link(String),
    /// The note does not fit.
    Note(String),
    /// It holds neither.
    Nothing,
}

#[derive(Deserialize)]
struct Said {
    #[serde(default)]
    link: Option<SaidLink>,
    #[serde(default)]
    note: Option<SaidNote>,
}

#[derive(Deserialize)]
struct SaidLink {
    pattern: String,
    address: String,
}

#[derive(Deserialize)]
struct SaidNote {
    section: NoteSection,
    kind: NoteKind,
    text: String,
}

/// The proposal an answer makes for `words`, or why it is not one.
pub(crate) fn read(said: &str, words: &str, reach: TeachingReach) -> Result<TeachingProposal, Unfit> {
    let json = crate::review_runs::object_in(said).map_err(Unfit::Unread)?;
    let said: Said = crate::review_runs::said(json).map_err(Unfit::Unread)?;
    let link = said
        .link
        .map(|one| {
            let (pattern, address) = (one.pattern.trim(), one.address.trim());
            match super::link::fault(pattern, address, words) {
                Some(why) => Err(Unfit::Link(why)),
                None => Ok(TeachingLink { pattern: pattern.to_string(), address: address.to_string(), example: words.to_string() }),
            }
        })
        .transpose()?;
    let note = said.note.map(|one| note(one, reach)).transpose()?;
    if link.is_none() && note.is_none() {
        return Err(Unfit::Nothing);
    }
    Ok(TeachingProposal { link, note })
}

fn note(said: SaidNote, reach: TeachingReach) -> Result<TeachingNote, Unfit> {
    let fits = match said.section {
        NoteSection::Repo => matches!(said.kind, NoteKind::Command | NoteKind::Gotcha | NoteKind::Convention),
        NoteSection::Machine => said.kind == NoteKind::Environment,
        NoteSection::Habits => said.kind == NoteKind::Habit,
    };
    if !fits {
        return Err(Unfit::Note("the kind does not belong in that section".to_string()));
    }
    let in_repository = said.section == NoteSection::Repo;
    if in_repository != (reach == TeachingReach::Repository) {
        return Err(Unfit::Note("the section does not match where the person said it applies".to_string()));
    }
    let text = said.text.trim();
    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    if text.is_empty() || lines > MOST_LINES || text.chars().count() > MOST_CHARS {
        return Err(Unfit::Note(format!("the text is empty, over {MOST_LINES} lines or over {MOST_CHARS} characters")));
    }
    Ok(TeachingNote { section: said.section, kind: said.kind, text: text.to_string() })
}
