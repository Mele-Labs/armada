//! A hosted session's permission door: the call is put to the person on the
//! session's own thread, and the answer goes back inside the call. Since 23.49.
//!
//! **Helm's asking machinery, not a second one**: the same table, the same
//! reading of what is destructive, pushes to a shared space or writes off this
//! machine, and the same words back to the model. What differs is where the ask
//! is drawn, which is the thread and never Helm's dock, and which calls reach a
//! person: `auto` asks only about those three classes, `ask` and `acceptEdits`
//! ask about everything the person's own settings do not cover.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal, Sessions};
use ipc::{
    AskingToRun, HelmCallAnswer, HelmCallInFlight, ManifestId, ReachesTo, RunOrNot, SessionAskState,
    SessionMode, SessionRow, TerminalAsked,
};

use super::serving::mode_of;
use super::walking::walk_open_refusal;
use crate::daemon::Fleet;
use crate::helm::{
    answering, blocked_folder, guarded_in_a_session, pages_opened, unanswered, Because, Folders, Guarded, Own,
    Said,
};

/// How long one `Wait` is held. The mod's request must outlast it.
const POLL_HOLD: Duration = Duration::from_secs(25);

/// How long a terminal question may go without a poll before its prompt is
/// taken as gone and its card closes.
pub(crate) const POLL_LAPSE: Duration = Duration::from_secs(180);

/// A kept terminal question that would not encode or read back.
const TERMINAL_ASK_UNREADABLE: &str = "fleet.terminal_ask_unreadable";

/// A block a question, the question in bold and what was chosen under it, for an answer that
/// allowed it. Nothing where
/// the call was a permission, which carries no answers, or was refused.
fn answers_text(said: &Said) -> Option<String> {
    if said.answer != HelmCallAnswer::AllowOnce || said.answers.is_empty() {
        return None;
    }
    let lines: Vec<String> = said
        .answers
        .iter()
        .map(|one| format!("**{}**\n{}", one.question, one.chosen.join(", ")))
        .collect();
    Some(lines.join("\n\n"))
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
    /// The hosted session whose process holds this call's connection.
    /// **Placed by the connection**, as Helm's is, never by what the caller says.
    pub(crate) fn session_holding(&self, caller: &Caller) -> Option<String> {
        self.session_placed(caller).map(|(id, _)| id)
    }

    /// The hosted session whose process holds this call's connection, and whether it is a
    /// repository's Helm, whose door is Helm's and not a Session's.
    pub(crate) fn session_placed(&self, caller: &Caller) -> Option<(String, bool)> {
        self.hosts().all().into_iter().find_map(|(id, runtime)| {
            let (pid, helm) = {
                let state = runtime.state();
                (state.process.as_ref().and_then(|p| p.pid())?, state.helm)
            };
            crate::peer::held_within(caller, self.host().port, &[pid], self.peers().as_ref())
                .then_some((id, helm))
        })
    }

    pub(crate) async fn session_permission(
        &self,
        id: &str,
        asking: AskingToRun,
    ) -> Result<RunOrNot, Refusal> {
        // A question held open during a walk would hold back every note and the Approve.
        if asking.tool_name == ipc::ASKS_A_QUESTION && self.walk_open(id).await.is_some() {
            let manifest = match self.session_and_hosting(id).await {
                Ok((_, hosting)) => ManifestId::carried(&hosting.manifest_id),
                Err(_) => ManifestId::carried(""),
            };
            self.walk_refused_ask(id, &manifest, &asking).await;
            return Ok(RunOrNot::Deny {
                message: walk_open_refusal(),
            });
        }
        // The owner's browser is never opened by a session: the page goes to
        // Bridge's window, and the session is told so.
        if asking.tool_name == "Bash" {
            let pages = pages_opened(asking.detail().unwrap_or_default());
            if !pages.is_empty() {
                for url in pages {
                    let show = ipc::ShowWindow {
                        url,
                        title: None,
                        session_id: Some(ipc::SessionId::carried(id)),
                        variations: Vec::new(),
                        ..Default::default()
                    };
                    let _ = self.show_window(None, show).await;
                }
                return Ok(RunOrNot::Deny {
                    message: String::from(
                        "Shown in Bridge's window instead. Use the armada show_window tool to show the owner a page; never `open` it.",
                    ),
                });
            }
        }
        let (session, hosting) = self.session_and_hosting(id).await?;
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)));
        let (held, root) = match &served {
            Ok(served) => (
                self.held_directories(id, served, &hosting),
                served.root().to_string(),
            ),
            // Setup's session holds no slot: the root it was started in is where it works.
            Err(_) if hosting.manifest_id.is_empty() => {
                (vec![session.cwd.clone()], session.cwd.clone())
            }
            Err(why) => return Err(why.clone()),
        };
        let mode = mode_of(&hosting.mode);
        // A question is for the person whatever the mode: nothing else can
        // answer it, and the agent is told what they chose.
        // **In `auto` a call runs unless it reaches past the slots.** A read
        // macOS guards and a write outside the slots and `/tmp` are put to the
        // owner on a card, with the path (16 Oct 2026, reversing 8 Oct's "never
        // waits"); the hold bounds the wait, and silence refuses. His folder
        // list answers first: blocked is refused, allowed runs, and then the
        // folders he allowed for this session alone. A removal
        // outside the slots is refused with the reason, which the agent reads
        // and works around.
        let mut reaching = None;
        let put_to_a_person = asking.tool_name == ipc::ASKS_A_QUESTION
            || match mode {
                SessionMode::Ask | SessionMode::AcceptEdits => true,
                SessionMode::Auto | SessionMode::Plan => {
                    let named: Vec<&str> = held.iter().map(String::as_str).collect();
                    // A branch is looked up only for a line that names one.
                    let merged = match (asking.detail().is_some_and(|line| line.contains("branch")), &served) {
                        (true, Ok(served)) => self.own_merged_branches(id, served, &hosting).await,
                        _ => Vec::new(),
                    };
                    let merged: Vec<&str> = merged.iter().map(String::as_str).collect();
                    let started = self.started_by(id, &held);
                    let own = Own { started: Some(&started), merged_branches: &merged };
                    let listed = self.reach_folders();
                    let home = std::env::var("HOME").unwrap_or_default();
                    let session = self.hosts().of(id).state().allowed_folders.clone();
                    let folders = Folders { allow: &listed.allow, block: &listed.block, session: &session, home: &home };
                    match guarded_in_a_session(&asking, &named, own, folders) {
                        Some(Guarded::Blocked { folder }) => {
                            return Ok(RunOrNot::Deny { message: blocked_folder(&folder) });
                        }
                        Some(Guarded::Because { because: Because::ReachesOutside, reaching: at, to })
                        | Some(Guarded::Because { because: Because::Destructive, reaching: at @ Some(_), to }) => {
                            reaching = Some((at, to.unwrap_or(ReachesTo::Write)));
                            true
                        }
                        Some(Guarded::Because { because: Because::Destructive, .. }) => {
                            return Ok(RunOrNot::Deny {
                                message: String::from(
                                    "Refused: that removes or overwrites something outside the slots you hold and /tmp. Do it inside a slot you hold, or say in your reply what you need and why.",
                                ),
                            });
                        }
                        _ => false,
                    }
                }
            };
        if !put_to_a_person {
            return Ok(RunOrNot::Allow {
                updated_input: asking.input,
            });
        }
        let asks = self.hosts().asks();
        let (waiting, answer) = asks.minted_reaching(
            &ManifestId::carried(&hosting.manifest_id),
            &asking,
            &self.now(),
            reaching,
        );
        let row_id = format!("ask-{}-{id}", waiting.call);
        let put = |state: SessionAskState, via: Option<ipc::Via>| SessionRow::Ask {
            id: row_id.clone(),
            at: self.instant(),
            ask: waiting.clone(),
            state,
            via,
        };
        self.hosts().of(id).state().asked = Some(waiting.clone());
        self.row_put(id, put(SessionAskState::Waiting, None)).await;
        let _ = self.published_hosted(&session.id).await;

        let held = asks.hold();
        let mut said_aloud = None;
        let (decided, state) = match tokio::time::timeout(held, answer).await {
            Ok(Ok(said)) => {
                said_aloud = answers_text(&said);
                let state = match said.answer {
                    HelmCallAnswer::AllowOnce => SessionAskState::AllowedOnce,
                    HelmCallAnswer::Refuse => SessionAskState::Refused,
                    // A list that will not be written is not a refusal: the person
                    // answered this call, and the next one asks again.
                    HelmCallAnswer::AlwaysAllow | HelmCallAnswer::AlwaysBlock => {
                        let blocked = said.answer == HelmCallAnswer::AlwaysBlock;
                        if let Some(folder) = &said.folder {
                            let _ = self.reach_folder_kept(folder, blocked).await;
                        }
                        match blocked {
                            true => SessionAskState::AlwaysBlocked,
                            false => SessionAskState::AlwaysAllowed,
                        }
                    }
                    // In memory only: it goes when the session is closed, or when Fleet stops.
                    HelmCallAnswer::AllowForSession => {
                        if let Some(folder) = &said.folder {
                            self.hosts().of(id).state().allowed_folders.push(folder.clone());
                        }
                        SessionAskState::AllowedForSession
                    }
                    HelmCallAnswer::AllowAndRemember => {
                        let at = adapters::personal_settings(&root);
                        // A settings file that will not be written is not a
                        // refusal: the person said yes.
                        let _ = adapters::remember_the_rule(&at, &waiting.rule);
                        SessionAskState::AllowedAndRemembered
                    }
                };
                (answering(&asking, &said), state)
            }
            Ok(Err(_)) => (unanswered(held.as_secs()), SessionAskState::SessionGone),
            Err(_) => {
                let _ = asks.withdraw(&waiting.call);
                (unanswered(held.as_secs()), SessionAskState::Unanswered)
            }
        };
        let via = {
            let host = self.hosts().of(id);
            let mut held = host.state();
            held.asked = None;
            held.answered_via.take()
        };
        self.row_put(id, put(state, via)).await;
        if let Some(text) = said_aloud {
            self.person_said(id, text).await;
        }
        let _ = self.published_hosted(id).await;
        Ok(decided)
    }

    /// A question refused for a walk, kept on the thread whole: every question, header, option and
    /// description, in an ask row settled `Kept`, which Bridge draws in Needs you as the question that
    /// lapsed. **Never put on the table**: the call is withdrawn as it is minted, so nothing waits on
    /// an answer and the card is not a live one.
    async fn walk_refused_ask(&self, id: &str, manifest: &ManifestId, asking: &AskingToRun) {
        let asks = self.hosts().asks();
        let (kept, _) = asks.minted(manifest, asking, &self.now());
        let _ = asks.withdraw(&kept.call);
        self.row_put(id, self.ask_row(id, &kept, SessionAskState::Kept, None)).await;
        let _ = self.published_hosted(id).await;
    }

    /// What the person chose, as a message of theirs in the thread: the ask row settles into a
    /// collapsed tool line, which is not where a person looks for what they said.
    async fn person_said(&self, id: &str, text: String) {
        let row = SessionRow::Message {
            id: self.row_id(id),
            at: self.instant(),
            from: ipc::SessionVoice::You,
            text,
            files: Vec::new(),
            tags: Vec::new(),
        };
        self.row_put(id, row).await;
    }

    /// A terminal session's `AskUserQuestion`, put to Bridge while its own
    /// prompt is up. **The same ask row a hosted session's question writes**, and
    /// the same table: whichever of Bridge and the terminal answers first wins,
    /// and the mod tells Fleet when it was the terminal.
    ///
    /// **Kept in the store, not held on a connection.** The mod asks `Wait` for
    /// the answer, so a request that ends or a Fleet that restarts costs a poll
    /// and never the question.
    pub(crate) async fn terminal_question(
        &self,
        id: &str,
        asking: AskingToRun,
    ) -> Result<TerminalAsked, Refusal> {
        let session = self.terminal_session(id).await?.ok_or_else(|| {
            self.hosted_refusal(
                super::serving::NO_SUCH_SESSION,
                &format!("no terminal job is named {id}"),
            )
        })?;
        // One at a time: a new question means the last one's prompt is gone.
        self.terminal_ask_closed(id, SessionAskState::Unanswered)
            .await;
        let question = asking.tool_name == ipc::ASKS_A_QUESTION;
        // A walk holds back a question, which would hold back his notes. A permission is the
        // terminal's own prompt, and is put to him whatever he is walking.
        if question && self.walk_open(id).await.is_some() {
            let manifest = ManifestId::carried(session.manifest_id.as_deref().unwrap_or_default());
            self.walk_refused_ask(id, &manifest, &asking).await;
            return Ok(TerminalAsked::Refused {
                message: walk_open_refusal(),
            });
        }
        let asks = self.hosts().asks();
        let manifest = ManifestId::carried(session.manifest_id.as_deref().unwrap_or_default());
        // The channel is not waited on; the answer is kept by whoever gives it.
        let (mut waiting, _) = asks.minted(&manifest, &asking, &self.now());
        // Nothing writes a terminal's rule for it yet, so its permission offers no remembering.
        waiting.offers.retain(|one| *one != HelmCallAnswer::AllowAndRemember);
        let kept = store::KeptTerminalAsk {
            call: waiting.call.clone(),
            asking: self.kept_text(&asking)?,
            in_flight: self.kept_text(&waiting)?,
            answer: None,
        };
        self.store()
            .lock()
            .await
            .keep_terminal_ask(id, &kept)
            .map_err(|why| self.ledger_fault(why))?;
        self.hosts().terminals().polled(&waiting.call);
        self.hosts().of(id).state().asked = Some(waiting.clone());
        // A question is a card in the thread. A permission is answered on its call and in Needs you.
        if question {
            self.row_put(id, self.ask_row(id, &waiting, SessionAskState::Waiting, None))
                .await;
        }
        let _ = self.published_hosted(id).await;
        Ok(TerminalAsked::Asked { call: waiting.call })
    }

    /// One poll for a terminal question's answer, held up to [`POLL_HOLD`].
    pub(crate) async fn terminal_question_wait(
        &self,
        id: &str,
        call: &str,
    ) -> Result<TerminalAsked, Refusal> {
        let terminals = self.hosts().terminals();
        terminals.polled(call);
        let until = tokio::time::Instant::now() + POLL_HOLD;
        loop {
            // Rung before the look, so an answer between the two is not missed.
            let rung = terminals.rung.notified();
            tokio::pin!(rung);
            rung.as_mut().enable();
            let kept = self.terminal_ask_kept(id).await?;
            match kept.filter(|kept| kept.call == call) {
                None => return Ok(TerminalAsked::Gone {}),
                Some(kept) => match kept.answer {
                    Some(answer) => {
                        return ipc::decode::<TerminalAsked>("a kept answer", answer.as_bytes())
                            .map_err(|why| self.hosted_fault(TERMINAL_ASK_UNREADABLE, &why.why))
                    }
                    None => self.terminal_ask_revived(id, &kept),
                },
            }
            if tokio::time::timeout_at(until, rung).await.is_err() {
                return Ok(TerminalAsked::Waiting {});
            }
        }
    }

    /// The terminal's own prompt ended before Bridge answered: the card closes
    /// and the mod's poll ends.
    pub(crate) async fn terminal_question_settled(
        &self,
        id: &str,
        answered: bool,
    ) -> Result<TerminalAsked, Refusal> {
        let state = if answered {
            SessionAskState::AllowedOnce
        } else {
            SessionAskState::Refused
        };
        self.terminal_ask_closed(id, state).await;
        Ok(TerminalAsked::Gone {})
    }

    /// A person answered a terminal session's question in Bridge: kept for the
    /// mod to collect, and the card closes. **Needs no poll under way.**
    pub(crate) async fn terminal_ask_answered(
        &self,
        id: &str,
        waiting: &HelmCallInFlight,
        said: &Said,
    ) -> Result<(), Refusal> {
        let Some(kept) = self
            .terminal_ask_kept(id)
            .await?
            .filter(|k| k.call == waiting.call)
        else {
            return Ok(());
        };
        let asking: AskingToRun = ipc::decode("a kept question", kept.asking.as_bytes())
            .map_err(|why| self.hosted_fault(TERMINAL_ASK_UNREADABLE, &why.why))?;
        let (state, asked) = match answering(&asking, said) {
            RunOrNot::Allow { updated_input } => (
                SessionAskState::AllowedOnce,
                TerminalAsked::Answered { updated_input },
            ),
            RunOrNot::Deny { message } => {
                (SessionAskState::Refused, TerminalAsked::Refused { message })
            }
        };
        let body = self.kept_text(&asked)?;
        self.store()
            .lock()
            .await
            .answer_terminal_ask(id, &waiting.call, &body)
            .map_err(|why| self.ledger_fault(why))?;
        let via = {
            let host = self.hosts().of(id);
            let mut held = host.state();
            held.asked = None;
            held.answered_via.take()
        };
        self.hosts().terminals().forget_polls(&waiting.call);
        if waiting.tool == ipc::ASKS_A_QUESTION {
            self.row_put(id, self.ask_row(id, waiting, state, via)).await;
        }
        self.hosts().terminals().rung.notify_waiters();
        Ok(())
    }

    /// Close a terminal session's standing question without an answer for the
    /// mod: its card settles at `state`, the kept question goes, and a poll
    /// under way ends. Where there is none, nothing happens.
    pub(crate) async fn terminal_ask_closed(&self, id: &str, state: SessionAskState) {
        let Ok(Some(kept)) = self.terminal_ask_kept(id).await else {
            return;
        };
        let waiting = self.hosts().of(id).state().asked.take();
        let waiting = match waiting {
            Some(waiting) => Some(waiting),
            None => ipc::decode::<HelmCallInFlight>("a kept call", kept.in_flight.as_bytes()).ok(),
        };
        let _ = self.store().lock().await.drop_terminal_ask(id);
        self.hosts().terminals().forget_polls(&kept.call);
        self.hosts().terminals().rung.notify_waiters();
        let _ = self.hosts().asks().withdraw(&kept.call);
        // An answered one already closed its card.
        if let (Some(waiting), None) = (waiting, kept.answer) {
            if waiting.tool == ipc::ASKS_A_QUESTION {
                self.row_put(id, self.ask_row(id, &waiting, state, None)).await;
            }
            let _ = self.published_hosted(id).await;
        }
    }

    /// Put a kept, unanswered terminal question back on the table and on the
    /// session, as a Fleet that restarted lost it, and close one whose mod has
    /// stopped polling: its prompt is gone, so Bridge must not offer the card.
    pub(crate) async fn terminal_ask_standing(&self, id: &str) {
        let Ok(Some(kept)) = self.terminal_ask_kept(id).await else {
            self.terminal_cards_closed(id).await;
            return;
        };
        if kept.answer.is_some() {
            return;
        }
        self.terminal_ask_revived(id, &kept);
        if self.hosts().terminals().since_polled(&kept.call) > POLL_LAPSE {
            self.terminal_ask_closed(id, SessionAskState::Unanswered)
                .await;
        }
    }

    fn terminal_ask_revived(&self, id: &str, kept: &store::KeptTerminalAsk) {
        let standing = self
            .hosts()
            .of(id)
            .state()
            .asked
            .as_ref()
            .map(|a| a.call == kept.call);
        if standing == Some(true) {
            return;
        }
        if let Ok(waiting) =
            ipc::decode::<HelmCallInFlight>("a kept call", kept.in_flight.as_bytes())
        {
            self.hosts().asks().restore(waiting.clone());
            self.hosts().of(id).state().asked = Some(waiting);
        }
    }

    /// A waiting card with no question kept behind it, as one from before
    /// questions were kept, can never be answered.
    async fn terminal_cards_closed(&self, id: &str) {
        let Ok(rows) = self.rows_of(id).await else {
            return;
        };
        for row in rows {
            if let SessionRow::Ask {
                ask,
                state: SessionAskState::Waiting,
                ..
            } = &row
            {
                self.row_put(id, self.ask_row(id, ask, SessionAskState::Unanswered, None))
                    .await;
            }
        }
        self.hosts().of(id).state().asked = None;
    }

    fn kept_text<T: serde::Serialize>(&self, value: &T) -> Result<String, Refusal> {
        ipc::encode(value)
            .map_err(|why| self.hosted_fault(TERMINAL_ASK_UNREADABLE, &format!("{why:?}")))
    }

    async fn terminal_ask_kept(&self, id: &str) -> Result<Option<store::KeptTerminalAsk>, Refusal> {
        self.store()
            .lock()
            .await
            .terminal_ask(id)
            .map_err(|why| self.ledger_fault(why))
    }

    fn ask_row(
        &self,
        id: &str,
        waiting: &HelmCallInFlight,
        state: SessionAskState,
        via: Option<ipc::Via>,
    ) -> SessionRow {
        SessionRow::Ask {
            id: format!("ask-{}-{id}", waiting.call),
            at: self.instant(),
            ask: waiting.clone(),
            state,
            via,
        }
    }
}
