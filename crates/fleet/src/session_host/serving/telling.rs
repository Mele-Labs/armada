//! The refusals, lookups and lines Fleet speaks into a hosted session.

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
    pub(crate) fn hosted_refusal(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    pub(crate) fn hosted_fault(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Fault(WireError::raised(code, why, self.run_id()))
    }

    pub(crate) fn closed(&self, id: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SESSION_CLOSED,
            format!("job {id} was closed, and a closed job takes nothing more"),
            self.run_id(),
        ))
    }

    pub(super) fn ask_not_waiting(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SESSION_ASK_NOT_WAITING,
            "nothing is waiting under that call. It was answered already, or the job \
             stopped waiting",
            self.run_id(),
        ))
    }

    /// The ledger's row for `id` where a person runs it in a terminal and Fleet
    /// hosts nothing of it.
    pub(crate) async fn terminal_session(&self, id: &str) -> Result<Option<KeptSession>, Refusal> {
        let store = self.store().lock().await;
        let found = store.session(id).map_err(|why| self.ledger_fault(why))?;
        let hosted = store.hosting(id).map_err(|why| self.ledger_fault(why))?;
        Ok(found.filter(|one| one.origin == "terminal" && hosted.is_none()))
    }

    pub(crate) async fn session_and_hosting(
        &self,
        id: &str,
    ) -> Result<(KeptSession, KeptHosting), Refusal> {
        let store = self.store().lock().await;
        let found = store
            .session(id)
            .and_then(|session| Ok((session, store.hosting(id)?)))
            .map_err(|why| self.ledger_fault(why))?;
        match found {
            (Some(session), Some(hosting)) => Ok((session, hosting)),
            _ => Err(self.hosted_refusal(
                NO_SUCH_SESSION,
                &format!("no job Fleet hosts is named {id}"),
            )),
        }
    }

    /// Where Setup's session runs, which has no Manifest and so no slot: the
    /// root it was started in. `None` for any other session.
    pub(crate) async fn unset_up_directory(
        &self,
        id: &str,
        hosting: &KeptHosting,
    ) -> Option<String> {
        if !hosting.manifest_id.is_empty() {
            return None;
        }
        let session = self.store().lock().await.session(id).ok().flatten()?;
        Some(session.cwd)
    }

    /// Where a session's process runs: the slot once it has leased one, the
    /// repository's root before.
    pub(crate) fn directory_of(served: &Served, hosting: &KeptHosting) -> String {
        match hosting.lease_slot {
            Some(slot) => adapter_traits::slot_path(served.root(), slot),
            None => served.root().to_string(),
        }
    }

    /// Whether there is a conversation to resume: the agent CLI kept one, or the agent has
    /// left anything in the thread but the person's own messages and Fleet's note that a process
    /// ended. A process that ended before its first turn left nothing else.
    pub(super) async fn has_a_conversation(&self, id: &str) -> bool {
        if adapters::terminal_thread::find(&self.host().home, id).is_some() {
            return true;
        }
        self.rows_of(id).await.is_ok_and(|rows| {
            rows.iter().any(|row| match row {
                // Fleet's own notes, a process that ended among them, are not the agent's.
                SessionRow::Message { from, .. } => *from == ipc::SessionVoice::Agent,
                // Fleet's word about a run, as its message was.
                SessionRow::Ran { .. } => false,
                // The owner's request is told as a run is; the agent's is its own act.
                SessionRow::Requested { by, .. } => *by == ipc::RequestBy::Agent,
                _ => true,
            })
        })
    }

    /// Fleet's own word to a session, started as a turn that wakes it. **Drawn as Fleet's** in a
    /// hosted thread, and held for the mod to submit in a terminal one, whose thread Fleet cannot
    /// write to. `Ok(false)` where there is nobody to tell: the session ended, or a terminal one
    /// that is not listening, and a caller asks again later. **The thread's row carries `notice`,**
    /// which every caller names, so a notice with no kind cannot be written.
    pub(crate) async fn told_by_fleet(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        notice: ipc::Notice,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, Some(Written::Noticed(notice))).await
    }

    /// [`Self::told_by_fleet`] where a hosted thread keeps `row` in place of Fleet's message, so what the
    /// person did is drawn once, as itself.
    pub(crate) async fn told_by_fleet_as(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        row: SessionRow,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, Some(Written::Row(row))).await
    }

    /// [`Self::told_by_fleet`] for a run that ended. **A hosted thread keeps a `ran` row** in place
    /// of Fleet's message, and the agent is sent the same words; a terminal Session's thread is
    /// not Fleet's to write, so it holds the words as it holds any other.
    pub(crate) async fn told_of_a_run(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        ran: RunEnd,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, Some(Written::Ran(ran))).await
    }

    /// [`Self::told_by_fleet`] for news that asks the agent for nothing: a pull request merged, a
    /// build moved. **The turn it starts leaves the agent's `all_done` standing**, since the agent
    /// has no work to find in it and would not call `all_done` again.
    pub(crate) async fn told_news_by_fleet(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        notice: ipc::Notice,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, true, Some(Written::Noticed(notice))).await
    }

    /// [`Self::told_news_by_fleet`] where a hosted thread keeps `row` in place of Fleet's message.
    pub(crate) async fn told_news_by_fleet_as(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        row: SessionRow,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, true, Some(Written::Row(row))).await
    }

    /// The first message of a Session that waited on another, written to its thread as the voice of the
    /// Session it waited on and started as a turn.
    pub(crate) async fn first_message_from(
        self: &Arc<Self>,
        session: &str,
        from: SessionVoice,
        text: &str,
    ) -> Result<(), Refusal> {
        self.first_message(session, from, text, false).await
    }

    /// [`Self::first_message_from`] for the brief a waiting Session is sent when its wait is met. **It is
    /// the person's brief in the waited Job's voice**, so the Session's first artifacts follow it as they
    /// follow a message of his.
    pub(crate) async fn waited_brief_from(
        self: &Arc<Self>,
        session: &str,
        from: SessionVoice,
        text: &str,
    ) -> Result<(), Refusal> {
        self.first_message(session, from, text, true).await
    }

    async fn first_message(
        self: &Arc<Self>,
        session: &str,
        from: SessionVoice,
        text: &str,
        followed: bool,
    ) -> Result<(), Refusal> {
        let sent = SendSessionMessage {
            session_id: SessionId::carried(session),
            text: text.to_string(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        Arc::clone(self).send_hosted_written(sent, Written::Message(from), None, None, followed).await.map(|_| ())
    }
}
