//! A pull request's change held up for a walk. `docs/concepts/pull-review.md`, *Walk*.
//!
//! **A deployed place holds nothing.** A pulled-down one leases a bay at the pull request's head,
//! starts the person's command there as a server under the machinery `start_server` uses, and polls
//! the address until it answers. **Whatever happens the bay goes back**: on a close, on a server that
//! ends, on a wait that runs out, and, for a Fleet that died holding one, on the next review walk
//! call ([`Fleet::review_walks_swept`]). The `review_walks` table is what that last road reads.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    Event, ManifestId, OpenReviewWalk, ReviewWalk, ReviewWalkState, ServerCheckout, ServerPhase, StartedBy, WalkHow,
    WalkPlace,
};
use store::HeldReviewWalk;
use tokio::net::TcpStream;

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::review_runs::UNREADABLE;
use crate::servers::{Holder, Launch};
use crate::walk_places::place_problem;

/// The name a walk's server is held under in its bay.
const SERVER: &str = "review-walk";
/// How long the address has to answer once the command starts.
const ANSWERS_WITHIN: Duration = Duration::from_secs(120);
/// How often the address is asked.
const ASKS_EVERY: Duration = Duration::from_millis(250);
/// How long one ask of the address may take.
const ONE_ASK: Duration = Duration::from_secs(1);
/// A walk nobody closed is closed after this long.
const HELD_FOR_MILLIS: i64 = 24 * 60 * 60 * 1000;

/// The key a walk is going under, in the set a search or a run is.
fn going_key(id: &str) -> String {
    format!("walk:{id}")
}

/// What the walk window opens: the web address, else the API base.
fn opens(place: &WalkPlace) -> &str {
    match place.address.is_empty() {
        true => &place.api,
        false => &place.address,
    }
}

/// The host and port an http or https address is served on.
pub(crate) fn endpoint_of(address: &str) -> Option<(String, u16)> {
    let (rest, default) = match (address.strip_prefix("https://"), address.strip_prefix("http://")) {
        (Some(rest), _) => (rest, 443),
        (_, Some(rest)) => (rest, 80),
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let (host, port) = match authority.strip_prefix('[') {
        Some(inside) => {
            let (host, after) = inside.split_once(']')?;
            (host, after.strip_prefix(':'))
        }
        None => match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    let port = match port {
        Some(port) => port.parse().ok()?,
        None => default,
    };
    (!host.is_empty()).then(|| (host.to_string(), port))
}

async fn answers(address: &str) -> bool {
    let Some(endpoint) = endpoint_of(address) else {
        return false;
    };
    matches!(tokio::time::timeout(ONE_ASK, TcpStream::connect(endpoint)).await, Ok(Ok(_)))
}

fn walk_of(
    id: &str,
    manifest_id: &ManifestId,
    number: u64,
    state: ReviewWalkState,
    slot: Option<u32>,
    place: Option<&WalkPlace>,
) -> ReviewWalk {
    ReviewWalk {
        id: id.to_string(),
        manifest_id: manifest_id.clone(),
        number,
        state,
        slot,
        address: place.map(|place| opens(place).to_string()).unwrap_or_default(),
        api: place.map(|place| place.api.clone()).unwrap_or_default(),
        failed: None,
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
    /// Hold up the place for a walk. **Answers at once**: a deployed place `ready`, a pulled-down one
    /// `checking_out`, whose work goes on a task of its own and arrives as `review_walk.changed`.
    pub(crate) async fn review_walk_opened(
        self: Arc<Self>,
        manifest_id: ManifestId,
        open: OpenReviewWalk,
    ) -> Result<ReviewWalk, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        if let Some(problem) = place_problem(&open.place) {
            return Err(self.walk_place_refusal(problem));
        }
        self.review_walks_swept().await;
        let id = self.mint().ulid().as_str().to_lowercase();
        if open.place.how == WalkHow::Deployed {
            return Ok(walk_of(&id, &manifest_id, open.number, ReviewWalkState::Ready, None, Some(&open.place)));
        }
        // The head the bay is cut at is the forge's to say, so a forge that will not is a refusal and
        // not a walk that fails a moment later.
        let number = open.number;
        let page = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_page(root, &number.to_string()))
            .await;
        if !page.and_then(|page| page.head).is_some_and(|head| !head.trim().is_empty()) {
            return Err(Refusal::Fault(self.pr_refusal(
                UNREADABLE,
                format!("the forge would not say what is on pull request {number}"),
            )));
        }
        self.review_going().lock().await.insert(going_key(&id));
        let walk = walk_of(&id, &manifest_id, number, ReviewWalkState::CheckingOut, None, Some(&open.place));
        self.publish(Event::ReviewWalkChanged { walk: walk.clone() });
        let (fleet, started) = (Arc::clone(&self), walk.clone());
        tokio::spawn(async move { fleet.review_walk_driven(started, served, open.place).await });
        Ok(walk)
    }

    /// Close a held walk: stop its server and give its bay back. **Idempotent**: one already closed
    /// or never held is answered `closed`.
    pub(crate) async fn review_walk_closed(&self, manifest_id: ManifestId, id: String) -> Result<ReviewWalk, Refusal> {
        self.served_named(Some(&manifest_id))?;
        self.review_walks_swept().await;
        let going = self.review_going().lock().await.remove(&going_key(&id));
        let held = self.review_walk_given_back(&id).await;
        let number = held.as_ref().map_or(0, |row| row.number);
        let walk = walk_of(&id, &manifest_id, number, ReviewWalkState::Closed, None, None);
        if going || held.is_some() {
            self.publish(Event::ReviewWalkChanged { walk: walk.clone() });
        }
        Ok(walk)
    }

    /// Give back every bay a walk holds that nothing here is serving, and every one held past a day.
    /// **Run at the start of every review walk call**, which is how a bay a dead Fleet held is found.
    pub(crate) async fn review_walks_swept(&self) {
        let held = self.store().lock().await.review_walks_held().unwrap_or_default();
        let now = self.now().epoch_millis();
        for row in held {
            let mine = self.review_going().lock().await.contains(&going_key(&row.id));
            let old = match (now, core_model::Timestamp::from_rfc3339(row.opened_at.clone()).epoch_millis()) {
                (Some(now), Some(opened)) => now - opened > HELD_FOR_MILLIS,
                _ => false,
            };
            if mine && !old {
                continue;
            }
            self.review_going().lock().await.remove(&going_key(&row.id));
            if let Some(given) = self.review_walk_given_back(&row.id).await {
                let manifest_id = ManifestId::carried(given.manifest_id.clone());
                let walk = walk_of(&given.id, &manifest_id, given.number, ReviewWalkState::Closed, None, None);
                self.publish(Event::ReviewWalkChanged { walk });
            }
        }
    }

    /// The server stopped, the bay released and the row deleted, **in that order**, so a Fleet that
    /// dies between them leaves the row for the next sweep. Answers the row only to the one call that
    /// deleted it, so two callers never both announce the close.
    async fn review_walk_given_back(&self, id: &str) -> Option<HeldReviewWalk> {
        // One at a time: a close and the walk's own end arrive together when a close stops the
        // server, and the second waits for the first rather than releasing the bay again.
        let closing = format!("walk-closing:{id}");
        if !self.review_going().lock().await.insert(closing.clone()) {
            while self.review_going().lock().await.contains(&closing) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            return None;
        }
        let given = self.review_walk_released(id).await;
        self.review_going().lock().await.remove(&closing);
        given
    }

    async fn review_walk_released(&self, id: &str) -> Option<HeldReviewWalk> {
        let row = self.store().lock().await.review_walks_held().ok()?.into_iter().find(|row| row.id == id)?;
        crate::servers::stopped_in(self.servers().clone(), &row.path).await;
        if let Some(served) = self.repositories().serving(&row.manifest_id) {
            let (vcs, pool) = (Arc::clone(self.vcs()), crate::leasing::pool_of(&served));
            let (slot, holder) = (row.slot, row.holder.clone());
            // A bay that will not go back is the pool's to report as stranded.
            let _ = tokio::task::spawn_blocking(move || vcs.release_review_bay(&pool, slot, &holder)).await;
        }
        let mut store = self.store().lock().await;
        let still = store.review_walks_held().ok()?.iter().any(|held| held.id == id);
        if still {
            let _ = store.release_review_walk(id);
        }
        still.then_some(row)
    }

    /// A pulled-down walk, from its bay to an address that answers and on to its end.
    async fn review_walk_driven(self: Arc<Self>, mut walk: ReviewWalk, served: Served, place: WalkPlace) {
        let outcome = self.review_walk_brought_up(&mut walk, &served, &place).await;
        let key = going_key(&walk.id);
        match outcome {
            // It came up, and ran until it ended or was closed.
            Ok(()) => {
                self.review_going().lock().await.remove(&key);
                if self.review_walk_given_back(&walk.id).await.is_some() {
                    walk.state = ReviewWalkState::Closed;
                    walk.slot = None;
                    self.publish(Event::ReviewWalkChanged { walk });
                }
            }
            Err(said) => {
                self.review_going().lock().await.remove(&key);
                // **The bay is back before the event says so.**
                let _ = self.review_walk_given_back(&walk.id).await;
                walk.state = ReviewWalkState::Failed;
                walk.slot = None;
                walk.failed = Some(said);
                self.publish(Event::ReviewWalkChanged { walk });
            }
        }
    }

    /// `Ok` once the walk was `ready` and has since ended; `Err` with why it never was. A walk closed
    /// on the way is `Ok` too: whoever closed it has said so.
    async fn review_walk_brought_up(self: &Arc<Self>, walk: &mut ReviewWalk, served: &Served, place: &WalkPlace) -> Result<(), String> {
        let key = going_key(&walk.id);
        let pool = crate::leasing::pool_of(served);
        let holder = format!("review-walk-{}", walk.id);
        let (slot, path, head) = self.review_bay_taken(&pool, walk.number, &holder).await?;
        let held = HeldReviewWalk {
            id: walk.id.clone(),
            manifest_id: walk.manifest_id.as_str().to_string(),
            number: walk.number,
            slot,
            holder: holder.clone(),
            path: path.clone(),
            opened_at: self.now().as_str().to_string(),
        };
        if let Err(why) = self.store().lock().await.keep_review_walk(&held) {
            let (vcs, pool, given) = (Arc::clone(self.vcs()), pool.clone(), (slot, holder));
            let _ = tokio::task::spawn_blocking(move || vcs.release_review_bay(&pool, given.0, &given.1)).await;
            return Err(format!("the bay could not be written down: {why}"));
        }
        // Closed while the bay was being taken: it was never announced, and is given back by the close.
        if !self.review_going().lock().await.contains(&key) {
            let _ = self.review_walk_given_back(&walk.id).await;
            return Ok(());
        }
        walk.slot = Some(slot);
        walk.state = ReviewWalkState::Starting;
        self.publish(Event::ReviewWalkChanged { walk: walk.clone() });

        let real = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
        let launch = Launch {
            holder: Holder::Checkout(real.to_string_lossy().into_owned()),
            name: SERVER.to_string(),
            manifest_id: walk.manifest_id.clone(),
            checkout: ServerCheckout { path: path.clone(), branch: None, commit: Some(head), behind: Some(0) },
            under: format!("review-walks/{}", walk.id),
            ports: Vec::new(),
            links: vec![ipc::ServerLink { url: opens(place).to_string(), name: None }],
            by: StartedBy::Person,
            session: None,
            whose: format!("review walk {}", walk.id),
            run: None,
            serve: place.command.trim().to_string(),
            ready: None,
            worktree: real.clone(),
            env: Vec::new(),
        };
        let (state, _) = Arc::clone(self)
            .launched(served.records_root(), launch)
            .await
            .map_err(|why| format!("the command could not be started: {why}"))?;
        let Some(mut server) = self.servers().running(&Holder::Checkout(real.to_string_lossy().into_owned()), SERVER) else {
            return Err(state.ended.unwrap_or_else(|| "the command ended at once".to_string()));
        };
        let address = opens(place).to_string();
        let waited = async {
            loop {
                if !self.review_going().lock().await.contains(&key) {
                    return Ok(false);
                }
                if answers(&address).await {
                    return Ok(true);
                }
                tokio::select! {
                    ended = server.wait_for(|seen| seen.phase == ServerPhase::Exited) => {
                        let said = ended.ok().and_then(|seen| seen.ended.clone());
                        return Err(format!(
                            "the command ended before {address} answered{}",
                            said.map(|said| format!(": {said}")).unwrap_or_default()
                        ));
                    }
                    () = tokio::time::sleep(ASKS_EVERY) => {}
                }
            }
        };
        match tokio::time::timeout(ANSWERS_WITHIN, waited).await {
            Err(_) => return Err(format!("{address} did not answer within {} seconds", ANSWERS_WITHIN.as_secs())),
            Ok(Err(said)) => return Err(said),
            Ok(Ok(false)) => return Ok(()),
            Ok(Ok(true)) => {}
        }
        walk.state = ReviewWalkState::Ready;
        self.publish(Event::ReviewWalkChanged { walk: walk.clone() });
        // Held until the server ends, which a close or a crash does.
        let _ = server.wait_for(|seen| seen.phase == ServerPhase::Exited).await;
        Ok(())
    }
}
