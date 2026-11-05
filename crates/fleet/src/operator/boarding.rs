//! The Operator's board, as Bridge draws it: the arrival a call is out on, those waiting their turn in
//! the order they will be taken, and the last rulings. `crates/ipc/operations/get_operator_board.toml`.
//!
//! **`next` is the queue's `pending`**: what arrived while a call was out, before the next pass takes
//! it. A person's move reorders it and a pass takes it from the front. Items already in a call are
//! shown first and cannot be moved, since a call has been handed them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::operator_board::{MoveOnOperatorBoard, OperatorBoard, OperatorBoardItem, OperatorBoardKind, OperatorBoardSettled};
use store::KeptRuling;

use super::{held, Arrived, Operator};
use crate::daemon::Fleet;

/// How many rulings the board keeps.
const DONE_SHOWN: u32 = 20;
/// The most a ruling's "what" says.
const WHAT_SHOWN: usize = 60;
const BOARD_ITEM_UNKNOWN: &str = "fleet.operator_board_item_unknown";

/// One arrival the Operator has not ruled on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnBoard {
    pub id: String,
    pub arrived: Arrived,
    pub session_id: String,
    pub manifest_id: String,
    pub subject: String,
    pub words: String,
    pub handed: Timestamp,
    /// In the call that is out, or in a pass about to make one: `put_before` cannot move it.
    pub taken: bool,
}

/// What waits for the Operator: the arrival a call is out on, and the rest in the order they will be taken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queued {
    pub now: Option<OnBoard>,
    pub next: Vec<OnBoard>,
}

/// An id that is not in the queue where it must be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unknown(pub String);

fn on_board(one: &super::Held, taken: bool) -> OnBoard {
    let arrival = &one.arrival;
    OnBoard {
        id: arrival.id.clone(),
        arrived: arrival.arrived,
        session_id: arrival.session_id.clone(),
        manifest_id: arrival.manifest_id.clone(),
        subject: arrival.subject.clone(),
        words: arrival.words.clone(),
        handed: one.handed.clone(),
        taken,
    }
}

impl Operator {
    /// What waits now. **Items in a pass come first**, then those arrived since, in the order a person left them.
    pub fn queued(&self) -> Queued {
        let queue = held(&self.queue);
        let mut all: Vec<OnBoard> = queue.out.iter().map(|one| on_board(one, true)).collect();
        all.extend(queue.pending.iter().map(|one| on_board(one, false)));
        let now = match queue.call {
            Some(_) if !all.is_empty() => Some(all.remove(0)),
            _ => None,
        };
        Queued { now, next: all }
    }

    /// `id` goes before `before` among those that have not been taken, or last where `before` is
    /// `None`. **Unknown** names the id that is not there to move or to move before.
    pub fn put_before(&self, id: &str, before: Option<&str>) -> Result<(), Unknown> {
        {
            let mut queue = held(&self.queue);
            let from = queue.pending.iter().position(|one| one.arrival.id == id).ok_or_else(|| Unknown(id.to_string()))?;
            if before == Some(id) {
                return Ok(());
            }
            if let Some(before) = before {
                if !queue.pending.iter().any(|one| one.arrival.id == before) {
                    return Err(Unknown(before.to_string()));
                }
            }
            let one = queue.pending.remove(from);
            let at = before.and_then(|before| queue.pending.iter().position(|other| other.arrival.id == before));
            match at {
                Some(at) => queue.pending.insert(at, one),
                None => queue.pending.push(one),
            }
        }
        self.desk.board_changed();
        Ok(())
    }

    /// `id` goes first among those that have not been taken.
    pub fn put_first(&self, id: &str) -> Result<(), Unknown> {
        let first = held(&self.queue).pending.first().map(|one| one.arrival.id.clone());
        self.put_before(id, first.as_deref())
    }
}

/// How a kind of arrival is shown on the board.
pub fn kind_of(arrived: Arrived) -> OperatorBoardKind {
    match arrived {
        Arrived::Start | Arrived::Plan => OperatorBoardKind::Start,
        Arrived::Question => OperatorBoardKind::Question,
        Arrived::Card => OperatorBoardKind::Card,
        Arrived::Stopped | Arrived::Red => OperatorBoardKind::Stall,
        Arrived::Report | Arrived::Issue | Arrived::RedMain | Arrived::Review | Arrived::Later => OperatorBoardKind::Report,
    }
}

fn first_line(text: &str) -> String {
    text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_string()
}

fn short(text: &str) -> String {
    let line = first_line(text);
    if line.chars().count() <= WHAT_SHOWN {
        return line;
    }
    let cut: String = line.chars().take(WHAT_SHOWN - 1).collect();
    format!("{}…", cut.trim_end())
}

/// How long ago `handed` was, as `12s`, `3m` or `2h`. `None` where either instant cannot be read.
fn waited(handed: &Timestamp, now: &Timestamp) -> Option<String> {
    let seconds = (now.epoch_millis()? - handed.epoch_millis()?).max(0) / 1000;
    Some(match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3600),
    })
}

/// A ruling in `Kind: what`, or the kind alone. `title_of` names a Job by its id.
fn outcome_of(row: &KeptRuling, title_of: &dyn Fn(&str) -> String) -> String {
    let said = |kind: &str, what: String| match short(&what) {
        what if what.is_empty() => kind.to_string(),
        what => format!("{kind}: {what}"),
    };
    let other = || row.other_session.as_deref().map(title_of).unwrap_or_default();
    match row.outcome.as_str() {
        "answer" => said("Answered", row.answer.clone().unwrap_or_default()),
        "raise" => said("Raised", row.because.clone()),
        "new_job" if matches!(Arrived::from_word(&row.arrived), Some(Arrived::Stopped | Arrived::Red)) => said("Started", "the rest as a new Job".to_string()),
        "new_job" => "Started".to_string(),
        "existing_job" => said("Handed over", format!("to {}", other())),
        "merge" => said("Merged", format!("into {}", other())),
        "keep_in_touch" => said("Kept in touch", format!("with {}", other())),
        "nudge" => "Nudged".to_string(),
        "link" => said("Linked", row.issue.clone().unwrap_or_default()),
        "file" => said("Filed", row.issue.clone().unwrap_or_default()),
        "passed_on" => "Passed on".to_string(),
        _ => "Settled".to_string(),
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
    /// The name of the repository `manifest_id` is.
    fn board_repository(&self, manifest_id: &str) -> Option<String> {
        self.repositories().hosted().into_iter().find(|one| one.manifest().id().as_str() == manifest_id).map(|one| {
            let root = one.root();
            std::path::Path::new(root).file_name().map_or_else(|| root.to_string(), |name| name.to_string_lossy().to_string())
        })
    }

    /// `get_operator_board`: read now.
    pub(crate) async fn operator_board_wire(&self) -> Result<OperatorBoard, Refusal> {
        let queued = self.operator_seat().operator().map(|operator| operator.queued()).unwrap_or_default();
        let now = self.now();
        let item = |one: OnBoard| OperatorBoardItem {
            repository: self.board_repository(&one.manifest_id),
            waits: waited(&one.handed, &now),
            kind: kind_of(one.arrived),
            session_id: Some(one.session_id).filter(|id| !id.is_empty()),
            title: first_line(&one.subject),
            id: one.id,
        };
        let done = {
            let store = self.store().lock().await;
            let rows = store.operator_rulings_signed(DONE_SHOWN).map_err(|why| self.ledger_fault(why))?;
            let title_of = |id: &str| store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
            rows.iter()
                .map(|row| OperatorBoardSettled {
                    id: row.arrival.clone(),
                    kind: kind_of(Arrived::from_word(&row.arrived).unwrap_or(Arrived::Start)),
                    session_id: Some(row.session_id.clone()).filter(|id| !id.is_empty()),
                    title: first_line(&row.subject),
                    repository: self.board_repository(&row.manifest_id),
                    outcome: outcome_of(row, &title_of),
                    at: row.at.clone(),
                })
                .collect()
        };
        Ok(OperatorBoard { on: self.operator_on(), now: queued.now.map(&item), next: queued.next.into_iter().map(item).collect(), done })
    }

    /// `move_on_operator_board`: one press. **404** where `id`, or the `before` it goes ahead of,
    /// is not among those waiting.
    pub(crate) async fn operator_board_moved(&self, moved: MoveOnOperatorBoard) -> Result<OperatorBoard, Refusal> {
        let unknown = |id: &str| {
            Refusal::NotFound(ipc::WireError::raised(BOARD_ITEM_UNKNOWN, format!("{id} is not waiting for the Operator, so it cannot be moved"), self.run_id()))
        };
        let Some(operator) = self.operator_seat().operator() else { return Err(unknown(&moved.id)) };
        operator.put_before(&moved.id, moved.before.as_deref()).map_err(|Unknown(id)| unknown(&id))?;
        self.operator_board_wire().await
    }
}
