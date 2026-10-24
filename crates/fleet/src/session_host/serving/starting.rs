//! Starting a hosted session: the process, the row and the first turn.

use super::*;

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
    /// The model the session runs on, as its process or its mod named it, and the window it last
    /// reported: what a subagent on the same model id is measured against. `None` unless both are known.
    pub(super) async fn parent_window(&self, id: &str) -> Option<(String, u64)> {
        let window = self.store().lock().await.session(id).ok().flatten()?.figures.context_window?;
        let model = self.hosts().of(id).state().resolved_model.clone();
        let model = model.or_else(|| self.hosts().terminals().facts_of(id).and_then(|facts| facts.model))?;
        Some((model, window))
    }

    /// A hosted session's ledger row and its hosting, marked as `marked` says.
    pub(crate) async fn started_hosted(
        &self,
        mut start: StartSession,
        marked: Marked,
    ) -> Result<SessionRecord, Refusal> {
        let rooted = start.root.is_some();
        let (manifest, cwd) = match (marked, start.root.take()) {
            (Marked::ModsFolder, _) => (None, self.mods_dir().to_string_lossy().into_owned()),
            (_, Some(root)) => {
                let repository = self.repository_named(Some(&root))?;
                let manifest = repository
                    .manifest()
                    .map(|one| one.id().as_str().to_string());
                (manifest, repository.root().to_string())
            }
            (_, None) => {
                let served = self.served_named(Some(&start.manifest_id))?;
                let manifest = served.manifest().id().as_str().to_string();
                (Some(manifest), served.root().to_string())
            }
        };
        // A root that has a Manifest by now is a plain start on it, not Setup's.
        let setup = rooted && manifest.is_none();
        let id = new_id();
        let now = self.now().as_str().to_string();
        let title = start
            .title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty());
        // A start with a request is being read: what the person set on it stands over the answer.
        // Marked before the row is published, so the first record already says so.
        if start.request.take().is_some() {
            let set = |value: &Option<String>| value.as_ref().is_some_and(|one| !one.trim().is_empty());
            let mut person = Vec::new();
            for (field, given) in [
                (ipc::ProposedField::Title, title.is_some()),
                (ipc::ProposedField::Model, set(&start.model)),
                (ipc::ProposedField::Effort, set(&start.effort)),
            ] {
                if given {
                    person.push(field);
                }
            }
            self.hosts().of(&id).state().proposal = super::super::proposing::ProposalState::begin(person);
        }
        {
            let mut store = self.store().lock().await;
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: manifest.clone(),
                    cwd,
                    title,
                    state: store::SessionState::Live,
                    started_at: now.clone(),
                    last_seen_at: now,
                    last_turn_at: None,
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                    mod_version: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    // Empty is Setup's session or a Mod Job in the mods folder, which have no
                    // Manifest to name.
                    manifest_id: manifest.unwrap_or_default(),
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    lease_slot: None,
                    lease_branch: None,
                    fork_of: None,
                    helm: marked == Marked::Helm,
                    setup,
                    workflow: None,
                    proposed: String::new(),
                    turn_model: None,
                    turn_effort: None,
                    paused_at: None,
                    paused_resuming: false,
                    auto_land: false,
                    mod_job: matches!(marked, Marked::Mod | Marked::ModsFolder),
                })
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }
}
