//! Mods, as the fake answers them: a scaffold is echoed, and nothing is on disk.
//! What is read, checked and committed is `fleet::mods`'s, against a real folder.

use std::sync::Arc;

use ipc::{
    ModChecked, ModList, ModPromoted, ModScaffolded, ModSessionStarted, ModSummary, PromoteMod,
    ScaffoldMod, SetModEnabled, StartModSession,
};

use super::FakeDaemon;
use crate::{Mods, Refusal};

impl Mods for FakeDaemon {
    async fn list_mods(&self) -> Result<ModList, Refusal> {
        Ok(ModList::default())
    }

    async fn scaffold_mod(&self, scaffold: ScaffoldMod) -> Result<ModScaffolded, Refusal> {
        Ok(ModScaffolded {
            path: format!("/mods/{}", scaffold.name),
            name: scaffold.name,
        })
    }

    async fn set_mod_enabled(&self, set: SetModEnabled) -> Result<ModSummary, Refusal> {
        Ok(ModSummary {
            name: set.name,
            origin: ipc::ModOrigin::Local,
            kind: Some(ipc::ModKind::Theme),
            version: Some(String::from("0.1.0")),
            description: None,
            taught_on: None,
            entry: None,
            made_by: None,
            enabled: set.enabled,
            valid: true,
            reason: None,
            changed_at: None,
            tried: None,
            stale: false,
            pull_request: None,
        })
    }

    async fn validate_mod(&self, name: String) -> Result<ModChecked, Refusal> {
        Ok(ModChecked {
            name,
            valid: true,
            problems: Vec::new(),
            css: None,
            layout: None,
            words: None,
            fleet: None,
            commands: None,
        })
    }

    async fn try_tracker_mod(&self, asked: ipc::TryTrackerMod) -> Result<ipc::TrackerTry, Refusal> {
        Ok(ipc::TrackerTry {
            at: ipc::Instant::carried("2026-10-23T00:00:00.000Z"),
            manifest_id: asked.manifest_id.unwrap_or_else(|| ipc::ManifestId::carried("armada")),
            passed: true,
            reason: None,
            listed: 0,
            read: None,
        })
    }

    async fn promote_mod(self: Arc<Self>, promote: PromoteMod) -> Result<ModPromoted, Refusal> {
        Ok(ModPromoted {
            branch: format!("armada/mod-{}-0", promote.name),
            name: promote.name,
            session_id: ipc::SessionId::carried("fake-session"),
            title: String::from("Add the mod"),
            commit: String::from("0000000"),
            pull_request: None,
            pull_request_not_opened: None,
        })
    }

    async fn start_mod_session(self: Arc<Self>, start: StartModSession) -> Result<ModSessionStarted, Refusal> {
        Ok(ModSessionStarted {
            session_id: ipc::SessionId::carried("fake-session"),
            title: start.title.unwrap_or_else(|| String::from("Mod")),
            send_failed: None,
        })
    }
}
