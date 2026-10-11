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
    AskingToRun, HelmCallAnswer, HelmCallInFlight, ManifestId, RunOrNot, SessionAskState,
    SessionMode, SessionRow, TerminalAsked,
};

use super::serving::mode_of;
use crate::daemon::Fleet;
use crate::helm::{answering, because_in_a_session, pages_opened, unanswered, Because, Said};

/// How long one `Wait` is held. The mod's request must outlast it.
const POLL_HOLD: Duration = Duration::from_secs(25);

/// How long a terminal question may go without a poll before its prompt is
/// taken as gone and its card closes.
pub(crate) const POLL_LAPSE: Duration = Duration::from_secs(180);

/// A kept terminal question that would not encode or read back.
const TERMINAL_ASK_UNREADABLE: &str = "fleet.terminal_ask_unreadable";

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
        self.hosts().all().into_iter().find_map(|(id, runtime)| {
            let pid = runtime.state().process.as_ref().and_then(|p| p.pid())?;
            crate::peer::held_within(caller, self.host().port, &[pid], self.peers().as_ref())
                .then_some(id)
        })
    }

    pub(crate) async fn session_permission(
        &self,
        id: &str,
        asking: AskingToRun,
    ) -> Result<RunOrNot, Refusal> {
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
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let mode = mode_of(&hosting.mode);
        // A question is for the person whatever the mode: nothing else can
        // answer it, and the agent is told what they chose.
        // **In `auto` a session never waits on the person** (the owner, 8 Oct
        // 2026: "JUST LET IT DO WHAT IT NEEDS", after it sat blocked while he
        // was away). A call runs, or, where it would reach into his own files
        // or take away something outside the slot and `/tmp`, is refused with
        // the reason, which the agent reads and works around. Only a question
        // is put to him.
        let put_to_a_person = asking.tool_name == ipc::ASKS_A_QUESTION
            || match mode {
                SessionMode::Ask | SessionMode::AcceptEdits => true,
                SessionMode::Auto | SessionMode::Plan => {
                    match because_in_a_session(&asking, &Self::directory_of(&served, &hosting)) {
                        Some(Because::ReachesOutside) => {
                            return Ok(RunOrNot::Deny {
                                message: String::from(
                                    "Refused: that reaches into the owner's own files outside your slot. Work inside your slot, /tmp, or a path under Armada's own folders.",
                                ),
                            });
                        }
                        Some(Because::Destructive) => {
                            return Ok(RunOrNot::Deny {
                                message: String::from(
                                    "Refused: that removes or overwrites something outside your slot and /tmp. Do it inside your slot, or say in your reply what you need and why.",
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
        let (waiting, answer) = asks.minted(
            &ManifestId::carried(&hosting.manifest_id),
            &asking,
            &self.now(),
        );
        let row_id = format!("ask-{}-{id}", waiting.call);
        let put = |state: SessionAskState| SessionRow::Ask {
            id: row_id.clone(),
            at: self.instant(),
            ask: waiting.clone(),
            state,
        };
        self.hosts().of(id).state().asked = Some(waiting.clone());
        self.row_put(id, put(SessionAskState::Waiting)).await;
        let _ = self.published_hosted(&session.id).await;

        let held = asks.hold();
        let (decided, state) = match tokio::time::timeout(held, answer).await {
            Ok(Ok(said)) => {
                let state = match said.answer {
                    HelmCallAnswer::AllowOnce => SessionAskState::AllowedOnce,
                    HelmCallAnswer::Refuse => SessionAskState::Refused,
                    HelmCallAnswer::AllowAndRemember => {
                        let at = adapters::personal_settings(served.root());
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
        self.hosts().of(id).state().asked = None;
        self.row_put(id, put(state)).await;
        let _ = self.published_hosted(id).await;
        Ok(decided)
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
                &format!("no terminal session is named {id}"),
            )
        })?;
        // One at a time: a new question means the last one's prompt is gone.
        self.terminal_ask_closed(id, SessionAskState::Unanswered)
            .await;
        let asks = self.hosts().asks();
        let manifest = ManifestId::carried(session.manifest_id.as_deref().unwrap_or_default());
        // The channel is not waited on; the answer is kept by whoever gives it.
        let (waiting, _) = asks.minted(&manifest, &asking, &self.now());
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
        self.row_put(id, self.ask_row(id, &waiting, SessionAskState::Waiting))
            .await;
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
        self.hosts().of(id).state().asked = None;
        self.hosts().terminals().forget_polls(&waiting.call);
        self.row_put(id, self.ask_row(id, waiting, state)).await;
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
            self.row_put(id, self.ask_row(id, &waiting, state)).await;
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
                self.row_put(id, self.ask_row(id, ask, SessionAskState::Unanswered))
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

    fn ask_row(&self, id: &str, waiting: &HelmCallInFlight, state: SessionAskState) -> SessionRow {
        SessionRow::Ask {
            id: format!("ask-{}-{id}", waiting.call),
            at: self.instant(),
            ask: waiting.clone(),
            state,
        }
    }
}
