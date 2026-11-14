//! A Job's routine question, answered by the Operator once the person has said Always for it, or once one of
//! his own answers in the same project already covers it.
//!
//! **Fleet detects; the call only reads.** The review call says per question whether it is `sure` and
//! what the question is `about`. A question it is sure of, about nothing he would see, no change of the
//! plan's scope and nothing that could lose work, and that Autopilot's own check finds no destructive
//! word in, is offered as a go-ahead: Yes answers it with the pick, Always here and Always keep the rule
//! `question:answer`. **Under a rule it is answered as it arrives and never queued**; anything else is
//! queued for him as before. An answer is a row of the Log and one line in the Job's thread.
//!
//! **A precedent is the second way.** Where the call is as sure, finds the question about nothing he would see and
//! says which answer of his, or which setting, it follows, and Fleet finds that citation true, the rule
//! `question:precedent` answers it with that citation in the Job's line. Without that rule it is queued with the
//! citation beside it, and his Always here or Always keeps the rule.

use ipc::operator_page::{OperatorFollows, OperatorFollowsKind};
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

/// What a rule for a question that follows an answer or a setting of his is called.
pub(super) const PRECEDENT: &str = "question:precedent";

/// What a question is about, as the review call reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum About {
    /// Something he would see: a look or a UX choice.
    Look,
    /// The answer changes the plan's scope.
    Scope,
    /// The answer could destroy work.
    Lost,
    /// The answer departs from how Classic does it today.
    Departs,
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
                "departs" => Some(About::Departs),
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

/// What a press of Always here or Always on `row` keeps a rule for: **the precedent rule where the pick followed
/// something of his**, else the question rule.
pub(super) fn what_pressed(row: &KeptTurn, item: &WaitingItem) -> Option<String> {
    what_of(item).map(|what| if row.follows.is_some() { PRECEDENT } else { what }.to_string())
}

/// Whether `item` is one the Operator would answer itself: a question it is sure of and that names an
/// option of its own, with no destructive word in it or its options.
pub(super) fn answerable(item: &WaitingItem, sure: &Sureness, read: Option<&OperatorRead>) -> bool {
    what_of(item).is_some() && sure.clear() && read.is_some_and(|read| !read.pick.is_empty()) && !destructive(item)
}

/// What `questions_answered` made of each question it was handed.
pub(super) struct Answers {
    /// Whether it was answered, here by a rule or a precedent.
    pub(super) done: Vec<bool>,
    /// The citation Fleet found true, on a question that passes in everything but a precedent rule to carry it out:
    /// it is queued with this beside it.
    pub(super) follows: Vec<Option<OperatorFollows>>,
}

/// One ruling to carry out: how it is told, kept and journaled.
struct Carry<'a> {
    /// The rule in Fleet's words.
    said: &'a str,
    /// `rule` or `precedent`, as the kept turn names how it was answered.
    answer: &'static str,
    /// The pick as the kept turn holds it.
    kept_pick: String,
    follows: Option<&'a OperatorFollows>,
    /// The line the Job is told.
    line: String,
    journal: String,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// How the Job is told what an answer followed.
fn following_of(follows: &OperatorFollows, date: &str) -> String {
    match follows.kind {
        OperatorFollowsKind::Setting => format!("following his saved setting {}", follows.said),
        OperatorFollowsKind::Answer => format!(
            "following the person's answer to \"{}\" in {} on {date}: \"{}\"",
            follows.question.as_deref().unwrap_or_default().trim(),
            follows.job.as_deref().unwrap_or("a Job"),
            follows.said
        ),
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
    /// The pick of the rule that covers a question in `manifest_id`, read in the order every rule is.
    fn question_pick(&self, rules: &[(i64, OperatorRule)], manifest_id: &str) -> Option<String> {
        rule_pick(rules, QUESTION, manifest_id, &self.rule_projects(manifest_id))
    }

    /// Whether the Operator is on and a rule covers the questions of `manifest_id`, a precedent rule among them,
    /// so Autopilot leaves them to it.
    pub(crate) async fn operator_answers_questions(&self, manifest_id: &str) -> bool {
        if !self.operator_on() {
            return false;
        }
        let rules = self.rules_kept().await;
        self.question_pick(&rules, manifest_id).is_some() || rule_pick(&rules, PRECEDENT, manifest_id, &self.rule_projects(manifest_id)).is_some()
    }

    /// `at` as the person's clock dates it, such as `13 Nov 2026`.
    async fn date_said(&self, at: &str) -> String {
        let Some(millis) = core_model::Timestamp::from_rfc3339(at).epoch_millis() else { return at.to_string() };
        let reading = crate::clock::reading_at(millis, crate::clock::local_offset_secs(millis).await);
        format!("{} {} {}", reading.day, MONTHS[usize::try_from(reading.month - 1).unwrap_or(0).min(11)], reading.year)
    }

    /// Each reviewed question a rule covers, answered with the pick: a `question:answer` rule first, then a
    /// `question:precedent` rule where the pick follows an answer or a setting of his that Fleet finds true.
    /// Answers, for each of `taken`, whether it was, and the citation of one that only lacked the precedent rule.
    /// **A rule that cannot be carried out leaves its item to the person.**
    pub(super) async fn questions_answered(self: &Arc<Self>, taken: &[ToRaise], reviewed: &Reviewed, standing: impl Fn(&ToRaise) -> bool) -> Answers {
        let rules = if self.operator_on() { self.rules_kept().await } else { Vec::new() };
        let mut out = Answers { done: vec![false; taken.len()], follows: vec![None; taken.len()] };
        for (at, one) in taken.iter().enumerate() {
            let (read, sure) = (reviewed.reads[at].as_ref(), &reviewed.sure[at]);
            if !answerable(&one.item, sure, read) || !standing(one) {
                continue;
            }
            let Some(read) = read else { continue };
            let text = one.item.text.trim();
            if let Some(pick) = self.question_pick(&rules, &one.manifest_id) {
                let said = rule_said(QUESTION, &pick);
                let ruling = Carry {
                    kept_pick: format!("{said}: {}", read.pick),
                    line: format!("The Operator answered \"{text}\" with \"{}\", by the rule \"{said}\".", read.pick),
                    journal: format!("By a rule, {said}: {}: {}. Chose: {}", one.job.as_deref().unwrap_or("A Job"), one.item.text, read.pick),
                    said: &said,
                    answer: "rule",
                    follows: None,
                };
                out.done[at] = self.ruling_carried(one, read, reviewed, ruling).await;
                continue;
            }
            let Some(follows) = self.follows_checked(one, &reviewed.follows[at]).await else { continue };
            let Some(pick) = rule_pick(&rules, PRECEDENT, &one.manifest_id, &self.rule_projects(&one.manifest_id)) else {
                out.follows[at] = Some(follows);
                continue;
            };
            let said = rule_said(PRECEDENT, &pick);
            let following = following_of(&follows, &match follows.at.as_ref() {
                Some(when) => self.date_said(when.as_str()).await,
                None => String::new(),
            });
            let ruling = Carry {
                kept_pick: read.pick.clone(),
                line: format!("The Operator answered \"{text}\" with \"{}\", {following}.", read.pick),
                journal: format!("By a precedent, {said}: {}: {}. Chose: {}, {following}", one.job.as_deref().unwrap_or("A Job"), one.item.text, read.pick),
                said: &said,
                answer: "precedent",
                follows: Some(&follows),
            };
            if self.ruling_carried(one, read, reviewed, ruling).await {
                out.done[at] = true;
            } else {
                out.follows[at] = Some(follows);
            }
        }
        out
    }

    /// Carry out `ruling` for `one`, keep its settled turn, journal it and tell the Job. `false` where it could not be
    /// done, which leaves the item to the person.
    async fn ruling_carried(self: &Arc<Self>, one: &ToRaise, read: &OperatorRead, reviewed: &Reviewed, ruling: Carry<'_>) -> bool {
        let doing = Doing { session: &one.session, item: &one.item, pick: &read.pick, reason: &read.read, rule: Some(ruling.said) };
        if let Err(fault) = self.turn_performed(&doing, None).await {
            let (super::turning::Fault::NotOffered(why) | super::turning::Fault::NotDone(why)) = fault;
            self.operator_warned("a rule could not be carried out, so its item is left to the person", why);
            return false;
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
            pick: Some(ruling.kept_pick),
            raised_at: at_now.as_str().to_string(),
            answer: Some(ruling.answer.to_string()),
            settled_at: Some(at_now.as_str().to_string()),
            call: reviewed.call.clone(),
            follows: ruling.follows.and_then(|follows| ipc::encode(follows).ok()),
            ..KeptTurn::default()
        };
        if let Err(why) = self.store().lock().await.add_operator_turn(&row) {
            self.operator_warned("a question a rule answered was not kept", why);
        }
        self.operator_journaled(&ruling.journal).await;
        let _ = self.told_by_fleet(&one.session, &ruling.line, ipc::Notice::Operator).await;
        true
    }
}
