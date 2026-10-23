//! Distil's answer, read back and held to the record: a claim citing no row the record holds is
//! dropped, and so is one naming an entry it was not offered. `docs/concepts/logbook.md`.
//!
//! **Read leniently, item by item**, as a retro is: one item the model spelt wrong costs that item,
//! never the answer.

use std::collections::BTreeSet;

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};
use store::{LogKind, LogSection};

/// The most characters of an entry's text, and its most lines: one or two lines saying what to do.
pub(crate) const MOST_CHARS: usize = 400;
const MOST_LINES: usize = 2;

/// One command a Session ran and how it came back, as the record lays it: `ran:n`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Ran {
    pub cite: String,
    pub command: String,
    pub failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<i32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub said: String,
}

/// What an answer may name: the record's cites, the entries offered and the commands it ran.
pub(crate) struct Known<'a> {
    pub cites: &'a BTreeSet<String>,
    pub ids: &'a BTreeSet<i64>,
    pub ran: &'a [Ran],
}

/// A claim about an entry the Logbook holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Claim {
    pub id: i64,
    /// The entry as it should read: required to contradict, optional to trip.
    pub text: Option<String>,
    pub cites: Vec<String>,
    pub hard: bool,
}

/// Something the record shows that no entry says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Learnt {
    pub section: LogSection,
    pub kind: LogKind,
    pub text: String,
    pub paths: Vec<String>,
    pub cites: Vec<String>,
    pub hard: bool,
}

/// Distil's answer, every claim held to the record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Distilled {
    pub confirmed: Vec<Claim>,
    pub contradicted: Vec<Claim>,
    pub tripped: Vec<Claim>,
    pub learnt: Vec<Learnt>,
}

// What the model's JSON is read into.
#[derive(Deserialize)]
struct Said {
    #[serde(default, deserialize_with = "lenient")]
    confirmed: Vec<SaidClaim>,
    #[serde(default, deserialize_with = "lenient")]
    contradicted: Vec<SaidClaim>,
    #[serde(default, deserialize_with = "lenient")]
    tripped: Vec<SaidClaim>,
    #[serde(default, rename = "new", deserialize_with = "lenient")]
    learnt: Vec<SaidNew>,
}

#[derive(Deserialize)]
struct SaidClaim {
    id: i64,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Deserialize)]
struct SaidNew {
    section: String,
    kind: String,
    text: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OrNot<T> {
    Read(T),
    Unread(IgnoredAny),
}

pub(super) fn lenient<'de, D: Deserializer<'de>, T: Deserialize<'de>>(input: D) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<OrNot<T>>>::deserialize(input)?
        .unwrap_or_default()
        .into_iter()
        .filter_map(|one| match one {
            OrNot::Read(item) => Some(item),
            OrNot::Unread(_) => None,
        })
        .collect())
}

/// An entry's text as it may be kept: trimmed, one or two lines, not too long. `None` otherwise.
pub(super) fn written(text: &str) -> Option<String> {
    let text = text.trim();
    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    (!text.is_empty() && lines <= MOST_LINES && text.chars().count() <= MOST_CHARS).then(|| text.to_string())
}

/// The section and kind a new entry names, where they go together.
pub(super) fn placed(section: &str, kind: &str) -> Option<(LogSection, LogKind)> {
    let section = LogSection::of_text(section)?;
    let kind = LogKind::of_text(kind)?;
    let fits = match section {
        LogSection::Repo => matches!(kind, LogKind::Command | LogKind::Gotcha | LogKind::Convention),
        LogSection::Machine => kind == LogKind::Environment,
        LogSection::Habits => kind == LogKind::Habit,
    };
    fits.then_some((section, kind))
}

/// Whether cites show a command that failed and a different one that then worked: the one session
/// evidence that stands an entry alone.
pub(crate) fn hard(cites: &[String], ran: &[Ran]) -> bool {
    let cited: Vec<(usize, &Ran)> =
        ran.iter().enumerate().filter(|(_, one)| cites.iter().any(|cite| *cite == one.cite)).collect();
    cited.iter().any(|(at, failed)| {
        failed.failed
            && cited.iter().any(|(later, worked)| later > at && !worked.failed && worked.command != failed.command)
    })
}

impl Known<'_> {
    fn held(&self, cites: Vec<String>) -> Vec<String> {
        let mut out: Vec<String> = cites.into_iter().filter(|cite| self.cites.contains(cite)).collect();
        out.sort();
        out.dedup();
        out
    }

    fn claim(&self, said: SaidClaim, needs_text: bool) -> Option<Claim> {
        let cites = self.held(said.evidence);
        let text = said.text.as_deref().and_then(written);
        let ok = !cites.is_empty() && self.ids.contains(&said.id) && (!needs_text || text.is_some());
        ok.then(|| Claim { id: said.id, text, hard: hard(&cites, self.ran), cites })
    }

    fn learnt(&self, said: SaidNew) -> Option<Learnt> {
        let (section, kind) = placed(&said.section, &said.kind)?;
        let cites = self.held(said.evidence);
        // A habit is the person's: only a message of theirs shows one.
        let shown = match section {
            LogSection::Habits => cites.iter().any(|cite| cite.starts_with("correction:")),
            _ => !cites.is_empty(),
        };
        let paths = said.paths.into_iter().map(|path| path.trim().to_string()).filter(|path| !path.is_empty()).collect();
        shown.then_some(())?;
        Some(Learnt { section, kind, text: written(&said.text)?, paths, hard: hard(&cites, self.ran), cites })
    }
}

/// The claims an answer makes that the record holds up.
pub(crate) fn read(said: &str, known: &Known<'_>) -> Result<Distilled, String> {
    let json = crate::review_runs::object_in(said)?;
    let said: Said = crate::review_runs::said(json)?;
    Ok(Distilled {
        confirmed: said.confirmed.into_iter().filter_map(|one| known.claim(one, false)).collect(),
        contradicted: said.contradicted.into_iter().filter_map(|one| known.claim(one, true)).collect(),
        tripped: said.tripped.into_iter().filter_map(|one| known.claim(one, false)).collect(),
        learnt: said.learnt.into_iter().filter_map(|one| known.learnt(one)).collect(),
    })
}
