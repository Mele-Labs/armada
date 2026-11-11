//! A Job's routine question, answered by the Operator once the person has said Always for it.
//!
//! **Fleet detects; the call only reads.** The review call says per question whether it is `sure` and
//! what the question is `about`. A question it is sure of, about nothing he would see, no change of the
//! plan's scope and nothing that could lose work, and that Autopilot's own check finds no destructive
//! word in, is offered as a go-ahead: Yes answers it with the pick, Always here and Always keep the rule
//! `question:answer`. **Under a rule it is answered as it arrives and never queued**; anything else is
//! queued for him as before. An answer is a row of the Log and one line in the Job's thread.

use ipc::{OperatorRead, WaitingItem, WaitingSource};
use store::KeptTurn;

use super::raising::{Reviewed, ToRaise};
use super::turning::{rule_pick, rule_said, Doing};
use crate::autopilot::destructive::destructive;
use crate::autopilot::sorting::question;
use crate::daemon::Fleet;
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_page::OperatorRule;
use std::sync::Arc;

/// What a rule for a routine question is called: the kind of arrival, then the way.
pub(super) const QUESTION: &str = "question:answer";

/// What a question is about, as the review call reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum About {
    /// Something he would see: a look or a UX choice.
    Look,
    /// The answer changes the plan's scope.
    Scope,
    /// The answer could destroy work.
    Lost,
    /// None of those.
    Nothing,
}

/// How sure the call was of a question. **Nothing unreadable is sure.**
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sureness {
    sure: Option<bool>,
    about: Option<About>,
}

impl Sureness {
    /// The two lines of one block, as written.
    pub(super) fn from_words(sure: Option<&str>, about: Option<&str>) -> Sureness {
        let bare = |text: &str| text.trim().trim_matches(['"', '.']).trim().to_lowercase();
        Sureness {
            sure: sure.map(bare).and_then(|said| match said.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            }),
            about: about.map(bare).and_then(|said| match said.as_str() {
                "look" => Some(About::Look),
                "scope" => Some(About::Scope),
                "lost" => Some(About::Lost),
                "none" => Some(About::Nothing),
                _ => None,
            }),
        }
    }

    /// Sure, and about none of the things that are his.
    fn clear(&self) -> bool {
        self.sure == Some(true) && self.about == Some(About::Nothing)
    }
}

/// Whether `item` is a question: a card or a Job's own item that offers options, as Autopilot reads one.
pub(crate) fn is_question(item: &WaitingItem) -> bool {
    matches!(item.source, WaitingSource::AskCard | WaitingSource::Agent) && question(item).is_some()
}

/// What a rule for `item` is called, where it is a question a rule could answer.
pub(super) fn what_of(item: &WaitingItem) -> Option<&'static str> {
    is_question(item).then_some(QUESTION)
}

/// Whether `item` is one the Operator would answer itself: a question it is sure of and that names an
/// option of its own, with no destructive word in it or its options.
pub(super) fn answerable(item: &WaitingItem, sure: &Sureness, read: Option<&OperatorRead>) -> bool {
    what_of(item).is_some() && sure.clear() && read.is_some_and(|read| !read.pick.is_empty()) && !destructive(item)
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
    /// The pick of the rule that covers a question in `manifest_id`, read in the order every rule is.
    fn question_pick(&self, rules: &[(i64, OperatorRule)], manifest_id: &str) -> Option<String> {
        rule_pick(rules, QUESTION, manifest_id, &self.rule_projects(manifest_id))
    }

    /// Whether the Operator is on and a rule covers the questions of `manifest_id`, so Autopilot leaves them to it.
    pub(crate) async fn operator_answers_questions(&self, manifest_id: &str) -> bool {
        self.operator_on() && self.question_pick(&self.rules_kept().await, manifest_id).is_some()
    }

    /// Each reviewed question a rule covers, answered with the pick. Answers, for each of `taken`, whether
    /// it was. **A rule that cannot be carried out leaves its item to the person.**
    pub(super) async fn questions_answered(self: &Arc<Self>, taken: &[ToRaise], reviewed: &Reviewed, standing: impl Fn(&ToRaise) -> bool) -> Vec<bool> {
        let rules = if self.operator_on() { self.rules_kept().await } else { Vec::new() };
        let mut done = vec![false; taken.len()];
        for (at, one) in taken.iter().enumerate() {
            let (read, sure) = (reviewed.reads[at].as_ref(), &reviewed.sure[at]);
            if !answerable(&one.item, sure, read) || !standing(one) {
                continue;
            }
            let (Some(read), Some(pick)) = (read, self.question_pick(&rules, &one.manifest_id)) else { continue };
            let said = rule_said(QUESTION, &pick);
            let doing = Doing { session: &one.session, item: &one.item, pick: &read.pick, rule: Some(&said) };
            if let Err(fault) = self.turn_performed(&doing, None).await {
                let (super::turning::Fault::NotOffered(why) | super::turning::Fault::NotDone(why)) = fault;
                self.operator_warned("a rule could not be carried out, so its item is left to the person", why);
                continue;
            }
            let at_now = self.now();
            let row = KeptTurn {
                session: one.session.clone(),
                item: one.item.id.clone(),
                manifest_id: one.manifest_id.clone(),
                job: one.job.clone(),
                go_ahead: true,
                body: ipc::operator_page::item_kept(&one.item),
                read_text: Some(read.read.clone()),
                pick: Some(format!("{said}: {}", read.pick)),
                raised_at: at_now.as_str().to_string(),
                answer: Some("rule".to_string()),
                settled_at: Some(at_now.as_str().to_string()),
                call: reviewed.call.clone(),
                ..KeptTurn::default()
            };
            if let Err(why) = self.store().lock().await.add_operator_turn(&row) {
                self.operator_warned("a question a rule answered was not kept", why);
            }
            self.operator_journaled(&format!("By a rule, {said}: {}: {}. Chose: {}", one.job.as_deref().unwrap_or("A Job"), one.item.text, read.pick)).await;
            let line = format!("The Operator answered \"{}\" with \"{}\", by the rule \"{said}\".", one.item.text.trim(), read.pick);
            let _ = self.told_by_fleet(&one.session, &line, ipc::Notice::Operator).await;
            done[at] = true;
        }
        done
    }
}
