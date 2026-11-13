//! A Job's sketches, drafts and pages as one record: `get_job_artifacts`.
//! `crates/ipc/operations/get_job_artifacts.toml` is the contract, `docs/concepts/artifact.md` the rules.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

/// One artifact a Job made, across every revision of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobArtifact {
    /// Stable across revisions, the ledger target it is kept under: `sketch:<title>`, the page's path, or
    /// `draft:<title>`.
    pub id: String,
    /// The newest revision's title.
    pub title: String,
    /// The newest revision's form.
    pub form: ArtifactForm,
    /// Oldest first. Never empty.
    pub revisions: Vec<ArtifactRevision>,
    /// Where the newest revision stands.
    pub state: ArtifactState,
    /// His note on a send back, his words with an approval, or his reason with a draft's dismissal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The Needs you item that answers it now: `sketch:<title>`, `document:<path>` or `draft:<row id>`.
    /// Absent where nothing waits on him.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Who folded it away, and when. Only `restore_artifact` lifts it; a new revision does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<ArtifactArchived>,
    /// What the newest revision is for, and what it links to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_line: Option<ArtifactFor>,
}

/// An artifact the agent or he archived as stale. `archive_artifact`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactArchived {
    pub by: ArtifactBy,
    pub at: Instant,
}

/// `archive_artifact` and `restore_artifact`: one artifact of a Job, by its [`JobArtifact::id`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// The artifact's id: `sketch:<title>`, the page's path, or `draft:<title>`.
    pub artifact: String,
    /// Which Job holds it. **Read only where the connection places no hosted session**, as `WithdrawSketch`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// On a sketch or page's attachment: `agent` where the Job took it back with `withdraw_sketch` or
/// `withdraw_document`, so it reads `withdrawn` and his Dismiss reads `dismissed`.
pub const ARTIFACT_WITHDRAWN_BY: &str = "withdrawn_by";

/// One revision of an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRevision {
    /// 1 and on, in the order they were kept.
    pub number: u32,
    /// When it was kept.
    pub at: Instant,
    pub by: ArtifactBy,
    pub form: ArtifactForm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactForm {
    Sketch,
    Draft,
    /// A document: an HTML page under `.armada/documents/`.
    Page,
}

/// Who wrote a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBy {
    Agent,
    Owner,
}

/// Where an artifact's newest revision stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    /// For reading: nothing was put to him.
    Reading,
    /// Put to him and not answered.
    Asking,
    Approved,
    SentBack,
    /// Taken back by the Job, replaced by a draft of the same title, or a page whose file is gone.
    Withdrawn,
    /// He passed on it with no verdict.
    Dismissed,
}

/// What an artifact is for: one sentence from the agent, the links it gave, then the links Fleet knows.
/// On a revision as it was kept, so a record reads the same later. `docs/concepts/artifact.md`, *The For line*.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactFor {
    /// What it is for and what his answer changes. At most [`MOST_FOR_CHARS`] characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The agent's own first, then Fleet's. At most [`MOST_FOR_LINKS`] of the agent's.
    #[serde(default)]
    pub links: Vec<ArtifactLink>,
}

/// One link on a For line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLink {
    pub kind: ArtifactLinkKind,
    pub label: String,
    /// A plan group's id, a plan task's id, `#<n>`, or an `http(s)` address.
    pub target: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactLinkKind {
    /// The plan group of the task the Job has in progress. Fleet's.
    PlanStep,
    /// The plan task the Job has in progress. Fleet's.
    Task,
    /// An open pull request the Job holds. Fleet's.
    PullRequest,
    /// An address the agent gave.
    Link,
}

/// A link as the agent gives it to `show_sketch` or `propose_draft`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForLink {
    pub label: String,
    /// `http://` or `https://`, else the call is refused.
    pub url: String,
}

/// The longest For sentence, in characters.
pub const MOST_FOR_CHARS: usize = 200;
/// The most links the agent gives one artifact.
pub const MOST_FOR_LINKS: usize = 5;

/// On an artifact's detail: its [`ArtifactFor`] as JSON text. Absent where it has none.
pub const ARTIFACT_FOR: &str = "for";

/// Whether `url` is an address a For link may hold.
fn is_web(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    ["http://", "https://"].iter().any(|scheme| lower.strip_prefix(scheme).is_some_and(|rest| !rest.is_empty()))
}

fn link_of(link: &ForLink) -> ArtifactLink {
    let url = link.url.trim().to_string();
    let label = link.label.trim();
    ArtifactLink { kind: ArtifactLinkKind::Link, label: if label.is_empty() { url.clone() } else { label.to_string() }, target: url }
}

impl ArtifactFor {
    /// The agent's For line off a tool call, or every reason it is refused in one sentence an agent can act on.
    /// `None` where it gave neither.
    pub fn checked(text: Option<&str>, links: &[ForLink]) -> Result<Option<ArtifactFor>, String> {
        let text = text.map(str::trim).filter(|text| !text.is_empty());
        let mut problems = Vec::new();
        if let Some(count) = text.map(|text| text.chars().count()).filter(|count| *count > MOST_FOR_CHARS) {
            problems.push(format!("`for` is {count} characters and holds at most {MOST_FOR_CHARS}. Say what it is for in one sentence"));
        }
        if links.len() > MOST_FOR_LINKS {
            problems.push(format!("`links` holds {} and takes at most {MOST_FOR_LINKS}", links.len()));
        }
        for (at, link) in links.iter().enumerate() {
            if !is_web(&link.url) {
                problems.push(format!("links[{at}] `{}` is `{}`, and a link must start http:// or https://", link.label.trim(), link.url.trim()));
            }
        }
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        let links: Vec<ArtifactLink> = links.iter().map(link_of).collect();
        Ok((text.is_some() || !links.is_empty()).then(|| ArtifactFor { text: text.map(str::to_string), links }))
    }

    /// A page's For line, which is never refused: the first [`MOST_FOR_CHARS`] characters, and the first
    /// [`MOST_FOR_LINKS`] `http(s)` links. Each link it dropped is named in the second.
    pub fn kept(text: Option<&str>, links: &[ForLink]) -> (Option<ArtifactFor>, Vec<String>) {
        let text = text.map(str::trim).filter(|text| !text.is_empty()).map(|text| text.chars().take(MOST_FOR_CHARS).collect::<String>());
        let mut dropped = Vec::new();
        let mut kept = Vec::new();
        for link in links {
            let named = format!("`{}` ({})", link.label.trim(), link.url.trim());
            if !is_web(&link.url) {
                dropped.push(format!("{named}, which is not http:// or https://"));
            } else if kept.len() == MOST_FOR_LINKS {
                dropped.push(format!("{named}, past the first {MOST_FOR_LINKS}"));
            } else {
                kept.push(link_of(link));
            }
        }
        ((text.is_some() || !kept.is_empty()).then(|| ArtifactFor { text, links: kept }), dropped)
    }

    /// `links` after the agent's, where neither the agent nor an earlier one has the same target. A pull
    /// request the agent linked by its address is not linked again.
    pub fn with_links(said: Option<ArtifactFor>, links: Vec<ArtifactLink>) -> Option<ArtifactFor> {
        let mut whole = said.unwrap_or_default();
        for link in links {
            let number = link.target.trim_start_matches('#');
            let held = whole.links.iter().any(|one| {
                one.target == link.target
                    || (link.kind == ArtifactLinkKind::PullRequest
                        && one.kind == ArtifactLinkKind::Link
                        && one.target.trim_end_matches('/').ends_with(&format!("/pull/{number}")))
            });
            if !held {
                whole.links.push(link);
            }
        }
        (whole.text.is_some() || !whole.links.is_empty()).then_some(whole)
    }

    /// An artifact's For line off its detail, where it holds one that reads.
    pub fn of_detail(detail: &std::collections::BTreeMap<String, String>) -> Option<ArtifactFor> {
        crate::decode("an artifact's for line", detail.get(ARTIFACT_FOR)?.as_bytes()).ok()
    }
}
