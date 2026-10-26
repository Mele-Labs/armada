//! What a workflow ends in, which the Job view's Landing draws: a pull
//! request, a report, a spike's findings, or a document. A Mod Job is not a
//! workflow's word; Bridge reads it off the Job.

/// The four things a workflow lands in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lands {
    PullRequest,
    Report,
    Spike,
    Document,
}

impl Lands {
    pub const ALL: &'static [Lands] = &[Lands::PullRequest, Lands::Report, Lands::Spike, Lands::Document];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Lands::PullRequest => "pull_request",
            Lands::Report => "report",
            Lands::Spike => "spike",
            Lands::Document => "document",
        }
    }

    pub fn from_wire(value: &str) -> Option<Lands> {
        Lands::ALL.iter().copied().find(|lands| lands.as_wire() == value)
    }

    /// What a workflow lands in where it declares nothing: **a pull request
    /// where a step delivers, a document otherwise**. Spelled once, here.
    pub fn of(declared: Option<Lands>, delivers: bool) -> Lands {
        declared.unwrap_or(if delivers { Lands::PullRequest } else { Lands::Document })
    }
}
