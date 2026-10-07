//! A hosted session's thread: rows kept and published one at a time, the line
//! Fleet adds for what a message names, and what a message carried.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use api::Sessions;
use ipc::{
    AttachmentReport, Instant, SessionFact, SessionId, SessionReport, SessionRow, SessionRowChanged, SessionTag,
    TagKind, WireError,
};
use serde::Serialize;

use crate::daemon::Fleet;

/// A row that would not encode or a thread that would not read.
pub(super) const SESSION_THREAD_UNREADABLE: &str = "fleet.session_thread_unreadable";

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
    pub(crate) fn instant(&self) -> Instant {
        Instant::from(&self.now())
    }

    pub(crate) fn row_id(&self, session: &str) -> String {
        self.hosts()
            .next_row(session, &self.now().as_str().replace([':', '-', '.'], ""))
    }

    /// Keep a row and publish it. **Best effort**: a thread that will not write
    /// costs the row its place in the record and not the person the turn.
    pub(crate) async fn row_put(&self, session: &str, row: SessionRow) {
        if let Ok(body) = ipc::encode(&row) {
            let _ = self
                .store()
                .lock()
                .await
                .keep_session_row(session, row.id(), &body);
        }
        self.publish(ipc::Event::SessionRow(SessionRowChanged {
            session_id: SessionId::carried(session),
            row,
        }));
    }

    /// A document the session wrote goes on its ledger as an artifact. The
    /// terminal mod tells the same fact from its own side; see `session.md`.
    pub(crate) async fn artifact_written(&self, session: &str, path: String) {
        let title = path.rsplit('/').next().unwrap_or(&path).to_string();
        let _ = self
            .report_session(SessionReport {
                harness: String::from(adapters::HOSTED_HARNESS),
                session_id: SessionId::carried(session),
                fact: SessionFact::Attached {
                    attachment: AttachmentReport {
                        kind: String::from("artifact"),
                        target: path,
                        detail: [("form".into(), "file".into()), ("title".into(), title)].into(),
                    },
                },
            })
            .await;
    }

    /// The thread, oldest first.
    pub(crate) async fn rows_of(&self, session: &str) -> Result<Vec<SessionRow>, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .session_rows(session)
            .map_err(|why| self.ledger_fault(why))?;
        kept.iter()
            .map(|body| {
                ipc::decode::<SessionRow>("a session row", body.as_bytes()).map_err(|why| {
                    Refusal::Fault(WireError::raised(
                        SESSION_THREAD_UNREADABLE,
                        format!("a row of the session's thread would not read: {}", why.why),
                        self.run_id(),
                    ))
                })
            })
            .collect()
    }
}

/// The line Fleet adds ahead of a message that names something. **Written for
/// the agent**, and never shown on the thread, which keeps the person's words.
pub(crate) fn mention_line(tags: &[(SessionTag, Option<String>)]) -> Option<String> {
    if tags.is_empty() {
        return None;
    }
    let named: Vec<String> = tags
        .iter()
        .map(|(tag, address)| match tag.kind {
            TagKind::Session => match address {
                Some(address) => format!(
                    "session \"{}\" (address {address}; write to it with SendMessage)",
                    tag.title
                ),
                None => format!(
                    "session \"{}\" (id {}; find its address with ListAgents)",
                    tag.title, tag.id
                ),
            },
            TagKind::Job => match &tag.job {
                Some(job) => format!(
                    "Job {} \"{}\" (id {}, branch {}, slot {}, {})",
                    job.number, tag.title, tag.id, job.branch, job.slot, job.state
                ),
                None => format!("Job \"{}\" (id {})", tag.title, tag.id),
            },
            TagKind::PullRequest => format!("pull request #{} \"{}\"", tag.id, tag.title),
            TagKind::Branch => format!("branch {}", tag.id),
        })
        .collect();
    Some(format!("The person named: {}.", named.join("; ")))
}

/// A file name as one path component: anything that is not a letter, a digit,
/// `.`, `-` or `_` is written as `_`, so no name can leave its directory.
pub(crate) fn safe_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let kept = kept.trim_start_matches('.').to_string();
    if kept.is_empty() {
        String::from("file")
    } else {
        kept
    }
}

/// One line of the agent CLI's input: a person's turn.
#[derive(Serialize)]
pub(crate) struct UserLine {
    #[serde(rename = "type")]
    kind: &'static str,
    message: UserMessage,
}

#[derive(Serialize)]
struct UserMessage {
    role: &'static str,
    content: Vec<Block>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block {
    Text { text: String },
    Image { source: Source },
}

#[derive(Serialize)]
struct Source {
    #[serde(rename = "type")]
    kind: &'static str,
    media_type: String,
    data: String,
}

impl UserLine {
    /// `text` and each picture as `(media type, base64)`.
    pub(crate) fn of(text: String, pictures: Vec<(String, String)>) -> UserLine {
        let mut content = vec![Block::Text { text }];
        content.extend(pictures.into_iter().map(|(media_type, data)| Block::Image {
            source: Source {
                kind: "base64",
                media_type,
                data,
            },
        }));
        UserLine {
            kind: "user",
            message: UserMessage {
                role: "user",
                content,
            },
        }
    }
}

/// A uuid, as the agent CLI insists a session id be. **Random, version 4.**
pub(crate) fn new_id() -> String {
    let mut bytes: [u8; 16] = rand::random();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}
