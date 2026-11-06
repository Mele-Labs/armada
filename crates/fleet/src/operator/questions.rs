//! A question or a permission card, as the Operator is handed it, and the answers Fleet's rules
//! leave it. **Fleet's rules decide first and only narrow**: an item they let it answer goes to a
//! call, which may still raise it; one they keep for the person is raised with no call.
//!
//! **Whether the person would see the result is Fleet's**, read from the seen mark of the plan
//! group the Job works on. A question from a Job on a seen group, or with no plan, is raised. A
//! second Job asking what the person already answered for a first is offered his earlier answer.
//! While Autopilot runs he is away, and its setup widens what may be answered.

use ipc::AutopilotDecision;

use super::arriving::{Arrival, Arrived, Source};
use super::ruling::Way;

/// The card's answer the Operator may give: never one that allows more than this call.
pub const ALLOW_ONCE: &str = "Allow once";

/// What Fleet's rules make of the item, as Autopilot sorts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A question with the option at this place recommended.
    Recommended(usize),
    /// A question with options and none recommended.
    Open,
    /// A card for a tool that only reads.
    Reads,
    /// A card for an edit inside the Job's slots.
    Edits,
    /// A card for a command Fleet can read whole.
    Runs,
}

/// The plan group the Job works on, by its seen mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Group {
    /// The group, by its title, changes nothing the person sees.
    Unseen(String),
    /// The group, by its title, changes what the person sees.
    Seen(String),
    NoPlan,
}

/// Whether the person is here, as the Operator's and Autopilot's switches read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Present {
    /// The Operator is off: nothing is answered.
    Off,
    Here,
    /// Autopilot runs for the Job: the kinds its setup checks may be answered, and its
    /// instruction is shown to the call.
    Away { lets: Vec<AutopilotDecision>, instruction: String },
}

/// The person's answer when another Job asked the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Earlier {
    pub session_id: String,
    pub title: String,
    pub answer: String,
}

/// One answer the item may be given: the option at `at`, as his press would choose it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub at: u32,
    pub label: String,
    /// Where it is his earlier answer to another Job.
    pub earlier: Option<Earlier>,
}

/// One item waiting on the person with an Answer path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub session_id: String,
    pub manifest_id: String,
    /// The Job's title.
    pub title: String,
    /// The item's id on the Job's Needs you list.
    pub item_id: String,
    /// What was asked: the question, or the tool and what it would do.
    pub text: String,
    /// The options in order: a question's labels, or a card's offers.
    pub options: Vec<String>,
    pub kind: Kind,
    pub group: Group,
    pub earlier: Option<Earlier>,
    pub present: Present,
}

impl Question {
    /// The arrival Fleet hands over, named so the same item arrives once; `None` while the
    /// Operator is off, or where no answer it may give is among the options.
    pub fn arrival(&self) -> Option<Arrival> {
        if self.present == Present::Off {
            return None;
        }
        let card = matches!(self.kind, Kind::Reads | Kind::Edits | Kind::Runs);
        let (answers, rule) = self.answers();
        if answers.is_empty() && rule.is_none() {
            return None;
        }
        let mut words = match card {
            true => format!("It asks to use a tool: {}.", self.text.trim()),
            false => format!("It asks: {}", self.text.trim()),
        };
        if !card {
            let options: Vec<String> = self.options.iter().enumerate().map(|(at, one)| format!("{}. {one}", at + 1)).collect();
            words.push_str(&format!(" Its options: {}.", options.join("; ")));
            match &self.group {
                Group::Unseen(title) => words.push_str(&format!(" It works on plan group \"{title}\", which changes nothing the person sees.")),
                Group::Seen(title) => words.push_str(&format!(" It works on plan group \"{title}\", which the person sees.")),
                Group::NoPlan => words.push_str(" It has recorded no plan."),
            }
        }
        if let Present::Away { instruction, .. } = &self.present {
            words.push_str(&format!(" The person is away, and left this instruction: {}", instruction.trim()));
        }
        Some(Arrival {
            id: asked_id(&self.session_id, &self.item_id),
            arrived: if card { Arrived::Card } else { Arrived::Question },
            source: Source::Agent,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.text.clone(),
            words,
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways: match answers.is_empty() {
                true => vec![Way::Raise],
                false => vec![Way::Answer, Way::Raise],
            },
            answers,
            rule,
            open: Vec::new(),
        })
    }

    /// The answers Fleet's rules let the call give, or the rule that keeps the item for the person.
    fn answers(&self) -> (Vec<Choice>, Option<String>) {
        let lets = |kind: AutopilotDecision| matches!(&self.present, Present::Away { lets, .. } if lets.contains(&kind));
        let choice = |at: usize, earlier: Option<Earlier>| Choice { at: at as u32, label: self.options[at].clone(), earlier };
        match self.kind {
            Kind::Reads | Kind::Edits | Kind::Runs => {
                let Some(at) = self.options.iter().position(|one| one == ALLOW_ONCE) else { return (Vec::new(), None) };
                match self.kind == Kind::Runs && !lets(AutopilotDecision::Writes) {
                    true => (Vec::new(), Some("it runs a command, which is the person's to allow while he is here".into())),
                    false => (vec![choice(at, None)], None),
                }
            }
            Kind::Recommended(_) | Kind::Open => {
                // His own answer to the same question is offered whatever group the Job is on.
                let earlier = self.earlier.as_ref().and_then(|one| Some((self.options.iter().position(|label| *label == one.answer)?, one)));
                if let Some((at, earlier)) = earlier {
                    return (vec![choice(at, Some(earlier.clone()))], None);
                }
                let decision = match self.kind {
                    Kind::Recommended(_) => AutopilotDecision::Recommended,
                    _ => AutopilotDecision::Open,
                };
                let kept = match &self.group {
                    Group::Seen(title) => Some(format!("it works on plan group \"{title}\", which the person sees")),
                    Group::NoPlan => Some("its Job has no plan, so whether the person would see the result is not known".into()),
                    Group::Unseen(_) => None,
                };
                if let Some(rule) = kept.filter(|_| !lets(decision)) {
                    return (Vec::new(), Some(rule));
                }
                match self.kind {
                    Kind::Recommended(at) if at < self.options.len() => (vec![choice(at, None)], None),
                    _ => ((0..self.options.len()).map(|at| choice(at, None)).collect(), None),
                }
            }
        }
    }
}

/// The arrival's id for item `item` of Job `session`: `asked:<session>:<item>`.
pub fn asked_id(session: &str, item: &str) -> String {
    format!("asked:{session}:{item}")
}

/// The item an arrival named by [`asked_id`] is about. A Job's id holds no colon, so the first
/// after the prefix ends it, and an item's own colons stay in the item.
pub fn asked_item(arrival: &Arrival) -> Option<&str> {
    arrival.id.strip_prefix("asked:")?.split_once(':').map(|(_, item)| item)
}
