//! The card a Job's Commander puts to the owner, and his answer. `#2166`.
//!
//! **A Drone's question, one subject over.** It is asked once, offers a closed
//! set and takes a pick from it, and it waits however long he takes: it is not
//! a held tool call and nothing times it out. There is no field for prose. A
//! person who needs to say something the options do not cover writes to the
//! Commander (`ask_commander`).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId};

/// The id Fleet minted for a card. **What an answer names**, so an answer sent
/// from a window left open across a newer card is a refusal and not a pick read
/// against the wrong options.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CardId(String);

impl CardId {
    pub fn carried(value: impl Into<String>) -> Self {
        CardId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A line of a card that has something in it. Blank does not decode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardText(String);

impl CardText {
    pub fn said(text: &str) -> Option<CardText> {
        let text = text.trim();
        (!text.is_empty()).then(|| CardText(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a card's words or options were refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unusable {
    Blank,
    /// Fewer than two or more than four, or a label blank or said twice.
    Options,
}

impl fmt::Display for Unusable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Unusable::Blank => "a line of a card has nothing in it",
            Unusable::Options => {
                "a card offers two to four options, each with a label, no two alike"
            }
        })
    }
}

impl TryFrom<String> for CardText {
    type Error = Unusable;

    fn try_from(text: String) -> Result<CardText, Unusable> {
        CardText::said(&text).ok_or(Unusable::Blank)
    }
}

impl From<CardText> for String {
    fn from(text: CardText) -> String {
        text.0
    }
}

/// The labels a card offers, in the order asked. **Two to four, each with
/// something in it and no two alike**, a Drone's question's rule, so a surface
/// may draw them as a closed set of controls without checking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct CardOptions(Vec<String>);

impl CardOptions {
    pub fn labelled(labels: Vec<String>) -> Result<CardOptions, Unusable> {
        let labels: Vec<String> = labels.iter().map(|label| label.trim().to_string()).collect();
        let distinct = labels
            .iter()
            .enumerate()
            .all(|(at, label)| !labels[..at].contains(label));
        match (2..=4).contains(&labels.len()) && distinct && labels.iter().all(|one| !one.is_empty())
        {
            true => Ok(CardOptions(labels)),
            false => Err(Unusable::Options),
        }
    }

    pub fn labels(&self) -> &[String] {
        &self.0
    }
}

impl TryFrom<Vec<String>> for CardOptions {
    type Error = Unusable;

    fn try_from(labels: Vec<String>) -> Result<CardOptions, Unusable> {
        CardOptions::labelled(labels)
    }
}

impl From<CardOptions> for Vec<String> {
    fn from(options: CardOptions) -> Vec<String> {
        options.0
    }
}

/// `POST /jobs/:job_id/commander/card` — the one thing a Commander sends the
/// owner. Every line is required: what is wrong, what is out of its reach, the
/// question and the options he picks from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskTheOwner {
    /// What is wrong.
    pub text: CardText,
    /// What is out of the Commander's reach, and so why it is asking.
    pub reach: CardText,
    pub question: CardText,
    pub options: CardOptions,
}

/// Who put a card out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardBy {
    /// The Commander, through `ask_the_owner`.
    Commander,
    /// Fleet, where the loop bound stopped the Commander acting on a problem.
    Fleet,
}

/// One option of a card. `id` is what an answer names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardOption {
    pub id: String,
    pub label: String,
}

/// A card that is out. **A Job has one at a time**, and while it is out the Job
/// counts as waiting on the owner (`JobSummary::asking`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderCard {
    pub card_id: CardId,
    pub job_id: JobId,
    pub by: CardBy,
    pub raised_at: Instant,
    pub text: String,
    pub reach: String,
    pub question: String,
    /// In the order asked. Never fewer than two and never more than four.
    pub options: Vec<CardOption>,
}

/// `POST /jobs/:job_id/commander/card/answer` — his pick. The body names the
/// card and the option and carries no prose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerCommanderCard {
    pub card_id: CardId,
    pub option_id: String,
}

/// What `answer_commander_card` answers with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderCardAnswered {
    pub job_id: JobId,
    pub card_id: CardId,
    pub option_id: String,
    /// The Commander was woken with the answer. `false` where it could not
    /// start (the Job has no worktree), which the Job's log says.
    pub woke: bool,
}

/// `commander.card`: a Job's card went out, or the one out was answered.
///
/// **`job.asking`'s shape.** Two messages per card and never a third: the one
/// going out carries the card, and the one coming back carries none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderCardChanged {
    pub job_id: JobId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CommanderCard>,
}
