//! The one question a person is asked about taught words: where they open in a repository that
//! neither the Manifest nor a merged pull request could say.
//!
//! **A call in the Session's Needs you list** (`WaitingSource::WordsAddress`), derived on every read
//! from the store's `word_asks` row and the mod: it stands while the row exists, the mod is a valid
//! Words mod with no address for the repository, and it has not been dismissed. The answer is the
//! address, written into the mod as `found = "person"`, which is also what ends the question.
//! **Once per mod and repository, for good**: the row is never deleted.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AddressFound, Instant, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use store::KeptWordAsk;

use super::finding::Wanted;
use super::saving::{address_given, words_file};
use crate::daemon::Fleet;
use crate::mods::slug_problem;

/// An item's id begins with this, then the mod and the Manifest id.
const PREFIX: &str = "words:";
/// The label of the option that takes the person's own words.
pub(crate) const OTHER: &str = "Other";
/// The most addresses offered besides Other.
const MOST_OFFERED: usize = 5;

fn item_id(mod_name: &str, manifest: &str) -> String {
    format!("{PREFIX}{mod_name}:{manifest}")
}

/// The mod and Manifest id an item id names.
pub(crate) fn parse_id(id: &str) -> Option<(String, String)> {
    let (name, manifest) = id.strip_prefix(PREFIX)?.split_once(':')?;
    (!name.is_empty() && !manifest.is_empty()).then(|| (name.to_string(), manifest.to_string()))
}

/// Every address any Words mod holds, with the repositories that use it, those of `first` leading.
fn known_sites(mods_dir: &Path, first: &str) -> Vec<(String, Vec<String>)> {
    let Ok(listing) = std::fs::read_dir(mods_dir) else { return Vec::new() };
    let mut names: Vec<String> = listing
        .filter_map(Result::ok)
        .map(|one| one.file_name().to_string_lossy().into_owned())
        .filter(|name| slug_problem(name).is_none())
        .collect();
    names.sort_by_key(|name| (name != first, name.clone()));
    let mut sites: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for name in names {
        let Some(file) = words_file(mods_dir, &name) else { continue };
        for one in file.address {
            let at = sites.len();
            let (_, used) = sites.entry(one.template).or_insert((at, Vec::new()));
            if !used.contains(&one.repository) {
                used.push(one.repository);
            }
        }
    }
    let mut ordered: Vec<(usize, String, Vec<String>)> = sites.into_iter().map(|(template, (at, used))| (at, template, used)).collect();
    ordered.sort();
    ordered.into_iter().map(|(_, template, used)| (template, used)).collect()
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
    /// Put the question `wanted` names to its Session's person, if it has one and the question was
    /// not put before.
    pub(crate) async fn address_asked(&self, wanted: &Wanted) {
        let Some(session) = &wanted.session_id else { return };
        let at = self.clock().now().as_str().to_string();
        let new = self.store().lock().await.ask_word_address(&wanted.mod_name, &wanted.manifest, session, &at);
        if matches!(new, Ok(true)) {
            let _ = self.published_hosted(session).await;
        }
    }

    /// The questions still open among `asks`, as Needs you items: one `answer` act each, an option
    /// for every address a Words mod already holds, and `Other` for the person's own.
    pub(crate) fn words_ask_items(&self, asks: &[KeptWordAsk], dismissed: &[String]) -> Vec<WaitingItem> {
        let dir = self.mods_dir();
        asks.iter()
            .filter_map(|ask| {
                let id = item_id(&ask.mod_name, &ask.repository);
                let file = words_file(&dir, &ask.mod_name)?;
                let covered = file.address.iter().any(|one| one.repository == ask.repository || one.repository == "*");
                if covered || dismissed.contains(&id) {
                    return None;
                }
                let mut options: Vec<WaitingOption> = known_sites(&dir, &ask.mod_name)
                    .into_iter()
                    .take(MOST_OFFERED)
                    .map(|(template, used)| WaitingOption { label: template, description: Some(used.join(", ")) })
                    .collect();
                options.push(WaitingOption { label: OTHER.to_string(), description: None });
                Some(WaitingItem {
                    id: id.clone(),
                    text: format!("Where does {} open in {}?", file.name, ask.repository),
                    since: Instant::carried(ask.asked_at.as_str()),
                    source: WaitingSource::WordsAddress,
                    act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id }),
                    options,
                    steps: Vec::new(),
                    place: None,
                    concern: None,
                    cites: Vec::new(),
                    confirmed: false,
                })
            })
            .collect()
    }

    /// The person answered the question `item` is: `picked` is the label of the option they chose,
    /// `words` what they wrote. An address picked is that address; `Other`, or no pick, is the words.
    /// The address is written to the mod, which closes the question.
    pub(crate) async fn words_address_chosen(
        self: &Arc<Self>,
        session: &str,
        item: &WaitingItem,
        picked: Option<&str>,
        words: Option<&str>,
    ) -> Result<SessionRecord, Refusal> {
        let (name, manifest) = parse_id(&item.id).ok_or_else(|| self.settled_already())?;
        let template = match (picked, words) {
            (Some(label), _) if label != OTHER => label.to_string(),
            (_, Some(written)) => written.trim().to_string(),
            _ => {
                return Err(self.hosted_refusal("fleet.session_waiting_empty", "choose one of the addresses, or write one with the matched words as {match}"));
            }
        };
        let dir = self.mods_dir();
        let held = (name.clone(), manifest.clone());
        let written = tokio::task::spawn_blocking(move || address_given(&dir, &held.0, &held.1, &template, AddressFound::Person, true))
            .await
            .map_err(|why| self.teaching_fault(format!("the address could not be written: {why}")))?;
        written.map_err(|why| self.hosted_refusal("fleet.session_waiting_empty", &why))?;
        let _ = self.mods_announced().await;
        self.published_hosted(session).await
    }
}
