//! Finding where a taught pattern opens in a repository the person did not name an address for.
//!
//! **Lazily and off the request path**: a save or a read of the rules starts the search and answers
//! at once; the address lands in the mod when it is found, and Bridge hears the mods moved. The
//! search takes the cheapest source first.
//!
//! | Order | Source | Written as |
//! | --- | --- | --- |
//! | 1 | The Manifest's tracker names a site, and its adapter builds the address from the example | `settings` |
//! | 2 | A recently merged pull request's text holds an address with a match of the pattern in it | `pull_request #N` |
//! | 3 | Neither: the person is asked once, in Needs you | `person` |
//!
//! A search that finds nothing is remembered for the life of the process, and the question is kept
//! in the store, so neither is repeated.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AddressFound, ProposedAddress, TeachingWords};
use regex::{Regex, RegexBuilder};

use super::link::address_fault;
use crate::enriching::LOOKUP_BUDGET;
use super::saving::{address_given, words_file};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// How many merged pull requests are read at most. Each is a forge call.
const MOST_PULLS: usize = 5;

/// A mod and a repository's Manifest id.
type Key = (String, String);

/// Where a search for a mod's address in a repository stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stands {
    Going,
    Missed,
}

/// The searches this process started. Never written down.
#[derive(Default)]
pub(crate) struct Searched(Mutex<HashMap<Key, Stands>>);

/// What [`Searched::begin`] says to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Begin {
    /// Nobody searched yet: search.
    Search,
    /// A search is going.
    Busy,
    /// A search found nothing.
    Missed,
}

impl Searched {
    /// Claim the search for `key`, or say how it stands.
    pub(crate) fn begin(&self, key: &Key) -> Begin {
        let mut every = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match every.get(key) {
            Some(Stands::Going) => Begin::Busy,
            Some(Stands::Missed) => Begin::Missed,
            None => {
                every.insert(key.clone(), Stands::Going);
                Begin::Search
            }
        }
    }

    /// The search for `key` ended: with nothing, it is not made again.
    pub(crate) fn ended(&self, key: &Key, found: bool) {
        let mut every = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match found {
            true => every.remove(key),
            false => every.insert(key.clone(), Stands::Missed),
        };
    }
}

/// What a search is for.
#[derive(Clone)]
pub(crate) struct Wanted {
    pub(crate) mod_name: String,
    /// The repository's Manifest id.
    pub(crate) manifest: String,
    /// The Session whose person is asked when nothing is found, if any.
    pub(crate) session_id: Option<String>,
}

/// The address a text gives for `pattern`: the first web address in it that holds a match, with the
/// match in its place as `{match}`. **The match must stand alone in a path or query value** — after
/// `/`, `=` or `#` and before `/`, `?`, `&`, `#` or the end — so a number inside a longer word is not one.
pub(crate) fn template_in(text: &str, pattern: &str) -> Option<String> {
    let compiled: Regex = RegexBuilder::new(pattern).size_limit(100_000).build().ok()?;
    let mut rest = text;
    while let Some(at) = ["https://", "http://"].iter().filter_map(|scheme| rest.find(scheme)).min() {
        let tail = &rest[at..];
        let end = tail.find(|c: char| c.is_whitespace() || "<>\"'`()[]".contains(c)).unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        if let Some(template) = template_of(url, &compiled) {
            return Some(template);
        }
        rest = &tail[end..];
    }
    None
}

fn template_of(url: &str, pattern: &Regex) -> Option<String> {
    let scheme = url.find("://")? + 3;
    let host_end = url[scheme..].find(['/', '?', '#']).map_or(url.len(), |at| scheme + at);
    pattern.find_iter(&url[host_end..]).find_map(|found| {
        let (start, end) = (host_end + found.start(), host_end + found.end());
        let before = url[..start].chars().next_back()?;
        let after = url[end..].chars().next();
        let alone = "/=#".contains(before) && after.map_or(true, |c| "/?&#".contains(c));
        let template = format!("{}{{match}}{}", &url[..start], &url[end..]);
        (alone && address_fault(&template).is_none()).then_some(template)
    })
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
    /// Where `pattern` opens in `served`, and how it was found: the tracker the Manifest names, else
    /// a recently merged pull request's text. `None` where neither says.
    pub(crate) async fn address_found(&self, served: &Served, pattern: &str, example: &str) -> Option<(String, AddressFound)> {
        if let Some(template) = self.address_in_settings(served, example) {
            return Some((template, AddressFound::Settings));
        }
        self.address_in_pulls(served, pattern).await
    }

    /// The address the Manifest's tracker builds from `example`, where it names a site.
    fn address_in_settings(&self, served: &Served, example: &str) -> Option<String> {
        let tracker = self.tracker_of(served);
        tracker.address_template(example).filter(|template| address_fault(template).is_none())
    }

    /// The address a recently merged pull request's text gives for `pattern`, and which pull request.
    async fn address_in_pulls(&self, served: &Served, pattern: &str) -> Option<(String, AddressFound)> {
        let base = served.manifest().base()?.to_string();
        let listed = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| vcs.recently_merged_pull_requests(root, &base, MOST_PULLS))
            .await?;
        for pull in listed.into_iter().take(MOST_PULLS) {
            let number = pull.number;
            let page = self
                .forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_page(root, &number.to_string()))
                .await;
            let Some(page) = page else { continue };
            let said = page.conversation.iter().map(|one| match one {
                adapter_traits::Said::Comment { body, .. } | adapter_traits::Said::Review { body, .. } => body.as_str(),
            });
            let texts = page.body.as_deref().into_iter().chain(said);
            if let Some(template) = texts.into_iter().find_map(|text| template_in(text, pattern)) {
                return Some((template, AddressFound::PullRequest(number)));
            }
        }
        None
    }

    /// The address a read proposes for `words` in the repository at `root`, whose Manifest id is
    /// `manifest`: the one a Words mod with this pattern already has, else the tracker's, else a
    /// merged pull request's. **The pull request search is held to [`LOOKUP_BUDGET`]**, since a person
    /// is waiting on the read; the other two read local state.
    pub(crate) async fn address_proposed(&self, root: &str, manifest: &str, words: &TeachingWords) -> Option<ProposedAddress> {
        if let Ok(kept) = self.words_kept().await {
            let held = kept.iter().filter(|one| one.file.pattern == words.pattern).find_map(|one| one.address_for(manifest));
            if let Some(held) = held {
                return Some(ProposedAddress { template: held.template.clone(), found: held.found });
            }
        }
        let served = self.repositories().served().into_iter().find(|one| one.root() == root)?;
        if let Some(template) = self.address_in_settings(&served, &words.example) {
            return Some(ProposedAddress { template, found: AddressFound::Settings });
        }
        let (template, found) = tokio::time::timeout(LOOKUP_BUDGET, self.address_in_pulls(&served, &words.pattern)).await.ok()??;
        Some(ProposedAddress { template, found })
    }

    /// Search for the address `wanted` names, once per mod and repository for the life of the
    /// process, and write it to the mod where it is found. Where nothing is, the person is asked
    /// (`crate::teaching::asking`), if a Session is there to ask. **Run it on a task of its own**:
    /// it calls the forge.
    pub(crate) async fn address_wanted(self: Arc<Self>, wanted: Wanted) {
        let dir = self.mods_dir();
        let Some(file) = words_file(&dir, &wanted.mod_name) else { return };
        let has = |key: &str| file.address.iter().any(|one| one.repository == key);
        if has(&wanted.manifest) || has("*") {
            return;
        }
        let key = (wanted.mod_name.clone(), wanted.manifest.clone());
        match self.searched().begin(&key) {
            Begin::Busy => return,
            Begin::Missed => return self.address_asked(&wanted).await,
            Begin::Search => {}
        }
        let served = self.repositories().served().into_iter().find(|one| one.manifest().id().as_str() == wanted.manifest);
        let found = match served {
            Some(served) => self.address_found(&served, &file.pattern, &file.example).await,
            None => None,
        };
        let written = match &found {
            Some((template, how)) => {
                let (name, key_text, template, how) = (wanted.mod_name.clone(), wanted.manifest.clone(), template.clone(), *how);
                tokio::task::spawn_blocking(move || address_given(&dir, &name, &key_text, &template, how, false))
                    .await
                    .is_ok_and(|done| done.unwrap_or(false))
            }
            None => false,
        };
        self.searched().ended(&key, written);
        if written {
            let _ = self.mods_announced().await;
        } else if found.is_none() {
            self.address_asked(&wanted).await;
        }
    }
}
