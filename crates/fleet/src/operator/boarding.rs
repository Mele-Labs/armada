//! The Operator's board, as Bridge draws it: the arrival a call is out on, those waiting their turn in
//! the order they will be taken, and the last rulings. `crates/ipc/operations/get_operator_board.toml`.
//!
//! **`next` is the queue's `pending`**: what arrived while a call was out, before the next pass takes
//! it. A person's move reorders it and a pass takes it from the front. Items already in a call are
//! shown first and cannot be moved, since a call has been handed them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, Level, Timestamp};
use ipc::operator_board::{
    MoveOnOperatorBoard, OperatorBoard, OperatorBoardChanged, OperatorBoardItem, OperatorBoardKind, OperatorBoardOption, OperatorBoardSettled, OperatorBoardSuggestion, OperatorBoardSwitch, OperatorDecider, OperatorPaused,
};
use store::KeptRuling;

use super::{held, Arrived, Operator};
use crate::daemon::Fleet;

/// How many rulings the board keeps.
const DONE_SHOWN: u32 = 20;
/// How many moves of the master switch the board keeps.
const SWITCHED_SHOWN: u32 = 20;
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
        Arrived::Report | Arrived::Issue | Arrived::RedMain | Arrived::Review | Arrived::Later | Arrived::Note | Arrived::Ended | Arrived::Merged | Arrived::Error => OperatorBoardKind::Report,
    }
}

pub(super) fn first_line(text: &str) -> String {
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

/// A ruling in `Kind: what`, or the kind alone. `title_of` names a Job by its id, and `summary_of`
/// gives what its agent said it achieved.
pub(super) fn outcome_of(row: &KeptRuling, title_of: &dyn Fn(&str) -> String, summary_of: &dyn Fn(&str) -> Option<String>) -> String {
    let said = |kind: &str, what: String| match short(&what) {
        what if what.is_empty() => kind.to_string(),
        what => format!("{kind}: {what}"),
    };
    let other = || row.other_session.as_deref().map(title_of).unwrap_or_default();
    match row.outcome.as_str() {
        "answer" => said("Answered", row.answer.clone().unwrap_or_default()),
        "raise" => said("Raised", row.because.clone()),
        "new_job" if matches!(Arrived::from_word(&row.arrived), Some(Arrived::Stopped | Arrived::Red)) => said("Started", "the rest as a new Job".to_string()),
        "new_job" => said("Started", other()),
        "existing_job" => said("Handed over", format!("to {}", other())),
        "merge" => said("Merged", format!("into {}", other())),
        "keep_in_touch" => said("Kept in touch", format!("with {}", other())),
        "sent" => said("Sent", format!("to {}", other())),
        "link" => said("Linked", row.issue.clone().unwrap_or_default()),
        "file" => said("Filed", row.issue.clone().unwrap_or_default()),
        "close" => said("Closed", row.issue.clone().unwrap_or_default()),
        "group" => said("Grouped", format!("with {}", row.issue.clone().unwrap_or_default())),
        // An end or a merge read and left: what it was is the row.
        "noted" => match Arrived::from_word(&row.arrived) {
            Some(Arrived::Ended) => said("Ended", summary_of(&row.session_id).unwrap_or_default()),
            Some(Arrived::Merged) => said("Merged", format!("#{} {}", row.arrival.rsplit(':').next().unwrap_or_default(), row.subject)),
            _ => "Noted".to_string(),
        },
        "ended" => "Ended".to_string(),
        "passed_on" => "Passed on".to_string(),
        "skipped" => "Skipped".to_string(),
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
    /// What a person reads for where a rule holds: a repository's name, a project's, and nothing for everywhere.
    pub(super) fn scope_named(&self, scope: &ipc::operator_page::RuleScope) -> Option<String> {
        use ipc::operator_page::RuleScope;
        match scope {
            RuleScope::Everywhere => None,
            RuleScope::Repository { manifest_id } => self.board_repository(manifest_id),
            RuleScope::Project { project } => Some(core_model::ScopeId::new(project).ok().and_then(|id| self.projects_held().get(&id).map(|one| one.name().as_str().to_string())).unwrap_or_else(|| project.clone())),
        }
    }

    pub(super) fn board_repository(&self, manifest_id: &str) -> Option<String> {
        self.repositories().hosted().into_iter().find(|one| one.manifest().id().as_str() == manifest_id).map(|one| {
            let root = one.root();
            std::path::Path::new(root).file_name().map_or_else(|| root.to_string(), |name| name.to_string_lossy().to_string())
        })
    }

    /// The board is not what it was: Bridge reads it again.
    pub(super) fn operator_board_changed(&self) {
        self.events().publish(ipc::Event::OperatorBoardChanged(OperatorBoardChanged {}));
    }

    /// The master switch was possibly moved by a save or a hand edit: keep the moment where it differs
    /// from `was_on`, and tell Bridge to read the page and the board again: the page empties Your turn
    /// while off, and the board shows `on` and the moves of the switch. A save that left it alone keeps and tells nothing.
    pub(crate) async fn operator_switched(&self, was_on: bool) {
        let on = self.operator_on();
        if on == was_on {
            return;
        }
        self.operator_page_changed();
        let kept = self.store().lock().await.add_operator_switch(&store::KeptSwitch { on, at: self.now().as_str().to_string() });
        if let Err(why) = kept {
            let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a move of the Operator's switch was not kept")
                .with_field("why", FieldValue::Str(why.to_string()));
            crate::transcript::note_aloud(&envelope);
            return;
        }
        self.operator_board_changed();
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
        let (suggestions, done, switched) = {
            let store = self.store().lock().await;
            let suggestions = store
                .operator_suggestions_of("")
                .map_err(|why| self.ledger_fault(why))?
                .into_iter()
                .map(|one| OperatorBoardSuggestion {
                    id: format!("suggestion:{}", one.n),
                    text: one.text,
                    options: ["Do it", "Skip"].map(|label| OperatorBoardOption { label: label.to_string() }).to_vec(),
                    at: one.raised_at,
                })
                .collect();
            let rows = store.operator_rulings_done(DONE_SHOWN).map_err(|why| self.ledger_fault(why))?;
            let title_of = |id: &str| store.session(id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| id.to_string());
            let summary_of = |id: &str| store.done_summary(id).ok().flatten();
            let done = rows
                .iter()
                .map(|row| OperatorBoardSettled {
                    id: row.arrival.clone(),
                    kind: kind_of(Arrived::from_word(&row.arrived).unwrap_or(Arrived::Start)),
                    session_id: Some(row.session_id.clone()).filter(|id| !id.is_empty()),
                    title: first_line(&row.subject),
                    repository: self.board_repository(&row.manifest_id),
                    outcome: outcome_of(row, &title_of, &summary_of),
                    at: row.at.clone(),
                    by: if row.by == "human" { OperatorDecider::Person } else { OperatorDecider::Operator },
                    because: row.because.trim().to_string(),
                })
                .collect();
            let switched = store
                .operator_switches(SWITCHED_SHOWN)
                .map_err(|why| self.ledger_fault(why))?
                .into_iter()
                .map(|one| OperatorBoardSwitch { on: one.on, at: one.at })
                .collect();
            (suggestions, done, switched)
        };
        let paused = if self.operator_on() { self.operator_paused().await } else { None }.map(|paused| OperatorPaused { week: paused.week, at: paused.at });
        Ok(OperatorBoard { on: self.operator_on(), now: queued.now.map(&item), next: queued.next.into_iter().map(item).collect(), suggestions, done, paused, switched })
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
