//! A Job's live graphs: [`api::Graphs`] for Fleet, each graph and its feeds' rows kept in the store, and the recipes
//! run on the watches' loop. `docs/practices/protocol/a-job-shows-a-live-graph.md`.
//!
//! **A recipe is a project metric's call, run with no model** (`adapters::run_graph_recipe`), on a blocking thread as
//! `crate::project_metrics` runs one. The recipes due in one pass run together and each Job is sent one
//! `graphs.changed` once they have all finished, so a 15 second recipe is one event a tick. **A broken read keeps the
//! rows** and records why on the graph. A graph's recipe stops when it is withdrawn or its Job ends.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Graphs, Refusal};
use ipc::graph::Row;
use ipc::{
    GraphChecked, GraphFeed, GraphList, GraphPoints, GraphPointsTaken, GraphReading, GraphRecipe, GraphShown, GraphsChanged, Instant, SessionId,
    ShowGraph, WireError, WithdrawGraph, GRAPHS_A_JOB, GRAPH_FEED_KEPT,
};
use store::{KeptGraph, KeptGraphFeed};

use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// A 422 for a graph, rows or a recipe that breaks a rule. The sentence names each and what is allowed.
pub(crate) const GRAPH_REFUSED: &str = "fleet.graph_refused";
const GRAPH_TOO_MANY: &str = "fleet.graph_too_many";
const GRAPH_NOT_SHOWN: &str = "fleet.graph_not_shown";
const GRAPH_UNPLACED: &str = "fleet.graph_unplaced";

/// The graph recipes running now, by (Job, graph id), so a slow one is not started again by the next pass.
#[derive(Default)]
pub(crate) struct Running(Mutex<HashSet<(String, String)>>);

impl Running {
    fn begin(&self, key: &(String, String)) -> bool {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(key.clone())
    }

    fn finished(&self, key: &(String, String)) {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(key);
    }
}

/// A kept graph read back as Fleet checked it. `None` for one this build no longer reads, which is left out.
fn checked_of(kept: &KeptGraph) -> Option<GraphChecked> {
    ipc::decode::<ShowGraph>("a kept graph", kept.shown.as_bytes()).ok()?.checked().ok()
}

fn rows_of(texts: &[String]) -> Vec<Row> {
    texts.iter().filter_map(|text| ipc::decode::<Row>("a feed's row", text.as_bytes()).ok()).collect()
}

/// Whether a kept graph's recipe is due at `now`: never run, or run its interval ago.
pub(crate) fn due(kept: &KeptGraph, now: &core_model::Timestamp) -> bool {
    let Some(every) = kept.every_seconds else { return false };
    match &kept.tried_at {
        None => true,
        Some(at) => elapsed(&core_model::Timestamp::from_rfc3339(at.clone()), now) >= Duration::from_secs(u64::from(every)),
    }
}

/// One recipe due this pass.
struct Due {
    key: (String, String),
    recipe: GraphRecipe,
    feeds: Vec<String>,
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
    fn graph_refusal(&self, code: &str, why: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why.into(), self.run_id()))
    }

    /// The hosted Job holding the connection, else the one named, which must be one Fleet holds.
    async fn graph_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<String, Refusal> {
        if let Some(held) = caller.and_then(|caller| self.session_holding(&caller)) {
            return Ok(held);
        }
        let Some(named) = named.map(|named| named.as_str().trim().to_string()).filter(|id| !id.is_empty()) else {
            return Err(self.graph_refusal(GRAPH_UNPLACED, "Name a Job: a graph belongs to a Job, and this call gave no Job id."));
        };
        match self.store().lock().await.session(&named).map_err(|why| self.ledger_fault(why))? {
            Some(_) => Ok(named),
            None => Err(self.graph_refusal(NO_SUCH_SESSION, format!("No Job has the id {named}."))),
        }
    }

    /// `job`'s graphs as Bridge draws them, and every row their feeds keep.
    async fn graph_list(&self, job: &str) -> Result<GraphList, Refusal> {
        let (kept, feeds, ended) = {
            let store = self.store().lock().await;
            let kept = store.graphs_of(job).map_err(|why| self.ledger_fault(why))?;
            let feeds = store.graph_feeds(job).map_err(|why| self.ledger_fault(why))?;
            let ended = store.session(job).ok().flatten().is_none_or(|session| session.state == store::SessionState::Ended);
            (kept, feeds, ended)
        };
        let feeds: Vec<GraphFeed> = feeds.into_iter().map(|KeptGraphFeed { name, rows, added }| GraphFeed { name, rows: rows_of(&rows), added }).collect();
        let by_name: BTreeMap<String, Vec<Row>> = feeds.iter().map(|feed| (feed.name.clone(), feed.rows.clone())).collect();
        let graphs = kept
            .iter()
            .filter_map(|one| {
                let reading = GraphReading {
                    live: one.live && !ended,
                    shown_at: Some(Instant::carried(one.shown_at.clone())),
                    read_at: one.read_at.clone().map(Instant::carried),
                    broken: one.broken.clone(),
                };
                Some(checked_of(one)?.card(&by_name, &reading))
            })
            .collect();
        Ok(GraphList { session_id: SessionId::carried(job), graphs, feeds })
    }

    /// Tell Bridge `job`'s graphs moved, with the rows `added` to its feeds since the last event.
    async fn graphs_published(&self, job: &str, added: Vec<GraphFeed>) {
        if let Ok(list) = self.graph_list(job).await {
            self.publish(ipc::Event::GraphsChanged(GraphsChanged { session_id: list.session_id, graphs: list.graphs, feeds: added }));
        }
    }

    /// The card of graph `id` of `job`, as `list_graphs` carries it.
    async fn graph_card(&self, job: &str, id: &str) -> Result<GraphShown, Refusal> {
        self.graph_list(job)
            .await?
            .graphs
            .into_iter()
            .find(|graph| graph.id == id)
            .ok_or_else(|| self.graph_refusal(GRAPH_NOT_SHOWN, format!("This Job holds no graph `{id}`.")))
    }

    /// Add `rows` to `job`'s `feed`: the feed as Bridge is sent it, with only the rows that were new.
    async fn graph_rows_added(&self, job: &str, feed: &str, rows: &[Row]) -> Result<(GraphFeed, usize), Refusal> {
        let texts: Vec<String> = rows.iter().filter_map(|row| ipc::encode(row).ok()).collect();
        let added = self.store().lock().await.add_graph_rows(job, feed, &texts, GRAPH_FEED_KEPT).map_err(|why| self.ledger_fault(why))?;
        Ok((GraphFeed { name: feed.to_string(), rows: rows_of(&added.new), added: added.added }, added.kept))
    }

    /// Run every live recipe that is due, together, and send each Job whose graphs read one `graphs.changed`. A Job
    /// that ended has its graphs ended first, and its recipes run no more.
    pub(crate) async fn graphs_read_when_due(self: &Arc<Self>) {
        let now = self.now();
        let Ok(live) = self.store().lock().await.live_graph_recipes() else { return };
        let mut ended: Vec<String> = Vec::new();
        let mut due_now: Vec<Due> = Vec::new();
        for kept in live {
            if ended.contains(&kept.session_id) {
                continue;
            }
            let gone = {
                let store = self.store().lock().await;
                store.session(&kept.session_id).ok().flatten().is_none_or(|session| session.state == store::SessionState::Ended)
            };
            if gone {
                ended.push(kept.session_id.clone());
                continue;
            }
            if !due(&kept, &now) {
                continue;
            }
            let Some(checked) = checked_of(&kept) else { continue };
            let Some(recipe) = checked.recipe else { continue };
            let key = (kept.session_id.clone(), kept.id.clone());
            if !self.graphs_running().begin(&key) {
                continue;
            }
            due_now.push(Due { key, recipe, feeds: checked.feeds });
        }
        for job in ended {
            let moved = self.store().lock().await.end_graphs_of(&job).unwrap_or(false);
            if moved {
                self.graphs_published(&job, Vec::new()).await;
            }
        }
        if due_now.is_empty() {
            return;
        }
        let runs: Vec<_> = due_now
            .iter()
            .map(|one| {
                let (recipe, feeds) = (one.recipe.clone(), one.feeds.clone());
                tokio::task::spawn_blocking(move || adapters::run_graph_recipe(&recipe, &feeds))
            })
            .collect();
        let mut read: BTreeMap<String, BTreeMap<String, GraphFeed>> = BTreeMap::new();
        for (one, run) in due_now.iter().zip(runs) {
            let (job, id) = &one.key;
            let fetched = run.await.unwrap_or_else(|_| Err(String::from("the read did not finish")));
            let at = self.now();
            let broken = match fetched {
                Ok(by_feed) => {
                    let mut broken = None;
                    for (feed, rows) in by_feed {
                        match self.graph_rows_added(job, &feed, &rows).await {
                            Ok((added, _)) => {
                                let held = read.entry(job.clone()).or_default();
                                match held.get_mut(&feed) {
                                    Some(earlier) => {
                                        earlier.rows.extend(added.rows);
                                        earlier.added = added.added;
                                    }
                                    None => {
                                        held.insert(feed, added);
                                    }
                                }
                            }
                            Err(_) => broken = Some(String::from("Fleet could not keep the rows it read.")),
                        }
                    }
                    broken
                }
                Err(why) => Some(why),
            };
            let _ = self.store().lock().await.graph_tried(job, id, at.as_str(), broken.as_deref());
            read.entry(job.clone()).or_default();
            self.graphs_running().finished(&one.key);
        }
        for (job, feeds) in read {
            let feeds = feeds.into_values().filter(|feed| !feed.rows.is_empty()).collect();
            self.graphs_published(&job, feeds).await;
        }
    }
}

impl<H, V, W> Graphs for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn show_graph(&self, caller: Option<api::Caller>, show: ShowGraph) -> Result<GraphShown, Refusal> {
        let checked = show.checked().map_err(|why| self.graph_refusal(GRAPH_REFUSED, why))?;
        if let Some(problem) = checked.recipe.as_ref().and_then(adapters::graph_recipe_problem) {
            return Err(self.graph_refusal(GRAPH_REFUSED, format!("the graph was not shown: its recipe {problem}")));
        }
        let job = self.graph_job(caller, show.session_id.as_ref()).await?;
        let kept = ShowGraph { id: checked.id.clone(), title: checked.title.clone(), session_id: None, ..show };
        let shown = ipc::encode(&kept).map_err(|why| self.graph_refusal(GRAPH_REFUSED, why.to_string()))?;
        {
            let mut store = self.store().lock().await;
            let held = store.graphs_of(&job).map_err(|why| self.ledger_fault(why))?;
            if held.len() >= GRAPHS_A_JOB && !held.iter().any(|one| one.id == checked.id) {
                return Err(self.graph_refusal(
                    GRAPH_TOO_MANY,
                    format!("A Job holds at most {GRAPHS_A_JOB} graphs. Show one again by its id to replace it, rather than a new id."),
                ));
            }
            let graph = KeptGraph {
                session_id: job.clone(),
                id: checked.id.clone(),
                shown,
                shown_at: self.now().as_str().to_string(),
                every_seconds: checked.recipe.as_ref().map(|recipe| recipe.every_secs),
                live: true,
                tried_at: None,
                read_at: None,
                broken: None,
            };
            store.keep_graph(&graph).map_err(|why| self.ledger_fault(why))?;
        }
        self.graphs_published(&job, Vec::new()).await;
        self.graph_card(&job, &checked.id).await
    }

    async fn graph_points(&self, caller: Option<api::Caller>, points: GraphPoints) -> Result<GraphPointsTaken, Refusal> {
        points.checked().map_err(|why| self.graph_refusal(GRAPH_REFUSED, format!("no rows were added: {why}")))?;
        let job = self.graph_job(caller, points.session_id.as_ref()).await?;
        let (added, kept) = self.graph_rows_added(&job, &points.feed, &points.rows).await?;
        let new = added.rows.len();
        let total = added.added;
        if new > 0 {
            self.graphs_published(&job, vec![added]).await;
        }
        Ok(GraphPointsTaken { session_id: SessionId::carried(&job), feed: points.feed, new, kept, added: total })
    }

    async fn withdraw_graph(&self, caller: Option<api::Caller>, withdraw: WithdrawGraph) -> Result<GraphShown, Refusal> {
        let job = self.graph_job(caller, withdraw.session_id.as_ref()).await?;
        let id = withdraw.id.trim();
        {
            let mut store = self.store().lock().await;
            if store.graph(&job, id).map_err(|why| self.ledger_fault(why))?.is_none() {
                let held: Vec<String> = store.graphs_of(&job).unwrap_or_default().into_iter().map(|one| one.id).collect();
                let held = if held.is_empty() { String::from("none") } else { held.join(", ") };
                return Err(self.graph_refusal(GRAPH_NOT_SHOWN, format!("This Job showed no graph `{id}`. Graphs it holds: {held}.")));
            }
            store.withdraw_graph(&job, id).map_err(|why| self.ledger_fault(why))?;
        }
        self.graphs_published(&job, Vec::new()).await;
        self.graph_card(&job, id).await
    }

    async fn list_graphs(&self, caller: Option<api::Caller>, session_id: Option<SessionId>) -> Result<GraphList, Refusal> {
        let job = self.graph_job(caller, session_id.as_ref()).await?;
        self.graph_list(&job).await
    }
}
