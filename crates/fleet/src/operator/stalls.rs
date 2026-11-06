//! A Job gone quiet with work left, as the Operator is handed it: one Fleet marked Stopped, or one
//! that left its pull request red. **Fleet reads it from state**, never from a transcript.

use super::arriving::{Arrival, Arrived, Source};
use super::ruling::Way;

/// How a Job went quiet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quiet {
    /// Fleet marked it Stopped at `at`: it stopped again after Fleet told it to carry on.
    Stopped { at: String },
    /// Its pull request `number` failed `failing` at `head`, and it ended a turn since with no push.
    Red { number: u64, head: String, failing: String },
}

/// One stall, with the Job's plan read against what is left of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stall {
    pub session_id: String,
    pub manifest_id: String,
    pub title: String,
    pub quiet: Quiet,
    /// Its plan's summary, where it recorded one.
    pub plan: Option<String>,
    /// Its tasks done, and its tasks left, in its plan's order.
    pub done: Vec<String>,
    pub left: Vec<String>,
}

impl Stall {
    /// The arrival Fleet hands over, named so the same stall arrives once.
    pub fn arrival(&self) -> Arrival {
        let (id, arrived, how) = match &self.quiet {
            Quiet::Stopped { at } => (
                format!("stopped:{}:{at}", self.session_id),
                Arrived::Stopped,
                "It stopped again after Fleet told it to carry on.".to_string(),
            ),
            Quiet::Red { number, head, failing } => (
                format!("red:{}:{number}:{head}", self.session_id),
                Arrived::Red,
                format!("#{number} failed its checks at {} ({failing}), and it ended a turn since with no push.", short(head)),
            ),
        };
        let mut words = how;
        match &self.plan {
            Some(plan) => words.push_str(&format!(" Its plan: {}", plan.trim())),
            None => words.push_str(" It recorded no plan."),
        }
        if !self.done.is_empty() {
            words.push_str(&format!(" Done: {}.", self.done.join("; ")));
        }
        match self.left.is_empty() {
            true => words.push_str(" Left: none."),
            false => words.push_str(&format!(" Left: {}.", self.left.join("; "))),
        }
        Arrival {
            id,
            arrived,
            source: Source::Fleet,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words,
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways: self.ways(),
            answers: Vec::new(),
            rule: None,
            open: Vec::new(),
        }
    }

    /// A stop with nothing left has nothing to start; a red pull request always has its
    /// checks left.
    fn ways(&self) -> Vec<Way> {
        let stopped = matches!(self.quiet, Quiet::Stopped { .. });
        if stopped && self.left.is_empty() {
            return vec![Way::Raise];
        }
        vec![Way::NewJob, Way::Raise]
    }
}

fn short(head: &str) -> &str {
    head.get(..7).unwrap_or(head)
}
