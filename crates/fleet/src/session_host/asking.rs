//! A hosted session's permission door: the call is put to the person on the
//! session's own thread, and the answer goes back inside the call. Since 23.49.
//!
//! **Helm's asking machinery, not a second one**: the same table, the same
//! reading of what is destructive, pushes to a shared space or writes off this
//! machine, and the same words back to the model. What differs is where the ask
//! is drawn, which is the thread and never Helm's dock, and which calls reach a
//! person: `auto` asks only about those three classes, `ask` and `acceptEdits`
//! ask about everything the person's own settings do not cover.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal, Sessions};
use ipc::{
    AskingToRun, HelmCallAnswer, HelmCallInFlight, ManifestId, RunOrNot, SessionAskState,
    SessionMode, SessionRow, TerminalAsked,
};

use super::serving::mode_of;
use crate::daemon::Fleet;
use crate::helm::{answering, because_in_a_session, pages_opened, unanswered, Because};

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
        let asks = self.hosts().asks();
        let manifest = ManifestId::carried(session.manifest_id.as_deref().unwrap_or_default());
        let (waiting, answer) = asks.minted(&manifest, &asking, &self.now());
        self.hosts().of(id).state().asked = Some(waiting.clone());
        self.row_put(id, self.ask_row(id, &waiting, SessionAskState::Waiting))
            .await;
        let _ = self.published_hosted(id).await;

        let held = asks.hold();
        let (state, asked) = match tokio::time::timeout(held, answer).await {
            Ok(Ok(said)) => match answering(&asking, &said) {
                RunOrNot::Allow { updated_input } => {
                    (SessionAskState::AllowedOnce, TerminalAsked::Answered { updated_input })
                }
                RunOrNot::Deny { message } => {
                    (SessionAskState::Refused, TerminalAsked::Refused { message })
                }
            },
            // The terminal got there first: its settling wrote the row.
            Ok(Err(_)) => return Ok(TerminalAsked::Gone {}),
            Err(_) => {
                let _ = asks.withdraw(&waiting.call);
                (SessionAskState::Unanswered, TerminalAsked::Gone {})
            }
        };
        self.hosts().of(id).state().asked = None;
        self.row_put(id, self.ask_row(id, &waiting, state)).await;
        let _ = self.published_hosted(id).await;
        Ok(asked)
    }

    /// The terminal's own prompt ended before Bridge answered: the card closes
    /// and the held request ends.
    pub(crate) async fn terminal_question_settled(
        &self,
        id: &str,
        answered: bool,
    ) -> Result<TerminalAsked, Refusal> {
        let waiting = self.hosts().of(id).state().asked.take();
        if let Some(waiting) = waiting {
            let _ = self.hosts().asks().withdraw(&waiting.call);
            let state = if answered {
                SessionAskState::AllowedOnce
            } else {
                SessionAskState::Refused
            };
            self.row_put(id, self.ask_row(id, &waiting, state)).await;
            let _ = self.published_hosted(id).await;
        }
        Ok(TerminalAsked::Gone {})
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
