//! Teaching as the wire has it: read into a proposal, saved as a Logbook entry and mods, and the
//! patterns Bridge links listed for a repository.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Teaching};
use ipc::{
    FleetFile, FleetRule, ModKind, ReadTeaching, SaveTeaching, SavedTeaching, TeachingProposal, WireError, WordRule, WordRules,
    WordsAddress, WordsFile,
};

use crate::daemon::Fleet;
use super::finding::Wanted;
use crate::mods::{self, fleet_rules, words};

/// The mods folder would not be read. A 500.
const FAILED: &str = "fleet.teaching_failed";

/// A Words mod that is on and valid.
pub(crate) struct KeptWords {
    pub(crate) name: String,
    pub(crate) file: WordsFile,
}

impl KeptWords {
    /// Where this pattern opens for `manifest`: the repository's own address, else the one for every
    /// repository.
    pub(crate) fn address_for(&self, manifest: &str) -> Option<&WordsAddress> {
        let named = |key: &str| self.file.address.iter().find(|one| one.repository == key);
        named(manifest).or_else(|| named("*"))
    }
}

impl<H, V, W> Teaching for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn read_teaching(&self, teach: ReadTeaching) -> Result<TeachingProposal, Refusal> {
        self.teaching_read(teach).await
    }

    async fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> Result<SavedTeaching, Refusal> {
        self.teaching_saved(save).await
    }

    async fn list_word_rules(
        self: Arc<Self>,
        repository: Option<String>,
        session_id: Option<String>,
    ) -> Result<WordRules, Refusal> {
        let root = repository.map(|named| self.run_repository(&named)).transpose()?;
        let manifest = root.as_deref().and_then(|root| self.manifest_id_at(root));
        let mut rules = Vec::new();
        for one in self.words_kept().await? {
            let Some(address) = one.address_for(manifest.as_deref().unwrap_or("")) else {
                // Words with no address here yet: look for one, off this request. The rule is
                // listed once it is found, on a later read.
                if let Some(manifest) = &manifest {
                    let wanted = Wanted { mod_name: one.name.clone(), manifest: manifest.clone(), session_id: session_id.clone() };
                    tokio::spawn(Arc::clone(&self).address_wanted(wanted));
                }
                continue;
            };
            rules.push(WordRule {
                mod_name: one.name.clone(),
                name: one.file.name.clone(),
                pattern: one.file.pattern.clone(),
                example: one.file.example.clone(),
                address: address.template.clone(),
                found: address.found,
            });
        }
        Ok(WordRules { rules })
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
    pub(crate) fn teaching_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(FAILED, why.to_string(), self.run_id()))
    }

    /// Every file of `kind` that is on and valid, off the async thread.
    async fn mod_files_kept(&self, kind: ModKind) -> Result<Vec<(String, String)>, Refusal> {
        let switches = self.mod_switches_read().await?;
        let dir = self.mods_dir();
        tokio::task::spawn_blocking(move || mods::kept(&dir, &switches, kind))
            .await
            .map_err(|why| self.teaching_fault(format!("the mods folder could not be read: {why}")))
    }

    /// Every Words mod that is on and valid, by name.
    pub(crate) async fn words_kept(&self) -> Result<Vec<KeptWords>, Refusal> {
        let files = self.mod_files_kept(ModKind::Words).await?;
        Ok(files.into_iter().filter_map(|(name, text)| words::read(&text).ok().map(|file| KeptWords { name, file })).collect())
    }

    /// What the Fleet mods say to do with `notice` for `session` in the repository `manifest`, if
    /// any does. **Read by what sends every notice**, `told_by_fleet`, after the person's mute.
    pub(crate) async fn fleet_rule_for(&self, notice: &str, text: &str, session: &str, manifest: &str) -> Option<FleetRule> {
        let files = self.mod_files_kept(ModKind::Fleet).await.ok()?;
        let read: Vec<FleetFile> = files.iter().filter_map(|(_, text)| fleet_rules::read(text).ok()).collect();
        fleet_rules::rule_for(&read, notice, text, session, manifest).cloned()
    }
}
