//! An adversarial review between the store's rows and the wire, and the message a sent finding is.
//! `adversarial_serving` is what reads and writes them.

use ipc::{
    AdversarialFinding, AdversarialReview, AdversarialReviewState, FindingAnchor, FindingHow, FindingSettled, Instant,
    ReviewSubject, SessionPlan,
};
use store::{KeptAdversarialFinding, KeptAdversarialReview};

pub(crate) const RUNNING: &str = "running";
pub(crate) const DONE: &str = "done";
pub(crate) const FAILED: &str = "failed";

/// What a review whose Fleet stopped before it ended reads as.
pub(crate) const DIED: &str = "Fleet restarted before the review finished";

fn state_of(word: &str) -> AdversarialReviewState {
    match word {
        RUNNING => AdversarialReviewState::Running,
        DONE => AdversarialReviewState::Done,
        _ => AdversarialReviewState::Failed,
    }
}

fn how_of(word: &str) -> FindingHow {
    match word {
        "sent" => FindingHow::Sent,
        _ => FindingHow::Dismissed,
    }
}

pub(crate) fn how_word(how: FindingHow) -> &'static str {
    match how {
        FindingHow::Sent => "sent",
        FindingHow::Dismissed => "dismissed",
    }
}

/// The review as the wire carries it, for the subject it read.
pub(crate) fn review_of(subject: &ReviewSubject, kept: &KeptAdversarialReview) -> AdversarialReview {
    AdversarialReview {
        subject: subject.clone(),
        state: state_of(&kept.state),
        started_at: Instant::carried(&kept.started_at),
        finished_at: kept.finished_at.as_deref().map(Instant::carried),
        failed: kept.failed.clone(),
        findings: kept.findings.iter().map(finding_of).collect(),
        cost_micros: kept.cost_micros,
    }
}

fn finding_of(kept: &KeptAdversarialFinding) -> AdversarialFinding {
    AdversarialFinding {
        id: kept.id.clone(),
        about: match kept.anchor.as_str() {
            "task" => FindingAnchor::Task { id: kept.anchor_id.clone() },
            "group" => FindingAnchor::Group { id: kept.anchor_id.clone() },
            _ => FindingAnchor::Plan,
        },
        consequence: kept.consequence.clone(),
        expected: kept.expected.clone(),
        found: kept.found.clone(),
        cites: kept.cites.clone(),
        settled: kept
            .settled
            .as_ref()
            .map(|(how, at)| FindingSettled { how: how_of(how), at: Instant::carried(at) }),
    }
}

/// A finding as it is kept, with the words of the part of `plan` it is pinned to.
pub(crate) fn kept_of(finding: &AdversarialFinding, plan: &SessionPlan) -> KeptAdversarialFinding {
    let (anchor, anchor_id, part) = match &finding.about {
        FindingAnchor::Plan => ("plan", String::new(), String::new()),
        FindingAnchor::Group { id } => {
            let title = plan.groups.iter().find(|group| &group.id == id).map(|group| group.title.clone());
            ("group", id.clone(), title.unwrap_or_default())
        }
        FindingAnchor::Task { id } => {
            let text = plan.groups.iter().flat_map(|group| &group.tasks).find(|task| &task.id == id).map(|task| task.text.clone());
            ("task", id.clone(), text.unwrap_or_default())
        }
    };
    KeptAdversarialFinding {
        id: finding.id.clone(),
        anchor: anchor.to_string(),
        anchor_id,
        part,
        consequence: finding.consequence.clone(),
        expected: finding.expected.clone(),
        found: finding.found.clone(),
        cites: finding.cites.clone(),
        settled: None,
    }
}

/// What the agent is told when the person sends a finding: the part it is about by its words, then the
/// claim and what it rests on. **No id**: the agent knows its plan by what it wrote.
pub(crate) fn message(finding: &KeptAdversarialFinding) -> String {
    let part = match (finding.anchor.as_str(), finding.part.trim()) {
        ("task", words) if !words.is_empty() => format!("the task \"{words}\""),
        ("group", words) if !words.is_empty() => format!("the group \"{words}\""),
        _ => String::from("the plan as a whole"),
    };
    let mut said = format!(
        "An adversarial review of your plan raised this about {part}:\n\n{}\n\nExpected: {}\nFound: {}",
        finding.consequence, finding.expected, finding.found
    );
    if !finding.cites.is_empty() {
        said.push_str(&format!("\n\nIt points at {}.", finding.cites.join(", ")));
    }
    said
}
