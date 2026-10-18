//! One repository's Helm Session as one quotable record. `#1367`.
//!
//! **What decides an answer, in one artifact**: the repository, the authority
//! and the model; the note appended to its system prompt; the tools the door offered; the
//! thread; and what the last poll was told. A bad answer
//! could only be reported by retyping the conversation without any of it.
//!
//! **The roster is read from the same predicate the door enforces** — `may`
//! over `api::offerable` — so the record cannot name a set the door does not.

use ipc::{
    HelmActionAuthority, HelmDebugLine, HelmDebugSaid, HelmDebugText, SessionAskState, SessionRow,
    SessionVoice,
};

use super::reach::{may, Authority};

/// How many of the thread's own lines a record carries. **A record is pasted
/// into an issue**: past this, what it proves is drowned by what it holds.
const LINES: usize = 160;

/// How much of one piece of prose a line carries. A reply of forty thousand
/// characters is the whole artifact otherwise.
const MOST: usize = 2_000;

/// The tools the door offers a session under `authority`, by name, as the
/// inventory orders them.
pub(super) fn tools(authority: Authority) -> Vec<String> {
    api::offerable()
        .filter(|row| may(authority, row))
        .map(|row| row.operation.to_string())
        .collect()
}

/// The Machine setting as the wire spells it.
pub(super) fn authority_on_the_wire(authority: Authority) -> HelmActionAuthority {
    match authority {
        Authority::Acting => HelmActionAuthority::Acting,
        Authority::ReadOnly => HelmActionAuthority::ReadOnly,
    }
}

/// The thread's last [`LINES`] lines, and how many were left out.
///
/// **What was cut is counted, never inferred.** The count is what this bound left behind, so a
/// record that is short says by how much.
pub(super) fn thread(rows: &[SessionRow]) -> (Vec<HelmDebugLine>, u64) {
    let lines: Vec<HelmDebugLine> = rows.iter().filter_map(line).collect();
    let cut = lines.len().saturating_sub(LINES);
    (lines.into_iter().skip(cut).collect(), cut as u64)
}

/// One thread row as the record holds it, or nothing where it holds none.
///
/// **A Session that wrote to Helm is read as an ask**, under its own title, because it is a message
/// Helm answered; everything the record has no word for (a lease, a window, a check) is left out.
fn line(row: &SessionRow) -> Option<HelmDebugLine> {
    let (at, said) = match row {
        SessionRow::Message { at, from, text, .. } => {
            let said = match from {
                SessionVoice::You => HelmDebugSaid::Asked {
                    text: HelmDebugText::bounded(text, MOST),
                },
                SessionVoice::Agent => HelmDebugSaid::Said {
                    text: HelmDebugText::bounded(text, MOST),
                },
                SessionVoice::Session { title, .. } => HelmDebugSaid::Asked {
                    text: HelmDebugText::bounded(&format!("{title}: {text}"), MOST),
                },
            };
            (at.clone(), said)
        }
        SessionRow::Tool { at, text, .. } => {
            let (tool, detail) = text.split_once(' ').unwrap_or((text.as_str(), ""));
            (
                at.clone(),
                HelmDebugSaid::Called {
                    tool: tool.to_string(),
                    detail: detail.to_string(),
                },
            )
        }
        SessionRow::Ask { at, ask, state, .. } => {
            let because = match state {
                SessionAskState::Refused => "the call was refused",
                SessionAskState::AlwaysBlocked => "the call was refused and its folder blocked",
                SessionAskState::Unanswered => "nobody answered inside the hold",
                SessionAskState::Kept => "the owner was walking a page",
                SessionAskState::SessionGone => "the process went before the answer did",
                _ => return None,
            };
            (
                at.clone(),
                HelmDebugSaid::Refused {
                    tool: ask.tool.clone(),
                    because: because.to_string(),
                },
            )
        }
        SessionRow::Sent {
            at, text, target, ..
        } => (
            at.clone(),
            HelmDebugSaid::Sent {
                text: HelmDebugText::bounded(text, MOST),
                to: target.title.clone(),
            },
        ),
        _ => return None,
    };
    Some(HelmDebugLine { at, said })
}
