//! Reading a Link in: the source fetched by Fleet, and what comes back hung
//! off the Link. `#1293`, `docs/concepts/studio.md`, *Promotion*.
//!
//! **Fleet fetches and the scout is handed text.** A scout's launch denies
//! every tool that could reach a source and carries no MCP server — the
//! confinement spike 017 measured — so the fetch happens here, in Fleet's own
//! unsandboxed process, as it already does for a bare issue link in a request.
//!
//! **The fetch is before the write.** A source that will not fetch is a
//! refusal with no node behind it, rather than a Finding that failed at once.
//!
//! **A milestone takes no scout.** It is a list, and a model asked to echo one
//! back is cost spent on a transcription; Fleet mints a Link per issue itself.

mod epic;
mod zone;

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, LookupCall, Vcs, WorkProduct};
use adapters::{Fetch, MilestoneRead, Source};
use api::{Redirector, Refusal};
use core_model::{
    EndedFinding, EpicTake, ScoutSource, ScoutSourceKind, StudioAuthor, StudioEdge, StudioEdgeId,
    StudioFinding, StudioId, StudioNode, StudioNodeContent, StudioNodeId, StudioPosition,
    StudioRelation,
};
use ipc::{HelmStudioAct, ManifestId, ReadInLink};
use tokio::process::Command;
use tokio::time::timeout;

use crate::daemon::Fleet;
use crate::framing::{HEAD, INSET};
use crate::promoting::NOT_A_LINK;
use crate::studios::author;

/// A Link to a board, a wiki or anything else no scout reads. A 422.
const STAYS_A_LINK: &str = "fleet.studio_link_stays_a_link";
/// A source Fleet could not fetch. A 422, and no node is written.
const UNREADABLE: &str = "fleet.studio_source_unreadable";

/// The most of a source's text one scout is handed.
///
/// **A bound because a source is somebody else's**: a 114-turn session and a
/// documentation page are both unbounded, and what did not fit is recorded on
/// the Finding rather than dropped quietly.
const MOST_CHARACTERS: usize = 120_000;

/// How far apart nodes a read-in made are placed, in canvas units.
pub(crate) const ACROSS: i64 = 340;
pub(crate) const DOWN: i64 = 180;
/// How many nodes a read-in puts in one column before starting another.
pub(crate) const DOWN_A_COLUMN: i64 = 6;

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
    /// `read_in_link`. The node survives and keeps its address; what comes
    /// back hangs off it by `produced` edges.
    pub(crate) async fn link_read_in(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        read_in: ReadInLink,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let link = read_in.node_id.to_domain();
        let source_node = self.source_node(&id, &link, within.as_ref()).await?;
        let address = String::from(
            source_node
                .content()
                .address()
                .expect("a node `source_node` answered keeps an address"),
        );
        let (root, home) = self.checkout_and_home(&id, within.as_ref()).await?;
        let source = adapters::source_of(&address, &root, &home).ok_or_else(|| {
            self.studio_unacceptable(
                STAYS_A_LINK,
                adapters::StaysALink {
                    address: address.clone(),
                }
                .to_string(),
            )
        })?;
        let printed = self.fetched(&source, &id, within.as_ref()).await?;
        match &source {
            Source::Milestone { number, .. } => {
                let read = adapters::milestone_read(&printed, number);
                // **Every issue where nothing asked** — an older Bridge, and
                // what reading one in used to do. `#1405`.
                let take = read_in
                    .take
                    .map_or(EpicTake::Everything, ipc::EpicTake::to_domain);
                self.milestone_read_in(
                    &studio_id,
                    &source_node,
                    read,
                    take,
                    read_in.position,
                    by,
                    within,
                )
                .await
            }
            _ => {
                self.scout_reading_in(
                    &studio_id,
                    &source_node,
                    &address,
                    source,
                    printed,
                    root,
                    read_in.position,
                    by,
                    within,
                )
                .await
            }
        }
    }

    /// An Epic: one Issue node per issue the answer takes, straight from what
    /// the forge printed. **No scout and no Finding** — nothing was learned, a
    /// list was copied, and the node that would say what it cost would be
    /// saying nought.
    ///
    /// **Every field on every Issue node is filled.** A milestone's own read
    /// already answers each issue's number, title and state, so nothing here
    /// is left for a later fetch — `#1394`.
    ///
    /// **Reading it in again widens or narrows what is on the board**, against
    /// the other answer — `#1405`, and [`epic::what_it_took`] decides it.
    #[allow(clippy::too_many_arguments)]
    async fn milestone_read_in(
        &self,
        studio_id: &ipc::StudioId,
        source: &StudioNode,
        read: MilestoneRead,
        take: EpicTake,
        at_position: ipc::StudioPosition,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let author = author(by);
        let link = source.id().clone();
        // **Its issues land in one Zone**, made on its first read and filled
        // on every one after — the owner, 2 Oct 2026. An Epic whose block was
        // laid out on the board before Zones keeps it there.
        let (took, zone, new_zone) = {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?;
            let held = epic::zone_of(&graph, source.id()).map(|zone| zone.id().clone());
            let new_zone = (held.is_none() && !epic::laid_out_before(&graph, source)).then(|| {
                StudioNode::added(
                    StudioNodeId::carried(self.mint().ulid()),
                    StudioNodeContent::Zone,
                    at_position.to_domain(),
                    at.clone(),
                    author,
                )
            });
            let zone = held.or_else(|| new_zone.as_ref().map(|zone| zone.id().clone()));
            let from = match &new_zone {
                Some(_) => StudioPosition { x: INSET, y: HEAD },
                None => at_position.to_domain(),
            };
            let took = epic::what_it_took(&graph, source, take, &read, from);
            (took, zone, new_zone)
        };
        let made: Vec<(StudioNode, StudioEdgeId)> = new_zone
            .into_iter()
            .map(|zone| (zone, StudioEdgeId::carried(self.mint().ulid())))
            .chain(took.made.into_iter().map(|(content, at_cell)| {
                (
                    StudioNode::added(
                        StudioNodeId::carried(self.mint().ulid()),
                        content,
                        at_cell,
                        at.clone(),
                        author,
                    )
                    .placed(zone.clone(), at_cell),
                    StudioEdgeId::carried(self.mint().ulid()),
                )
            }))
            .collect();
        // **What the Epic itself now says**: the node survives with its
        // address and gains its title, which of its issues the read took, how
        // many of how many are on the board, how many the answer left out and
        // how many it left standing because somebody had worked on them.
        let resolved = source.resolved(&read.facts(took.read_in)).ok();
        let node_ids: Vec<ipc::StudioNodeId> = made
            .iter()
            .map(|(node, _)| ipc::StudioNodeId::from(node.id()))
            .collect();
        let taken_back = took.taken_back;
        let studio = self
            .written(studio_id, within, |store, id| {
                store.keep_read_in(id, &link, resolved.as_ref(), &made, &[], &taken_back, &at)
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::ReadIn {
                from: ipc::StudioNodeId::from(&link),
                node_ids,
            },
        );
        Ok(studio)
    }

    /// Every other source: a Finding Gathering, produced by the node it was
    /// read in on, and a scout told the text on its one turn.
    ///
    /// **An Issue or a Pull request also gains what the forge said about it**
    /// — its title and where it stands, off the first line of what was
    /// fetched, in the same write. `#1394`.
    #[allow(clippy::too_many_arguments)]
    async fn scout_reading_in(
        self: &Arc<Self>,
        studio_id: &ipc::StudioId,
        source_node: &StudioNode,
        address: &str,
        source: Source,
        printed: String,
        root: String,
        at_position: ipc::StudioPosition,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let link = source_node.id().clone();
        // Only an issue's and a pull request's fetch prints the line this
        // reads; a page, a session and a thread have no forge fields at all.
        let resolved = match &source {
            Source::Issue { .. } | Source::PullRequest { .. } => {
                source_node.resolved(&adapters::forge_facts(&printed)).ok()
            }
            _ => None,
        };
        let text = match &source {
            Source::Page { .. } => adapters::text_of_a_page(&printed),
            Source::Session { .. } => adapters::text_of_a_session(&printed),
            _ => printed,
        };
        let (text, cut) = adapters::bounded(&text, MOST_CHARACTERS);
        if text.trim().is_empty() {
            return Err(self.studio_unacceptable(
                UNREADABLE,
                format!("`{address}` came back with nothing to read"),
            ));
        }
        let checkout = self.checkout_read(&root).await?;
        let at = self.now();
        // **Everything the read-in brings back lands in one Zone**, its Finding
        // first — the owner, 2 Oct 2026. The Zone goes where the person is
        // looking, and what the scout answers is laid out in it later.
        let zone = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Zone,
            at_position.to_domain(),
            at.clone(),
            author(by),
        );
        let first = StudioPosition { x: INSET, y: HEAD };
        let proposed = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Finding(StudioFinding::asked(&format!("Read in {address}"))),
            first,
            at.clone(),
            author(by),
        )
        .placed(Some(zone.id().clone()), first);
        let gathering = proposed
            .reading_in(
                checkout,
                vec![ScoutSource {
                    address: address.to_string(),
                    kind: kind_of(&source),
                    cut,
                }],
            )
            .expect("a node just added is a Proposed Finding");
        let edge = StudioEdgeId::carried(self.mint().ulid());
        let zone_edge = StudioEdgeId::carried(self.mint().ulid());
        let node_ids = vec![
            ipc::StudioNodeId::from(zone.id()),
            ipc::StudioNodeId::from(gathering.node().id()),
        ];
        let made = [(zone, zone_edge), (gathering.node().clone(), edge)];
        let studio = self
            .written(studio_id, within.clone(), |store, id| {
                store.keep_read_in(id, &link, resolved.as_ref(), &made, &[], &[], &at)
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::ReadIn {
                from: ipc::StudioNodeId::from(&link),
                node_ids,
            },
        );
        let told = crate::scout::told_a_read_in(&root, &source.told(), &text);
        Arc::clone(self)
            .scouting(id, gathering, root, told, Some(link))
            .await;
        self.studio_now(studio_id, within).await
    }

    /// What a scout asked the Studio for, read off its answer and written.
    ///
    /// **Nothing here refuses.** The read-in is over by the time this runs and
    /// its Finding is already kept, so an answer that is not the shape leaves
    /// the Finding saying what the scout said and no node beside it.
    pub(crate) async fn what_came_back(
        &self,
        studio: &StudioId,
        link: &StudioNodeId,
        ended: &EndedFinding,
    ) {
        let StudioNodeContent::Finding(finding) = ended.node().content() else {
            return;
        };
        let Some(learned) = finding.learned() else {
            return;
        };
        let Ok(read) = ipc::what_a_scout_read_in(learned) else {
            return;
        };
        let read = zone::without_thin_clusters(read);
        if read.empty() {
            return;
        }
        let at = self.now();
        // In the Finding's own Zone, beside it, where the read-in put one.
        let zone = ended.node().within().cloned();
        let laid = zone::laid_out_beside(ended.node().position(), &read);
        let mut made: Vec<(StudioNode, StudioEdgeId)> = Vec::new();
        let mut by_handle: Vec<(String, StudioNodeId)> = Vec::new();
        let mut mint = |content: StudioNodeContent,
                        handle: Option<&str>,
                        within: Option<StudioNodeId>,
                        to: StudioPosition| {
            let node = StudioNode::added(
                StudioNodeId::carried(self.mint().ulid()),
                content,
                to,
                at.clone(),
                // A scout runs on a person's ask, and a Studio records a
                // person or Helm: whoever asked owns what came back.
                ended.node().added_by().unwrap_or(StudioAuthor::Person),
            )
            .placed(within, to);
            let id = node.id().clone();
            if let Some(handle) = handle {
                by_handle.push((handle.to_string(), id.clone()));
            }
            made.push((node, StudioEdgeId::carried(self.mint().ulid())));
            id
        };
        // **Each frame before what it holds**, which the store's key needs.
        let mut clusters: Vec<(StudioNodeId, &[String])> = Vec::new();
        for (cluster, corner) in read.clusters.iter().zip(&laid.clusters) {
            let id = mint(
                StudioNodeContent::Cluster {
                    title: cluster.title.clone(),
                },
                None,
                zone.clone(),
                *corner,
            );
            clusters.push((id, &cluster.of));
        }
        let mut drawn_in: Vec<(StudioNodeId, StudioNodeId)> = Vec::new();
        for (note, (cluster, to)) in read.notes.iter().zip(&laid.notes) {
            let frame = cluster.map(|at| clusters[at].0.clone());
            let id = mint(
                StudioNodeContent::Note {
                    said: note.said.clone(),
                    capture: None,
                },
                Some(&note.id),
                frame.clone().or_else(|| zone.clone()),
                *to,
            );
            if let Some(frame) = frame {
                drawn_in.push((id, frame));
            }
        }
        for (one, to) in read.contradictions.iter().zip(&laid.contradictions) {
            mint(
                StudioNodeContent::Contradiction {
                    first: one.first.clone(),
                    second: one.second.clone(),
                    answer: None,
                },
                Some(&one.id),
                zone.clone(),
                *to,
            );
        }
        let named = |handle: &str| -> Option<StudioNodeId> {
            by_handle
                .iter()
                .find(|(given, _)| given == handle)
                .map(|(_, id)| id.clone())
        };
        let author = ended.node().added_by().unwrap_or(StudioAuthor::Person);
        let mut edges: Vec<StudioEdge> = Vec::new();
        // A Cluster is its Notes, by a `produced` edge from each, the way
        // grouping makes one. A handle naming no Note in this answer is
        // skipped, as an unknown one in a relation is, and so is a Note
        // already drawn in an earlier Cluster: a Note is in one at a time.
        for (cluster, of) in &clusters {
            let mut joined: Vec<StudioNodeId> = Vec::new();
            for handle in *of {
                let Some(note) =
                    named(handle).filter(|_| read.notes.iter().any(|note| &note.id == handle))
                else {
                    continue;
                };
                let elsewhere = drawn_in
                    .iter()
                    .any(|(drawn, frame)| drawn == &note && frame != cluster);
                if joined.contains(&note) || elsewhere {
                    continue;
                }
                joined.push(note.clone());
                if let Ok(edge) = StudioEdge::produced(
                    StudioEdgeId::carried(self.mint().ulid()),
                    note,
                    cluster.clone(),
                    at.clone(),
                    author,
                ) {
                    edges.push(edge);
                }
            }
        }
        for relation in &read.relations {
            let (Some(from), Some(to)) = (named(&relation.from), named(&relation.to)) else {
                continue;
            };
            let Some(kind) = StudioRelation::from_wire(&relation.relation) else {
                continue;
            };
            // Proposed, whoever asked for it: only a person accepts a relation.
            if let Ok(edge) = StudioEdge::proposed(
                StudioEdgeId::carried(self.mint().ulid()),
                from,
                to,
                kind,
                at.clone(),
                author,
            ) {
                edges.push(edge);
            }
        }
        let kept =
            self.store()
                .lock()
                .await
                .keep_read_in(studio, link, None, &made, &edges, &[], &at);
        let _ = kept;
    }

    /// The node a read-in was asked on, refused where it keeps no address.
    ///
    /// **Four kinds keep one** — a Link, an Issue, a Pull request and an Epic
    /// (`#1394`) — so reading in is every address node's rung rather than a
    /// Link's alone. The Link a person pasted and the Issue it turned out to
    /// be are the same node, and a rung that reached only one of them would
    /// depend on whether an adapter recognised the address.
    async fn source_node(
        &self,
        studio: &StudioId,
        node_id: &StudioNodeId,
        within: Option<&ManifestId>,
    ) -> Result<StudioNode, Refusal> {
        let store = self.store().lock().await;
        let graph = self.studio_held(&store, studio, within)?;
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id() == node_id)
            .ok_or_else(|| self.no_such_node(node_id))?;
        match node.content().address() {
            Some(_) => Ok(node.clone()),
            None => Err(self.studio_unacceptable(
                NOT_A_LINK,
                format!(
                    "`{}` is a {} and keeps no address, so there is nothing to read in",
                    node_id.as_str(),
                    node.kind().as_wire()
                ),
            )),
        }
    }

    /// The repository's checkout, and the home the agent CLI keeps its
    /// sessions under.
    async fn checkout_and_home(
        &self,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<(String, String), Refusal> {
        let root = self.checkout_of(studio, within).await?;
        Ok((root, self.host().home.clone()))
    }

    /// Run the fetch and answer with what it printed.
    async fn fetched(
        &self,
        source: &Source,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<String, Refusal> {
        let unreadable = |why: String| self.studio_unacceptable(UNREADABLE, why);
        match adapters::fetching(source) {
            Fetch::Calls(calls) => {
                let mut printed = Vec::new();
                for call in &calls {
                    printed.push(ran(call, &self.host().path).await.map_err(&unreadable)?);
                }
                Ok(printed.join("\n"))
            }
            Fetch::File(file) => tokio::fs::read_to_string(&file)
                .await
                .map_err(|why| unreadable(format!("{} would not open: {why}", file.display()))),
            Fetch::Thread => self.helm_thread(studio, within).await,
        }
    }

    /// This repository's Helm thread, as what was said in it.
    ///
    /// **Fleet's own read and not a tool.** `observe_helm` refuses every agent
    /// — `crates/ipc/operations.toml` — so a scout reaching a thread through a
    /// tool would be that door opened; the file is read here instead and its
    /// prose handed over.
    async fn helm_thread(
        &self,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<String, Refusal> {
        let manifest = {
            let store = self.store().lock().await;
            self.studio_held(&store, studio, within)?
                .studio
                .manifest_id
                .clone()
        };
        let served = self.served_named(Some(&ManifestId::from(&manifest)))?;
        let file =
            crate::helm::ConversationKey::of_repository(&manifest).thread_in(served.records_root());
        let read = tokio::fs::read_to_string(&file).await.map_err(|why| {
            self.studio_unacceptable(
                UNREADABLE,
                format!("this repository's Helm thread would not open: {why}"),
            )
        })?;
        Ok(crate::helm::what_was_said(&read))
    }
}

/// One rendered fetch, run in Fleet's own process. `crate::proposal`'s
/// `resolved`, with this read-in's own budget.
///
/// **`path` is the one Fleet gives every process it spawns**, so a fetch finds
/// the same programs a Drone would and a Fleet configured with a narrow PATH
/// does not quietly reach a wider one.
async fn ran(call: &LookupCall, path: &str) -> Result<String, String> {
    let mut spawning = Command::new(call.program());
    spawning
        .args(call.args())
        .env("PATH", path)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = timeout(
        Duration::from_secs(adapters::FETCH_SECONDS + 5),
        spawning.output(),
    )
    .await
    .map_err(|_| format!("`{}` took too long", call.program()))?
    .map_err(|error| format!("`{}` would not run: {error}", call.program()))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("`{}` said: {said}", call.program()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The `n`th node of a read-in, down a column and then across.
pub(crate) fn laid_out(from: StudioPosition, n: i64) -> StudioPosition {
    StudioPosition {
        x: from.x + (n / DOWN_A_COLUMN) * ACROSS,
        y: from.y + (n % DOWN_A_COLUMN) * DOWN,
    }
}

fn kind_of(source: &Source) -> ScoutSourceKind {
    ScoutSourceKind::from_wire(source.kind()).expect("adapters spells the domain's own set")
}
