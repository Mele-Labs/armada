//! Writing a turn to a hosted session: ahead of a running one, or after it has been started.

use super::*;
use super::super::start_card::FirstTurn;
use crate::session_links::LinkedBy;

/// The line after the person's words when a done stood as they wrote. The message clears the done
/// and the agent is not otherwise told, so it answered in prose and never called `all_done` again.
/// Only the agent reads it: the person's row keeps their words.
pub(crate) const TAKES_DONE_BACK: &str = "You had called all_done, and this message takes that back. If nothing is left once you have dealt with it, call all_done again.";

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
    pub(super) async fn fleet_word(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        news: bool,
        kept: Option<Written>,
    ) -> Result<bool, Refusal> {
        if let Some(terminal) = self.terminal_session(session).await? {
            if terminal.state == store::SessionState::Ended {
                return Ok(false);
            }
            let said = format!("{FLEET_NAME}: {text}");
            return Ok(self.hosts().terminals().hold(
                session,
                said,
                super::super::terminal::LISTENING_FOR,
            ));
        }
        let (held, _) = self.session_and_hosting(session).await?;
        if held.state == store::SessionState::Ended {
            return Ok(false);
        }
        let sent = SendSessionMessage {
            session_id: SessionId::carried(session),
            text: text.to_string(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        let written = match kept {
            Some(kept) => kept,
            None => Written::Message(SessionVoice::Session {
                id: FLEET_NAME.to_lowercase(),
                title: FLEET_NAME.to_string(),
            }),
        };
        // Built here rather than in the send, so a notice that asks something is kept by its row.
        let row = self.written_row(session, &written, text.trim(), &[], &[]);
        let notice = store::KeptNotice {
            session_id: session.to_string(),
            row_id: row.id().to_string(),
            at: self.now().as_str().to_string(),
            text: text.trim().to_string(),
        };
        if news {
            self.hosts().of(session).state().fleet_told += 1;
        }
        Arc::clone(self)
            .send_hosted_written(sent, Written::Row(row), None, None, false)
            .await?;
        // News asks nothing, so there is nothing in the agent's replies to read against it. With
        // dispute reviews off nothing is kept, so switching them on reads none sent before.
        if !news && self.dispute_reviews_on() {
            let _ = self.store().lock().await.keep_fleet_notice(&notice);
        }
        Ok(true)
    }

    /// The row a thread keeps of `written`.
    fn written_row(
        &self,
        session: &str,
        written: &Written,
        text: &str,
        files: &[ipc::SentFile],
        tags: &[ipc::SessionTag],
    ) -> SessionRow {
        match written {
            Written::Message(from) => SessionRow::Message {
                id: self.row_id(session),
                at: self.instant(),
                from: from.clone(),
                text: text.to_string(),
                files: files.to_vec(),
                tags: tags.to_vec(),
                notice: None,
            },
            Written::Noticed(notice) => SessionRow::Message {
                id: self.row_id(session),
                at: self.instant(),
                from: SessionVoice::Session {
                    id: FLEET_NAME.to_lowercase(),
                    title: FLEET_NAME.to_string(),
                },
                text: text.to_string(),
                files: files.to_vec(),
                tags: tags.to_vec(),
                notice: Some(notice.word().to_string()),
            },
            Written::Row(row) => row.clone(),
            Written::Ran(ran) => SessionRow::Ran {
                id: self.row_id(session),
                at: self.instant(),
                command: ran.command.clone(),
                code: ran.code,
                tail: ran.tail.clone(),
            },
        }
    }

    /// A message to a session Fleet hosts, written to its thread as `from` and started as a turn,
    /// with a line of Fleet's own ahead of the words in the turn. **`You` is a person's; Fleet's
    /// own notices come through [`Self::told_by_fleet`].** **The thread's row keeps the words as
    /// typed**, so the line is the agent's and never drawn: Helm's "where they are" is the one
    /// caller that sets it.
    pub(super) async fn send_hosted_ahead(
        self: Arc<Self>,
        sent: SendSessionMessage,
        from: SessionVoice,
        ahead: Option<String>,
        instead: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        self.send_hosted_written(sent, Written::Message(from), ahead, instead, false)
            .await
    }

    /// [`Self::send_hosted_ahead`] with what the thread keeps of the words chosen: a message from
    /// someone, or the `ran` row of a command that ended. `instead` is what the agent reads in place
    /// of the words, Fleet's lines with them, as pinned notes' are, and is no more the thread's than
    /// `ahead` is. `followed` makes the row the one the Session's artifacts follow, as a person's is.
    pub(super) async fn send_hosted_written(
        self: Arc<Self>,
        sent: SendSessionMessage,
        written: Written,
        ahead: Option<String>,
        instead: Option<String>,
        followed: bool,
    ) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().to_string();
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let text = sent.text.trim().to_string();
        if text.is_empty() && sent.attachments.is_empty() {
            return Err(self.hosted_refusal(
                MESSAGE_EMPTY,
                "a message needs words or something sent with it",
            ));
        }
        // Before this message is written, so a Job held across a restart does not read it back as an old one.
        self.start_hold_restored(&id, &hosting).await?;
        let kept = self.keep_uploads(&self.uploads_of(&id), &id, &sent.attachments)?;
        let addressed = self.addressed(&sent.mentions).await?;
        // A person's first message to a Session that was started without a request is read as a request is.
        // Fleet's words and a Session's brief (`first_message_from`) are not the person's to be read.
        if matches!(written, Written::Message(SessionVoice::You)) {
            self.reading_begun(&session, &hosting, &text);
        }
        // Read against the thread before the answer's own row is put in it.
        let handing_on = matches!(written, Written::Message(SessionVoice::You))
            && self.handoff_answered_later(&id, &text).await;
        // A held Job's turns are kept off the thread, whole: the first for him to edit, and Start writes and sends them all.
        let first = match &written {
            Written::Message(from) if !handing_on => Some(FirstTurn {
                from: from.clone(),
                text: text.clone(),
                files: kept.files.clone(),
                tags: sent.mentions.clone(),
                instead: instead.clone(),
                ahead: ahead.clone(),
            }),
            _ => None,
        };
        if let Some(first) = first {
            if self.turn_kept_while_held(&id, &first).await? {
                if matches!(written, Written::Message(SessionVoice::You)) {
                    self.noted_on_the_ledger(&session, &text).await?;
                    self.issues_named(&id, &text, &sent.mentions).await;
                    self.links_seen(&id, &text, LinkedBy::You).await;
                    self.handoff_failure_cleared(&id).await;
                }
                return self.published_hosted(&id).await;
            }
        }
        let row = self.written_row(&id, &written, &text, &kept.files, &sent.mentions);
        let row_id = row.id().to_string();
        if matches!(written, Written::Message(SessionVoice::You)) {
            self.hosts().of(&id).state().activity.writing.person_said(row.id(), &text);
        }
        self.row_put(&id, row).await;
        // Before the early returns below: a handoff's answer is still the person's latest message. A waiting
        // Session's brief is the person's too, though it is written in the voice of the Job it waited on.
        if followed || matches!(written, Written::Message(SessionVoice::You)) {
            // The artifacts drawn from here on follow this message, until the next one.
            if let Err(why) = self.store().lock().await.keep_session_message(&id, &row_id) {
                self.session_warned(&id, "a job's latest message was not kept", format!("{why:?}"));
            }
        }
        if handing_on {
            return self.published_hosted(&id).await;
        }
        if matches!(written, Written::Message(SessionVoice::You)) {
            self.hosts().of(&id).state().owes_reply = true;
            self.noted_on_the_ledger(&session, &text).await?;
            self.issues_named(&id, &text, &sent.mentions).await;
            self.links_seen(&id, &text, LinkedBy::You).await;
            self.handoff_failure_cleared(&id).await;
        }

        let held = super::super::proposing::Held {
            text,
            kept,
            addressed,
            ahead,
            instead,
        };
        // A Session whose request is still being read starts on the answer, and this turn with it.
        let runtime = self.hosts().of(&id);
        let held = {
            let mut state = runtime.state();
            match state.proposal.holds_back() {
                true => {
                    state.proposal.held.push_back(held);
                    None
                }
                false => Some(held),
            }
        };
        if let Some(held) = held {
            self.deliver_turn(&id, held).await?;
        }
        self.published_hosted(&id).await
    }

    /// Start the process if it has none, and write the turn to it.
    pub(crate) async fn deliver_turn(
        self: &Arc<Self>,
        id: &str,
        held: super::super::proposing::Held,
    ) -> Result<(), Refusal> {
        let super::super::proposing::Held {
            text,
            kept,
            addressed,
            ahead,
            instead,
        } = held;
        let (session, hosting) = self.session_and_hosting(id).await?;
        // A session closed while its turn was held takes nothing more.
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        let runtime = self.hosts().of(id);
        // Before the process starts, so the rail sees a Job that has run only with its step already kept.
        let stepped = self.first_step_kept(id, &hosting, &text).await;
        self.ensure_process(id, &hosting)
            .await
            .map_err(|why| self.hosted_fault(SESSION_UNSTARTED, &why))?;
        let mut turn = String::new();
        if let Some(line) = &ahead {
            turn.push_str(line);
            turn.push_str("\n\n");
        }
        if let Some(line) = self.turn_told(id, &hosting).await {
            turn.push_str(&line);
            turn.push_str("\n\n");
        }
        if let Some(words) = &stepped {
            turn.push_str(words);
            turn.push_str("\n\n");
        }
        if let Some(line) = mention_line(&addressed) {
            turn.push_str(&line);
            turn.push_str("\n\n");
        }
        turn.push_str(instead.as_deref().unwrap_or(&text));
        for path in &kept.paths {
            turn.push_str(&format!("\n\nAttached file: {path}"));
        }
        let turn = super::super::cut_off::told(&mut runtime.state(), turn);
        let line = ipc::encode(&UserLine::of(turn, kept.pictures))
            .map_err(|why| self.hosted_refusal(SESSION_UNSTARTED, &why.to_string()))?;

        {
            let mut state = runtime.state();
            state.turn = SessionTurn::Working { woken_by: None };
            state.last_active = std::time::Instant::now();
            state.send_turn(line);
        }
        // The process said it was gone before the line was written to it.
        let lost = {
            let mut state = runtime.state();
            match (state.process.is_none(), state.retried) {
                (true, false) if !state.unstarted.is_empty() => {
                    state.retried = true;
                    std::mem::take(&mut state.unstarted)
                }
                _ => Vec::new(),
            }
        };
        if !lost.is_empty() {
            self.send_again(id, &runtime, lost).await;
        }
        Ok(())
    }

    /// Each session a message names, with the address another session writes
    /// to where Fleet hosts it.
    pub(crate) async fn addressed(
        &self,
        tags: &[ipc::SessionTag],
    ) -> Result<Vec<(ipc::SessionTag, Option<String>)>, Refusal> {
        let store = self.store().lock().await;
        tags.iter()
            .map(|tag| {
                let address = match tag.kind {
                    ipc::TagKind::Session => store
                        .hosting(&tag.id)
                        .map_err(|why| self.ledger_fault(why))?
                        .map(|_| address_of(&tag.id)),
                    _ => None,
                };
                Ok((tag.clone(), address))
            })
            .collect()
    }

    /// The title a first message gives.
    pub(super) async fn noted_on_the_ledger(
        &self,
        session: &KeptSession,
        text: &str,
    ) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        let mut changed = false;
        // A title the proposer is about to give is not made from the first message first.
        let reading = self.hosts().of(&session.id).state().proposal.reading.is_some();
        if session.title.is_none() && !reading {
            if let Some(title) = title_of(text) {
                let mut row = session.clone();
                row.title = Some(title);
                store
                    .keep_session(&row)
                    .map_err(|why| self.ledger_fault(why))?;
                changed = true;
            }
        }
        drop(store);
        if changed {
            self.published_hosted(&session.id).await?;
        }
        Ok(())
    }

    /// A message to a session whose kind the caller has already read: `terminal` is the session
    /// where it is a terminal one.
    pub(crate) async fn send_known(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        self.send_known_ahead(terminal, sent, None).await
    }

    /// [`Self::send_known`] with the line [`Self::send_hosted_ahead`] puts ahead of the words.
    /// Read only for a hosted session.
    pub(crate) async fn send_known_ahead(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
        ahead: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        self.send_known_added(terminal, sent, ahead, None).await
    }

    /// [`Self::send_known_ahead`] with what the agent reads in place of the words as well, for a
    /// terminal session too: pinned notes', the words with Fleet's lines.
    pub(crate) async fn send_known_added(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
        ahead: Option<String>,
        instead: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().to_string();
        let approved = super::super::walking::approved_address(&sent.text).map(str::to_string);
        let noted = super::super::readiness::note_address(&sent.text).map(str::to_string);
        // Read before the send: the clear below runs after it, and the agent has to be told.
        let standing = self.store().lock().await.done_since(&id).ok().flatten().is_some();
        let instead = if standing {
            let words = instead.unwrap_or_else(|| sent.text.clone());
            Some(format!("{words}\n{TAKES_DONE_BACK}"))
        } else {
            instead
        };
        let record = if let Some(session) = terminal {
            let addressed = self.addressed(&sent.mentions).await?;
            let kept = self.keep_uploads(&self.uploads_of(&id), &id, &sent.attachments)?;
            let mut paths = kept.paths;
            paths.extend(kept.picture_paths);
            self.send_to_terminal(&session, sent, addressed, paths, instead)
                .await?
        } else {
            Arc::clone(&self)
                .send_hosted_ahead(sent, SessionVoice::You, ahead, instead)
                .await?
        };
        if let Some(url) = approved {
            self.walk_ended(&id, &url).await;
        }
        if noted.is_some() {
            self.window_noted(&id).await;
        }
        // The person spoke, so the agent's word that nothing was left no longer stands.
        let done = self.done_cleared(&id).await;
        // And so does the reply the last turn left: he has answered it.
        let replied = self.turn_end_cleared(&id).await;
        // And the person has taken the session back from the limit: Fleet does not wake it as well.
        let limited = self.limit_cleared(&id).await;
        if done || replied || limited {
            return self.published_hosted(&id).await;
        }
        Ok(record)
    }
}
