//! Graphs, as the fake answers them: a show is checked and echoed, and nothing is kept or run.

use ipc::{GraphList, GraphPoints, GraphPointsTaken, GraphReading, GraphShown, Instant, SessionId, ShowGraph, WithdrawGraph};

use super::FakeDaemon;
use crate::{Caller, Graphs, Refusal};

fn refused(why: String) -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised("fleet.graph_refused", why, super::run_id()))
}

impl Graphs for FakeDaemon {
    async fn show_graph(&self, _caller: Option<Caller>, show: ShowGraph) -> Result<GraphShown, Refusal> {
        let checked = show.checked().map_err(refused)?;
        let reading = GraphReading { live: true, shown_at: Some(Instant::carried("2026-11-27T00:00:00.000Z")), ..GraphReading::default() };
        Ok(checked.card(&Default::default(), &reading))
    }

    async fn graph_points(&self, _caller: Option<Caller>, points: GraphPoints) -> Result<GraphPointsTaken, Refusal> {
        points.checked().map_err(refused)?;
        let n = points.rows.len();
        Ok(GraphPointsTaken {
            session_id: points.session_id.unwrap_or_else(|| SessionId::carried("session")),
            feed: points.feed,
            new: n,
            kept: n,
            added: n as u64,
        })
    }

    async fn withdraw_graph(&self, _caller: Option<Caller>, withdraw: WithdrawGraph) -> Result<GraphShown, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.graph_not_shown",
            format!("This Job showed no graph `{}`.", withdraw.id),
            super::run_id(),
        )))
    }

    async fn list_graphs(&self, _caller: Option<Caller>, session_id: Option<SessionId>) -> Result<GraphList, Refusal> {
        Ok(GraphList { session_id: session_id.unwrap_or_else(|| SessionId::carried("session")), graphs: Vec::new(), feeds: Vec::new() })
    }
}
