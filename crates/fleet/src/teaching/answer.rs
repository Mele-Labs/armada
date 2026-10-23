//! The reader's answer, held to what Fleet and Bridge can use before a person sees it: a note that
//! fits its section and reach, a pattern that works in Bridge, a rule for a notice that exists, a
//! tweak the stylesheet checks accept and a layout the registry accepts.
//!
//! **What the source cannot make is dropped, not refused**: the model is told what may be proposed,
//! and a part outside that is a model's slip and not the person's mistake.

use std::collections::BTreeMap;

use ipc::{
    AddressFound, Does, FleetRule, LogbookKind, LogbookSection, ProposedAddress, RuleScope, TeachingLayout, TeachingNote,
    TeachingProposal, TeachingReach, TeachingSourceKind, TeachingWords, TweakChange,
};
use serde::{Deserialize, Serialize};

use super::{link, tokens};
use crate::mods::{fleet_rules, stylesheet};

/// The most characters of a note, and its most lines: what the Logbook keeps an entry to.
const MOST_CHARS: usize = 400;
const MOST_LINES: usize = 2;
const MOST_NAME: usize = 60;
const MOST_TWEAKS: usize = 12;

/// Why an answer is not a proposal.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unfit {
    /// It is not an object with the fields asked for. The call failed.
    Unread(String),
    Words(String),
    Note(String),
    Rule(String),
    Tweak(String),
    Layout(String),
    /// It holds nothing the source can make.
    Nothing,
}

/// What an answer is read against.
pub(crate) struct Context<'a> {
    /// The selected words, or what a pressed thing shows.
    pub(crate) subject: &'a str,
    pub(crate) kind: TeachingSourceKind,
    pub(crate) reach: TeachingReach,
    /// The notice a `notice` source was pressed on.
    pub(crate) notice: Option<&'a str>,
    pub(crate) session: Option<&'a str>,
    /// The Manifest id of the repository the teaching is in.
    pub(crate) manifest: Option<&'a str>,
}

/// What a source may propose.
pub(crate) struct Allowed {
    pub(crate) note: bool,
    pub(crate) words: bool,
    pub(crate) fleet: bool,
    pub(crate) tweak: bool,
    pub(crate) layout: bool,
}

pub(crate) fn allowed(kind: TeachingSourceKind) -> Allowed {
    let bridge = kind.is_bridge();
    Allowed {
        note: !bridge,
        words: !bridge && kind != TeachingSourceKind::Notice,
        fleet: kind == TeachingSourceKind::Notice,
        tweak: bridge,
        layout: bridge,
    }
}

#[derive(Deserialize)]
struct Said {
    #[serde(default)]
    note: Option<SaidNote>,
    #[serde(default)]
    words: Option<SaidWords>,
    #[serde(default)]
    fleet: Option<SaidFleet>,
    #[serde(default)]
    tweak: Option<Vec<SaidTweak>>,
    #[serde(default)]
    layout: Option<SaidLayout>,
}

#[derive(Deserialize)]
struct SaidNote {
    section: LogbookSection,
    kind: LogbookKind,
    text: String,
}

#[derive(Deserialize)]
struct SaidWords {
    name: String,
    pattern: String,
    #[serde(default)]
    address: Option<String>,
}

#[derive(Deserialize)]
struct SaidFleet {
    does: Does,
    #[serde(rename = "where")]
    place: SaidPlace,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SaidPlace {
    Job,
    Repo,
    Everywhere,
}

#[derive(Deserialize)]
struct SaidTweak {
    token: String,
    now: String,
}

#[derive(Deserialize)]
struct SaidLayout {
    name: String,
    regions: BTreeMap<String, SaidRegion>,
}

#[derive(Deserialize, Serialize)]
struct SaidRegion {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    order: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    first: Option<String>,
}

#[derive(Serialize)]
struct LayoutFile<'a> {
    version: u8,
    #[serde(flatten)]
    regions: &'a BTreeMap<String, SaidRegion>,
}

/// The proposal an answer makes, or why it is not one.
pub(crate) fn read(said: &str, at: &Context) -> Result<TeachingProposal, Unfit> {
    let json = crate::review_runs::object_in(said).map_err(Unfit::Unread)?;
    let said: Said = crate::review_runs::said(json).map_err(Unfit::Unread)?;
    let may = allowed(at.kind);
    let proposal = TeachingProposal {
        note: said.note.filter(|_| may.note).map(|one| note(one, at.reach)).transpose()?,
        words: said.words.filter(|_| may.words).map(|one| words(one, at.subject)).transpose()?,
        fleet: said.fleet.filter(|_| may.fleet).map(|one| rule(one, at)).transpose()?,
        tweak: said.tweak.filter(|_| may.tweak).map(|many| tweaks(many)).transpose()?.unwrap_or_default(),
        layout: said.layout.filter(|_| may.layout).map(layout).transpose()?,
    };
    let empty = proposal.note.is_none()
        && proposal.words.is_none()
        && proposal.fleet.is_none()
        && proposal.tweak.is_empty()
        && proposal.layout.is_none();
    if empty {
        return Err(Unfit::Nothing);
    }
    Ok(proposal)
}

/// A note as a Logbook entry takes it, for the reach the person chose.
pub(crate) fn note_fault(section: LogbookSection, kind: LogbookKind, text: &str, reach: TeachingReach) -> Option<String> {
    let fits = match section {
        LogbookSection::Repo => matches!(kind, LogbookKind::Command | LogbookKind::Gotcha | LogbookKind::Convention),
        LogbookSection::Machine => kind == LogbookKind::Environment,
        LogbookSection::Habits => kind == LogbookKind::Habit,
    };
    if !fits {
        return Some("the kind does not belong in that section".to_string());
    }
    if (section == LogbookSection::Repo) != (reach == TeachingReach::Repository) {
        return Some("the section does not match where the person said it applies".to_string());
    }
    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    if text.trim().is_empty() || lines > MOST_LINES || text.trim().chars().count() > MOST_CHARS {
        return Some(format!("the text is empty, over {MOST_LINES} lines or over {MOST_CHARS} characters"));
    }
    None
}

fn note(said: SaidNote, reach: TeachingReach) -> Result<TeachingNote, Unfit> {
    match note_fault(said.section, said.kind, &said.text, reach) {
        Some(why) => Err(Unfit::Note(why)),
        None => Ok(TeachingNote { section: said.section, kind: said.kind, text: said.text.trim().to_string() }),
    }
}

/// Why a words rule cannot work in Bridge, where it cannot.
pub(crate) fn words_fault(name: &str, pattern: &str, address: Option<&str>, example: &str) -> Option<String> {
    if name.trim().is_empty() || name.chars().count() > MOST_NAME || name.chars().any(char::is_control) {
        return Some(format!("the name is 1 to {MOST_NAME} characters on one line"));
    }
    if example.trim().is_empty() {
        return Some("there are no selected words to match".to_string());
    }
    link::pattern_fault(pattern, example).or_else(|| {
        address.and_then(|address| {
            if address.contains("{match}") {
                link::address_fault(address)
            } else {
                Some("the address has no {match} for the matched words to go in".to_string())
            }
        })
    })
}

fn words(said: SaidWords, subject: &str) -> Result<TeachingWords, Unfit> {
    let (name, pattern) = (said.name.trim(), said.pattern.trim());
    let address = said.address.as_deref().map(str::trim).filter(|one| !one.is_empty());
    match words_fault(name, pattern, address, subject) {
        Some(why) => Err(Unfit::Words(why)),
        None => Ok(TeachingWords {
            name: name.to_string(),
            pattern: pattern.to_string(),
            example: subject.to_string(),
            address: address.map(|template| ProposedAddress { template: template.to_string(), found: AddressFound::Person }),
        }),
    }
}

fn rule(said: SaidFleet, at: &Context) -> Result<FleetRule, Unfit> {
    let notice = at.notice.ok_or_else(|| Unfit::Rule("a rule needs the notice it was pressed on".to_string()))?;
    if !fleet_rules::is_notice_word(notice) {
        return Err(Unfit::Rule(format!("`{notice}` is not a notice Fleet sends")));
    }
    let scope = match said.place {
        SaidPlace::Job => RuleScope::Job(
            at.session.ok_or_else(|| Unfit::Rule("a rule for one Job needs the Job it was taught in".to_string()))?.to_string(),
        ),
        SaidPlace::Repo => RuleScope::Repo(
            at.manifest.ok_or_else(|| Unfit::Rule("a rule for one repository needs the repository".to_string()))?.to_string(),
        ),
        SaidPlace::Everywhere => RuleScope::Everywhere,
    };
    Ok(FleetRule { notice: notice.to_string(), does: said.does, scope })
}

/// The `tweak.css` a list of changes makes.
pub(crate) fn tweak_css(changes: &[(String, String)]) -> String {
    let lines: String = changes.iter().map(|(token, now)| format!("  {token}: {now};\n")).collect();
    format!(":root {{\n{lines}}}\n")
}

/// Why these changes cannot be a tweak, where they cannot. **The stylesheet checks a theme gets.**
pub(crate) fn tweak_fault(changes: &[(String, String)]) -> Option<String> {
    if changes.is_empty() || changes.len() > MOST_TWEAKS {
        return Some(format!("a tweak sets 1 to {MOST_TWEAKS} tokens"));
    }
    let unknown = changes.iter().find(|(token, _)| tokens::named(token).is_none());
    if let Some((token, _)) = unknown {
        return Some(format!("`{token}` is not a design token"));
    }
    stylesheet::problems_in(&tweak_css(changes), "tweak.css").into_iter().next()
}

fn tweaks(said: Vec<SaidTweak>) -> Result<Vec<TweakChange>, Unfit> {
    let changes: Vec<(String, String)> = said.into_iter().map(|one| (one.token.trim().to_string(), one.now.trim().to_string())).collect();
    if let Some(why) = tweak_fault(&changes) {
        return Err(Unfit::Tweak(why));
    }
    Ok(changes
        .into_iter()
        .filter_map(|(token, now)| {
            let known = tokens::named(&token)?;
            Some(TweakChange {
                used_in: tokens::aliases_of(&token),
                note: known.note.clone(),
                was: known.value.clone(),
                token,
                now,
            })
        })
        .collect())
}

/// The `layout.json` text a set of regions makes, or why it is not one the registry accepts.
fn layout_text(regions: &BTreeMap<String, SaidRegion>) -> Result<String, String> {
    let known = ipc::layout::regions();
    if let Some(bad) = regions.keys().find(|key| !known.contains(key)) {
        return Err(format!("`{bad}` is not a region Bridge arranges"));
    }
    let text = ipc::encode(&LayoutFile { version: 1, regions }).map_err(|why| why.to_string())?;
    match ipc::layout::problems(&text).into_iter().next() {
        Some(why) => Err(why),
        None if regions.is_empty() => Err("the layout changes nothing".to_string()),
        None => Ok(text),
    }
}

fn layout(said: SaidLayout) -> Result<TeachingLayout, Unfit> {
    let name = said.name.trim();
    if name.is_empty() || name.chars().count() > MOST_NAME || name.chars().any(char::is_control) {
        return Err(Unfit::Layout(format!("the name is 1 to {MOST_NAME} characters on one line")));
    }
    let layout = layout_text(&said.regions).map_err(Unfit::Layout)?;
    Ok(TeachingLayout { name: name.to_string(), layout })
}
