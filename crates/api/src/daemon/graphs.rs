//! A Job's live graphs: shown, fed and withdrawn by its agent, and listed for Bridge.
//! `docs/practices/protocol/a-job-shows-a-live-graph.md`.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and its recipes run on the
//! watches' loop without being watches.

use std::future::Future;

use ipc::{GraphList, GraphPoints, GraphPointsTaken, GraphShown, SessionId, ShowGraph, WithdrawGraph};

use crate::daemon::Refusal;

pub trait Graphs: Send + Sync + 'static {
    /// `show_graph` — check it and keep it for the Job `caller` places, else the one `show.session_id` names,
    /// replacing one of the same id in place.
    fn show_graph(&self, caller: Option<crate::Caller>, show: ShowGraph) -> impl Future<Output = Result<GraphShown, Refusal>> + Send;

    /// `graph_points` — add the agent's rows to a feed of its Job.
    fn graph_points(&self, caller: Option<crate::Caller>, points: GraphPoints) -> impl Future<Output = Result<GraphPointsTaken, Refusal>> + Send;

    /// `withdraw_graph` — the graph takes no more rows, and its recipe stops. Its card stays.
    fn withdraw_graph(&self, caller: Option<crate::Caller>, withdraw: WithdrawGraph) -> impl Future<Output = Result<GraphShown, Refusal>> + Send;

    /// `list_graphs` — a Job's graphs and every row their feeds keep.
    fn list_graphs(&self, caller: Option<crate::Caller>, session_id: Option<SessionId>) -> impl Future<Output = Result<GraphList, Refusal>> + Send;
}
