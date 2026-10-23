//! Keeping a teaching: the note becomes a Logbook entry through `teach_logbook_entry`, with the
//! Session and the message it was taught from as its evidence, and each part that changes something
//! becomes a mod on this machine, switched on.
//!
//! **Everything is checked before anything is written**, so a part that cannot work leaves nothing
//! behind. A pattern that is already a Words mod gains an address and is not made twice.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Logbook, Refusal};
use ipc::{
    AddressFound, Does, FleetFile, FleetRule, LogbookSection, ModKind, SaveTeaching, SavedTeaching, TeachLogbookEntry,
    TeachSource, TeachingLayout, TeachingReach, TeachingSource, TeachingWords, WireError, WordsAddress, WordsFile,
};

use super::finding::Wanted;
use super::{answer, link, INPUT_REFUSED, LAYOUT_REFUSED, NOTE_REFUSED, RULE_REFUSED, TWEAK_REFUSED, WORDS_REFUSED};
use crate::daemon::Fleet;
use crate::mods::scaffold::{self, Taught};
use crate::mods::{self, fleet_rules, manifest, slug_problem, words};

/// The longest a name made from a title is, leaving room for a `-2` after it.
const MOST_SLUG: usize = 34;

/// What a save writes, every part checked.
struct Plan {
    words: Option<(TeachingWords, String)>,
    fleet: Option<FleetRule>,
    tweak: Vec<(String, String)>,
    layout: Option<TeachingLayout>,
    taught_on: Option<String>,
    entry: Option<String>,
}

impl Plan {
    /// The text of a mod's file, with where the teaching came from.
    fn taught<'a>(&'a self, text: &'a str) -> Taught<'a> {
        Taught { text, taught_on: self.taught_on.as_deref(), entry: self.entry.as_deref() }
    }
}

/// A name made of `text`: lowercase letters and digits, joined by `-`.
fn slug(text: &str, fallback: &str) -> String {
    let mut out = String::new();
    for one in text.chars().flat_map(char::to_lowercase) {
        if one.is_ascii_alphanumeric() {
            out.push(one);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(MOST_SLUG);
    let out = out.trim_end_matches('-');
    if out.is_empty() { fallback.to_string() } else { out.to_string() }
}

/// `base`, or `base-2`, `base-3` and on, whichever no folder has and a mod may be called.
fn unique(mods_dir: &Path, base: &str) -> Result<String, String> {
    let base = if slug_problem(base).is_some() { format!("{base}-mod") } else { base.to_string() };
    (1..100)
        .map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") })
        .find(|name| slug_problem(name).is_none() && std::fs::symlink_metadata(mods_dir.join(name)).is_err())
        .ok_or_else(|| format!("every name from `{base}` to `{base}-99` is taken"))
}

/// A line of text as a mod description or `taught_on` keeps it: one line, at most `most` characters.
fn one_line(text: &str, most: usize) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    flat.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(most).collect()
}

fn does_word(does: Does) -> &'static str {
    match does {
        Does::Tell => "tell",
        Does::Quiet => "quiet",
        Does::Batch => "batch",
        Does::Call => "call",
    }
}

/// Whether `file` is the Words mod for `pattern`.
fn is_for(file: &WordsFile, pattern: &str) -> bool {
    file.pattern == pattern
}

/// The address `file` gains for `key`: replaced where it has one.
fn with_address(mut file: WordsFile, key: &str, template: &str, found: AddressFound) -> WordsFile {
    file.address.retain(|one| one.repository != key);
    file.address.push(WordsAddress { repository: key.to_string(), template: template.to_string(), found });
    file
}

/// Give the Words mod `name` an address for `key`, a Manifest id or `*`. **Without `replace` an
/// address the mod already has for `key` stays**, so what a person wrote is never undone by what
/// was found. Whether the mod now holds `template` for `key`.
pub(crate) fn address_given(
    mods_dir: &Path,
    name: &str,
    key: &str,
    template: &str,
    found: AddressFound,
    replace: bool,
) -> Result<bool, String> {
    if let Some(why) = link::address_fault(template) {
        return Err(why);
    }
    if !template.contains("{match}") {
        return Err("the address has no {match} for the matched words to go in".to_string());
    }
    let Some(file) = words_file(mods_dir, name) else {
        return Err(format!("`{name}` is not a Words mod that is valid"));
    };
    if !replace && file.address.iter().any(|one| one.repository == key) {
        return Ok(false);
    }
    let label = file.name.clone();
    let text = toml::to_string(&with_address(file, key, template, found)).map_err(|why| why.to_string())?;
    std::fs::write(mods_dir.join(name).join("words.toml"), text).map_err(|why| format!("words.toml could not be written: {why}"))?;
    // The file is what counts; a history that would not take it is no reason to refuse.
    let _ = scaffold::committed(&mods_dir.join(name), "words.toml", &format!("Give {label} an address for {key}"));
    Ok(true)
}

/// The Words file of the mod `name`, where it is a valid Words mod. Switched off or not.
pub(crate) fn words_file(mods_dir: &Path, name: &str) -> Option<WordsFile> {
    match mods::examine(mods_dir, name).files?.payload {
        mods::Payload::Words(text) => words::read(&text).ok(),
        _ => None,
    }
}

/// Write every part to `mods_dir`. The names written or added to, in order.
fn write(mods_dir: &Path, switches: &BTreeMap<String, bool>, plan: &Plan) -> Result<Vec<String>, String> {
    let mut named = Vec::new();
    let refuse = |why: scaffold::Refused| match why {
        scaffold::Refused::Unacceptable(why) | scaffold::Refused::Failed(why) => why,
    };
    if let Some((one, key)) = &plan.words {
        let held = mods::kept(mods_dir, switches, ModKind::Words)
            .into_iter()
            .find_map(|(name, text)| words::read(&text).ok().filter(|file| is_for(file, &one.pattern)).map(|file| (name, file)));
        match (held, one.address.as_ref()) {
            (Some((name, file)), Some(carried)) => {
                // What the mod already opens this repository at is not written again, as a copy.
                let has = |key: &str| file.address.iter().find(|one| one.repository == key);
                let same = has(key).or_else(|| has("*")).is_some_and(|one| one.template == carried.template);
                if !same {
                    address_given(mods_dir, &name, key, &carried.template, carried.found, true)?;
                }
                named.push(name);
            }
            (Some((name, _)), None) => named.push(name),
            (None, address) => {
                let file = WordsFile {
                    name: one.name.clone(),
                    pattern: one.pattern.clone(),
                    example: one.example.clone(),
                    address: address
                        .map(|carried| WordsAddress { repository: key.clone(), template: carried.template.clone(), found: carried.found })
                        .into_iter()
                        .collect(),
                };
                let text = toml::to_string(&file).map_err(|why| why.to_string())?;
                let name = unique(mods_dir, &slug(&one.name, "words"))?;
                let description = one_line(&format!("Links {}", one.name), manifest::MOST_DESCRIPTION);
                scaffold::make_taught(mods_dir, &name, Some(&description), ModKind::Words, Some(&plan.taught(&text))).map_err(refuse)?;
                named.push(name);
            }
        }
    }
    if let Some(rule) = &plan.fleet {
        let text = toml::to_string(&FleetFile { rule: vec![rule.clone()] }).map_err(|why| why.to_string())?;
        let name = unique(mods_dir, &slug(&format!("fleet {}", rule.notice), "fleet"))?;
        let place = String::from(rule.scope.clone());
        let description = one_line(&format!("{}: {} {place}", rule.notice, does_word(rule.does)), manifest::MOST_DESCRIPTION);
        scaffold::make_taught(mods_dir, &name, Some(&description), ModKind::Fleet, Some(&plan.taught(&text))).map_err(refuse)?;
        named.push(name);
    }
    if let Some((first, _)) = plan.tweak.first() {
        let text = answer::tweak_css(&plan.tweak);
        let name = unique(mods_dir, &slug(&format!("tweak {first}"), "tweak"))?;
        let tokens: Vec<&str> = plan.tweak.iter().map(|(token, _)| token.as_str()).collect();
        let description = one_line(&format!("Sets {}", tokens.join(", ")), manifest::MOST_DESCRIPTION);
        scaffold::make_taught(mods_dir, &name, Some(&description), ModKind::Tweak, Some(&plan.taught(&text))).map_err(refuse)?;
        named.push(name);
    }
    if let Some(one) = &plan.layout {
        let name = unique(mods_dir, &slug(&one.name, "layout"))?;
        let description = one_line(&one.name, manifest::MOST_DESCRIPTION);
        scaffold::make_taught(mods_dir, &name, Some(&description), ModKind::Layout, Some(&plan.taught(&one.layout))).map_err(refuse)?;
        named.push(name);
    }
    Ok(named)
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
    fn teaching_refused(&self, code: &'static str, why: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why.into(), self.run_id()))
    }

    /// Every part of `save` held to what `read_teaching` holds a proposal to. The plan to write, with
    /// no entry yet.
    fn teaching_planned(&self, save: &SaveTeaching, manifest: Option<&str>) -> Result<Plan, Refusal> {
        let proposal = &save.proposal;
        if let Some(note) = &proposal.note {
            if let Some(why) = answer::note_fault(note.section, note.kind, &note.text, save.reach) {
                return Err(self.teaching_refused(NOTE_REFUSED, why));
            }
        }
        if let Some(one) = &proposal.words {
            if let Some(why) = answer::words_fault(&one.name, &one.pattern, one.address.as_ref().map(|one| one.template.as_str()), &one.example) {
                return Err(self.teaching_refused(WORDS_REFUSED, why));
            }
        }
        if let Some(rule) = &proposal.fleet {
            let rule = &rule.rule;
            if !fleet_rules::is_notice_word(&rule.notice) {
                return Err(self.teaching_refused(RULE_REFUSED, format!("`{}` is not a notice Fleet sends", rule.notice)));
            }
            let taught_on = save.source.as_ref().filter(|one| one.kind == ipc::TeachingSourceKind::Notice).map(|one| one.text.trim());
            if let Some(why) = fleet_rules::rule_fault(rule, taught_on) {
                return Err(self.teaching_refused(RULE_REFUSED, why));
            }
        }
        let tweak: Vec<(String, String)> =
            proposal.tweak.iter().map(|one| (one.token.trim().to_string(), one.now.trim().to_string())).collect();
        if !tweak.is_empty() {
            if let Some(why) = answer::tweak_fault(&tweak) {
                return Err(self.teaching_refused(TWEAK_REFUSED, why));
            }
        }
        if let Some(one) = &proposal.layout {
            let fault = ipc::layout::problems(&one.layout).into_iter().next().or_else(|| one.name.trim().is_empty().then(|| "the name is empty".to_string()));
            if let Some(why) = fault {
                return Err(self.teaching_refused(LAYOUT_REFUSED, why));
            }
        }
        Ok(Plan {
            words: proposal.words.clone().map(|one| (one, manifest.unwrap_or("*").to_string())),
            fleet: proposal.fleet.as_ref().map(|one| one.rule.clone()),
            tweak,
            layout: proposal.layout.clone(),
            taught_on: save.source.as_ref().map(taught_on).filter(|text| !text.is_empty()),
            entry: None,
        })
    }

    /// `save_teaching`. The Logbook entry is made first, so a mod from a Job records it; a part that
    /// is refused is refused before either is written.
    pub(crate) async fn teaching_saved(self: Arc<Self>, save: SaveTeaching) -> Result<SavedTeaching, Refusal> {
        let proposal = &save.proposal;
        if proposal.note.is_none()
            && proposal.words.is_none()
            && proposal.fleet.is_none()
            && proposal.tweak.is_empty()
            && proposal.layout.is_none()
        {
            return Err(self.teaching_refused(INPUT_REFUSED, "There is nothing to keep."));
        }
        let root = save.repository.as_deref().map(|named| self.run_repository(named)).transpose()?;
        if save.reach == TeachingReach::Repository && root.is_none() {
            return Err(self.teaching_refused(INPUT_REFUSED, "A teaching for one repository needs the repository."));
        }
        // A rule for a repository is held to the repository's Manifest id.
        let manifest = root.as_deref().and_then(|root| self.manifest_id_at(root));
        let mut plan = self.teaching_planned(&save, manifest.as_deref())?;
        let entry = match &save.proposal.note {
            Some(note) => {
                let taught = TeachLogbookEntry {
                    section: note.section,
                    repository: if note.section == LogbookSection::Repo { save.repository.clone() } else { None },
                    kind: note.kind,
                    text: note.text.clone(),
                    paths: Vec::new(),
                    source: save
                        .session_id
                        .clone()
                        .map(|session_id| TeachSource { session_id, cite: save.cite.clone().unwrap_or_default() }),
                };
                Some(Logbook::teach_logbook_entry(&*self, taught).await?.id)
            }
            None => None,
        };
        plan.entry = entry.clone();
        // Words kept for one repository with no address yet are looked for, off this request.
        let unaddressed = plan.words.as_ref().filter(|(one, key)| one.address.is_none() && key != "*").map(|(_, key)| key.clone());
        let switches = self.mod_switches_read().await?;
        let dir = self.mods_dir();
        let written = tokio::task::spawn_blocking(move || write(&dir, &switches, &plan))
            .await
            .map_err(|why| self.teaching_fault(format!("the mods could not be written: {why}")))?
            .map_err(|why| self.teaching_fault(why))?;
        if !written.is_empty() {
            self.mods_announced().await?;
        }
        if let (Some(manifest), Some(mod_name)) = (unaddressed, written.first()) {
            let wanted = Wanted { mod_name: mod_name.clone(), manifest, session_id: save.session_id.clone() };
            tokio::spawn(Arc::clone(&self).address_wanted(wanted));
        }
        // The teaching is kept, so nobody to tell is not a failure: the Session ended, or Fleet hosts
        // none of that name. A Session mid-turn has the line queued behind it.
        if let (Some(session), Some(note)) = (&save.session_id, &save.proposal.note) {
            let _ = self.told_news_by_fleet(session, &format!("From now on: {}", note.text.trim()), ipc::Notice::Taught).await;
        }
        Ok(SavedTeaching { entry, mods: written })
    }
}

/// What Teach was pressed on, as a mod records it.
fn taught_on(source: &TeachingSource) -> String {
    one_line(&source.text, manifest::MOST_TAUGHT_ON)
}
