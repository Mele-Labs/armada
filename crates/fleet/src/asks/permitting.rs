//! Carrying one held call to a person and the answer back. `#1389`.
//!
//! **Three things happen here and the order matters**: the ask goes on the
//! table, the person is told, and only then is the call held. A publish before
//! the insert would put a card in front of a person that `answer_call`
//! could not find.
//!
//! `super` is the table and the words; this is the part that needs a
//! whole Fleet — a clock, an event stream, a repository's root to write a
//! remembered rule into.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AnswerCall, AskingToRun, CallAskingToRun, CallAnswer, CallAnswered,
    CallInFlight, CallSettled, CallsWaiting, Instant, ManifestId, RunOrNot, WireError,
};

use super::{answering, restarted, unanswered, NotAnswerable, Said};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// An answer naming nothing that is waiting.
const CALL_NOT_WAITING: &str = "fleet.call_not_waiting";
/// An answer that call did not offer.
const ANSWER_NOT_OFFERED: &str = "fleet.answer_not_offered";

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
    /// Hold one call open while a person answers it in the dock.
    ///
    /// **The bound is the answer, not the hold's own end.** Silence runs out
    /// into a `deny` carrying how long it waited, so a session tells its person
    /// what it was waiting on rather than dying inside a tool call.
    pub(crate) async fn call_permission(
        &self,
        asking: AskingToRun,
        manifest_id: Option<ManifestId>,
    ) -> Result<RunOrNot, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let on_the_wire = ManifestId::carried(served.manifest().id().as_str());
        let asks = self.asks();
        let (waiting, answer) = asks.minted(&on_the_wire, &asking, &self.now());
        // **Read after the ask is minted and before it is published**, so the
        // call that runs unasked carries the same tool, detail and rule a card
        // would have shown — one producer of those, never two. Nothing is on
        // the table yet, so there is nothing to take off. `#1525`.
        if crate::auto_gate::because(&asking).is_none() {
            asks.withdraw(&waiting.call);
            return Ok(self.settled(
                &waiting,
                CallSettled::RanUnasked,
                RunOrNot::Allow {
                    updated_input: asking.input,
                },
            ));
        }
        let call = waiting.call.clone();
        let held = asks.hold();
        self.events()
            .publish(ipc::Event::CallAskingToRun(CallAskingToRun {
                waiting: waiting.clone(),
            }));
        let said = match tokio::time::timeout(held, answer).await {
            Ok(Ok(said)) => said,
            // Fleet is stopping: answered rather than held, so the call is not cut. `#2663`.
            Ok(Err(_)) if asks.stopping() => {
                return Ok(self.settled(&waiting, CallSettled::Unanswered, restarted()))
            }
            // The sender went without sending, which only `withdraw` does.
            Ok(Err(_)) => {
                return Ok(self.settled(
                    &waiting,
                    CallSettled::SessionGone,
                    unanswered(held.as_secs()),
                ))
            }
            Err(_) => {
                asks.withdraw(&call);
                return Ok(self.settled(
                    &waiting,
                    CallSettled::Unanswered,
                    unanswered(held.as_secs()),
                ));
            }
        };
        let settled = self.recorded(&served, &waiting, &said);
        Ok(self.settled(&waiting, settled, answering(&asking, &said)))
    }

    /// What is waiting on a person right now.
    pub(crate) fn calls_waiting(&self) -> CallsWaiting {
        CallsWaiting {
            waiting: self.asks().waiting(),
        }
    }

    /// A person's answer, into the call waiting on it.
    ///
    /// **It answers with the list that is left**, so the dock takes the card
    /// down on the same round trip and does not wait for the event to arrive.
    pub(crate) fn call_answered(
        &self,
        said: AnswerCall,
    ) -> Result<CallsWaiting, Refusal> {
        match self.asks().answer(&said) {
            Ok(_) => Ok(self.calls_waiting()),
            Err(NotAnswerable::NothingWaiting) => Err(Refusal::IllegalMove(WireError::raised(
                CALL_NOT_WAITING,
                "nothing is waiting under that call. It was answered already, or the job \
                 stopped waiting",
                self.run_id(),
            ))),
            Err(NotAnswerable::NotOffered) => Err(Refusal::IllegalMove(WireError::raised(
                ANSWER_NOT_OFFERED,
                "that call does not offer that answer",
                self.run_id(),
            ))),
            // An ask with no session offers no folder or "like this" answer, so one never reaches those checks.
            Err(NotAnswerable::Incomplete | NotAnswerable::NotAFolderAbove | NotAnswerable::NotItsKind) => Err(Refusal::IllegalMove(WireError::raised(
                ANSWER_NOT_OFFERED,
                "that call asked questions, and each needs an answer",
                self.run_id(),
            ))),
        }
    }

    /// Write the rule where a person said to remember it, and say which of the
    /// three allows this was.
    ///
    /// **A settings file that will not be written is not a refusal.** The
    /// person said yes; the call runs, and the record says the rule did not
    /// land.
    fn recorded(
        &self,
        served: &Served,
        waiting: &CallInFlight,
        said: &Said,
    ) -> CallSettled {
        match said.answer {
            CallAnswer::AllowOnce => CallSettled::AllowedOnce,
            // An ask with no session never offers the folder answers: it has no checkout of its own.
            CallAnswer::Refuse | CallAnswer::AlwaysBlock => CallSettled::Refused,
            CallAnswer::AlwaysAllow
            | CallAnswer::AllowForSession
            | CallAnswer::AllowLikeThisForSession => CallSettled::AllowedOnce,
            CallAnswer::AllowAndRemember => {
                let at = adapters::personal_settings(served.root());
                match adapters::remember_the_rule(&at, &waiting.rule) {
                    Ok(_) => CallSettled::AllowedAndRemembered,
                    Err(_) => CallSettled::AllowedButNotRemembered,
                }
            }
        }
    }

    /// Publish what became of one ask, and hand back what the session is told.
    fn settled(
        &self,
        waiting: &CallInFlight,
        settled: CallSettled,
        decided: RunOrNot,
    ) -> RunOrNot {
        self.events()
            .publish(ipc::Event::CallAnswered(CallAnswered {
                call: waiting.call.clone(),
                manifest_id: waiting.manifest_id.clone(),
                tool: waiting.tool.clone(),
                detail: waiting.detail.clone(),
                rule: waiting.rule.clone(),
                settled,
                at: Instant::from(&self.now()),
            }));
        decided
    }
}

/// The shipped hold: fourteen minutes.
///
/// **Chosen against three bounds and it is the smallest.** A reply's whole
/// process ends at `super::hosting::REPLY_BUDGET`, fifteen minutes, so a hold
/// of fourteen ends before the reply does; spike 15
/// measured a stdio permission tool abandoned near thirty minutes, and the door
/// a hosted Job holds is stdio; and spike 19 measured a two-minute hold
/// surviving intact. Beyond that it is a person's patience rather than a
/// process's. It was five minutes until 13 Oct 2026, when the owner found a
/// question lapsing before he came back to it.
pub const SHIPPED_ASK_HOLD: Duration = Duration::from_secs(14 * 60);
