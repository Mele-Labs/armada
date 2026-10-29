//! The agent's own `Agent` calls, whose rows carry the subagent each started, and the person's rows
//! written while a foreground one held the turn. armada-d3h.1.
//!
//! **A subagent's calls are never rows of the thread**, which would put its words in the agent's
//! mouth: its latest call is a line on the `Agent` row, put again by id.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, SessionRow, ToolSubagent, ToolSubagentState};

use crate::daemon::Fleet;

/// How often a subagent's latest call puts its row again. A subagent can make several calls a second.
const LATEST_EVERY: Duration = Duration::from_secs(1);
/// How many calls are remembered. One that is never answered or settled is dropped oldest first.
const CALLS_AT_MOST: usize = 64;

/// In memory: a Fleet that restarts settles a row it kept from the thread itself.
#[derive(Default)]
pub(crate) struct CallRows {
    calls: VecDeque<Call>,
    /// The person's rows put `queued`, to be put again once the agent has read them.
    queued: Vec<SessionRow>,
}

struct Call {
    call: String,
    background: bool,
    answered: bool,
    row: Option<Drawn>,
    put_at: Option<std::time::Instant>,
    /// A latest call is held for [`LATEST_EVERY`], and one put is on its way.
    owed: bool,
    /// The mod told of the subagent before the stream drew its call: the two come by different pipes.
    early: Option<ToolSubagent>,
}

struct Drawn {
    id: String,
    at: Instant,
    text: String,
    subagent: Option<ToolSubagent>,
}

impl Drawn {
    fn row(&self) -> SessionRow {
        SessionRow::Tool { id: self.id.clone(), at: self.at.clone(), text: self.text.clone(), subagent: self.subagent.clone() }
    }
}

/// What a subagent's latest call asks of its row.
pub(crate) enum Latest {
    Put(SessionRow),
    After(Duration),
    Held,
}

impl CallRows {
    pub(crate) fn spawned(&mut self, call: &str, background: bool) {
        self.call(call).background = background;
    }

    /// The call `call`, remembered from now where it was not. One the stream has not named yet is
    /// taken as background, so it holds no turn until it is.
    fn call(&mut self, call: &str) -> &mut Call {
        if let Some(at) = self.calls.iter().position(|one| one.call == call) {
            return &mut self.calls[at];
        }
        let one = Call { call: call.to_string(), background: true, answered: false, row: None, put_at: None, owed: false, early: None };
        self.calls.push_back(one);
        while self.calls.len() > CALLS_AT_MOST {
            self.calls.pop_front();
        }
        self.calls.back_mut().expect("a call was just pushed")
    }

    /// The agent's own call was drawn as row `id`: the row to put, where the call started a subagent.
    pub(crate) fn drawn(&mut self, call: &str, id: &str, at: &Instant, text: &str) -> Option<SessionRow> {
        let one = self.calls.iter_mut().find(|one| one.call == call)?;
        let subagent = one.early.take();
        if subagent.is_some() {
            one.put_at = Some(std::time::Instant::now());
        }
        let drawn = Drawn { id: id.to_string(), at: at.clone(), text: text.to_string(), subagent };
        let row = drawn.row();
        one.row = Some(drawn);
        Some(row)
    }

    /// A foreground subagent call is drawn and has not returned, so the CLI holds what the person writes.
    pub(crate) fn holds_the_turn(&self) -> bool {
        self.calls.iter().any(|one| !one.background && !one.answered && one.row.is_some())
    }

    pub(crate) fn queued(&mut self, row: SessionRow) {
        self.queued.push(row);
    }

    /// The agent's own call `call` returned. The person's held rows, once nothing holds the turn.
    pub(crate) fn answered(&mut self, call: &str) -> Vec<SessionRow> {
        if let Some(one) = self.calls.iter_mut().find(|one| one.call == call) {
            one.answered = true;
        }
        match self.holds_the_turn() {
            true => Vec::new(),
            false => self.read(),
        }
    }

    /// The person's held rows, read: the turn ended, or the process went.
    pub(crate) fn read(&mut self) -> Vec<SessionRow> {
        std::mem::take(&mut self.queued)
            .into_iter()
            .map(|row| match row {
                SessionRow::Message { id, at, from, text, files, tags, notice, .. } => {
                    SessionRow::Message { id, at, from, text, files, tags, notice, queued: false }
                }
                other => other,
            })
            .collect()
    }

    /// The subagent call `call` started, running. Its row, where the call was drawn; else it waits for the row.
    pub(crate) fn started(&mut self, call: &str, subagent: ToolSubagent) -> Option<SessionRow> {
        let one = self.call(call);
        let Some(drawn) = one.row.as_mut() else {
            one.early = Some(subagent);
            return None;
        };
        drawn.subagent = Some(subagent);
        one.put_at = Some(std::time::Instant::now());
        Some(drawn.row())
    }

    pub(crate) fn latest(&mut self, call: &str, latest: String, now: std::time::Instant) -> Latest {
        let Some(one) = self.calls.iter_mut().find(|one| one.call == call) else { return Latest::Held };
        let Some(subagent) = one.row.as_mut().and_then(|drawn| drawn.subagent.as_mut()) else { return Latest::Held };
        if subagent.state != ToolSubagentState::Running {
            return Latest::Held;
        }
        subagent.latest = Some(latest);
        if one.owed {
            return Latest::Held;
        }
        let since = one.put_at.map_or(LATEST_EVERY, |at| now.saturating_duration_since(at));
        if since < LATEST_EVERY {
            one.owed = true;
            return Latest::After(LATEST_EVERY - since);
        }
        one.put_at = Some(now);
        one.row.as_ref().map_or(Latest::Held, |drawn| Latest::Put(drawn.row()))
    }

    /// The put a held latest call was waiting for, unless it settled meanwhile.
    pub(crate) fn owed(&mut self, call: &str, now: std::time::Instant) -> Option<SessionRow> {
        let one = self.calls.iter_mut().find(|one| one.call == call && one.owed)?;
        one.owed = false;
        one.put_at = Some(now);
        let drawn = one.row.as_ref()?;
        drawn.subagent.as_ref().filter(|subagent| subagent.state == ToolSubagentState::Running)?;
        Some(drawn.row())
    }

    /// Subagent `target` settled. Its row, where this Fleet drew it; the call is forgotten.
    pub(crate) fn settled(&mut self, target: &str, state: ToolSubagentState, ended: &Instant) -> Option<SessionRow> {
        let at = self.calls.iter().position(|one| {
            one.row.as_ref().and_then(|drawn| drawn.subagent.as_ref()).is_some_and(|subagent| subagent.id == target)
        })?;
        let mut one = self.calls.remove(at)?;
        let drawn = one.row.as_mut()?;
        let subagent = drawn.subagent.as_mut()?;
        subagent.state = state;
        subagent.ended = Some(ended.clone());
        Some(drawn.row())
    }

    /// The process went, and every call with it.
    pub(crate) fn forgotten(&mut self) {
        self.calls.clear();
    }
}

/// `row`, settled, where it is a call whose subagent `pick` names and is still running.
fn settled_row(row: SessionRow, pick: impl Fn(&str) -> bool, state: ToolSubagentState, ended: &Instant) -> Option<SessionRow> {
    match row {
        SessionRow::Tool { id, at, text, subagent: Some(mut subagent) }
            if subagent.state == ToolSubagentState::Running && pick(&subagent.id) =>
        {
            subagent.state = state;
            subagent.ended = Some(ended.clone());
            Some(SessionRow::Tool { id, at, text, subagent: Some(subagent) })
        }
        _ => None,
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
    /// The mod told of subagent `target`, started by the agent's call `call`: that call's row carries it.
    pub(crate) async fn subagent_started(&self, id: &str, target: &str, call: &str, detail: &BTreeMap<String, String>) {
        let runtime = self.hosts().of(id);
        let row = {
            let mut state = runtime.state();
            let agent_type = detail.get("type").cloned().or_else(|| state.spawns.kind_of(call).map(str::to_string));
            let subagent = ToolSubagent {
                id: target.to_string(),
                agent_type,
                task: detail.get("description").cloned().unwrap_or_default(),
                latest: None,
                state: ToolSubagentState::Running,
                since: self.instant(),
                ended: None,
            };
            state.call_rows.started(call, subagent)
        };
        if let Some(row) = row {
            self.row_put(id, row).await;
        }
    }

    /// Subagent `call`'s latest call, `latest`: its row is put again at most every [`LATEST_EVERY`].
    pub(crate) async fn subagent_called(self: &Arc<Self>, id: &str, call: &str, latest: String) {
        let runtime = self.hosts().of(id);
        let now = std::time::Instant::now();
        let said = runtime.state().call_rows.latest(call, latest, now);
        match said {
            Latest::Put(row) => self.row_put(id, row).await,
            Latest::After(wait) => {
                let (fleet, id, call) = (Arc::clone(self), id.to_string(), call.to_string());
                tokio::spawn(async move {
                    tokio::time::sleep(wait).await;
                    let row = fleet.hosts().of(&id).state().call_rows.owed(&call, std::time::Instant::now());
                    if let Some(row) = row {
                        fleet.row_put(&id, row).await;
                    }
                });
            }
            Latest::Held => {}
        }
    }

    /// Subagent `target` settled as `state`. **A row this Fleet did not draw is read off the thread**,
    /// so one a restart of Fleet came between still settles.
    pub(crate) async fn subagent_settled(&self, id: &str, target: &str, state: ToolSubagentState) {
        let ended = self.instant();
        let kept = self.hosts().of(id).state().call_rows.settled(target, state, &ended);
        let rows = match kept {
            Some(row) => vec![row],
            None => self.running_rows(id, |one| one == target, state, &ended).await,
        };
        for row in rows {
            self.row_put(id, row).await;
        }
    }

    /// The process went: every subagent still running on the thread is cut off, and the person's
    /// held rows are read, since what was written to the process is gone or sent again.
    pub(crate) async fn calls_let_go(&self, id: &str) {
        let read = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            state.call_rows.forgotten();
            state.call_rows.read()
        };
        let ended = self.instant();
        let cut = self.running_rows(id, |_| true, ToolSubagentState::CutOff, &ended).await;
        for row in read.into_iter().chain(cut) {
            self.row_put(id, row).await;
        }
    }

    async fn running_rows(
        &self,
        id: &str,
        pick: impl Fn(&str) -> bool,
        state: ToolSubagentState,
        ended: &Instant,
    ) -> Vec<SessionRow> {
        let Ok(rows) = self.rows_of(id).await else { return Vec::new() };
        rows.into_iter().filter_map(|row| settled_row(row, &pick, state, ended)).collect()
    }
}
