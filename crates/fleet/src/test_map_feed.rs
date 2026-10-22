//! Fleet feeding the test map from work it already does. `docs/concepts/test-map.md`,
//! *How it stays current*.
//!
//! | Moment | What runs |
//! |---|---|
//! | Main moves from one tip to another | The files between them mark their edges stale; each collector that costs nothing and needs no rebuild reads the stale tests again |
//! | Sleep mode is on and a bay is free | Each collector that needs a rebuild measures its stale tests, one at a time |
//!
//! **Nothing that needs a rebuild runs when main moves.** A rebuild costs the machine a turn, and
//! the map's rule is that it is paid on the forge's runner or in a sleeping bay.
//!
//! **A collector that costs nothing reads main's own checkout** when that checkout is at the new
//! tip, which Fleet keeps it at, since reading files changes none and a bay would be a lease
//! taken from a person for a read. Where it is not at the tip, a bay at main's tip is leased the
//! way the sleeping pass leases one, and given back.
//!
//! **Feeders are read off the Manifest each time they are wanted** and not kept, so a Manifest
//! that was reloaded is the one that is read, with no second place that has to hear of the reload.
//! A repository may add one beside them: `Feed::add`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use adapter_traits::{AgentHarness, Delivery, LapCost, SlotLeased, Vcs, WorkProduct, WorktreeSpec};
use adapters::{clash, Feeder};
use checks_runner::CheckWidth;
use core_model::{Component, EdgeSource, Envelope, FieldValue, Level, ManifestId};

use crate::daemon::Fleet;
use crate::places::Asking;
use crate::repositories::Served;
use crate::test_map::{source_of, NotMapped};

/// What Fleet remembers between a tip being seen and the pass that feeds the map from it.
/// Never written down: the moves are re-derived from the forge, and a map a restart did
/// not hear of is stale in the way a stale edge already says.
#[derive(Default)]
pub struct Feed {
    /// Per Manifest id, the earliest tip not yet fed and the latest.
    moved: Mutex<BTreeMap<String, (String, String)>>,
    added: Mutex<Vec<(String, Feeder)>>,
    /// Whether a sleeping pass is under way, so the next tick does not start a second.
    sleeping_bay: AtomicBool,
}

impl Feed {
    /// Main's tip went from `from` to `to`. A second move before the first is fed joins it.
    fn moved(&self, manifest: &str, from: &str, to: &str) {
        let mut moved = self.moved.lock().unwrap_or_else(PoisonError::into_inner);
        let from = moved.get(manifest).map_or(from, |(earlier, _)| earlier.as_str()).to_string();
        moved.insert(manifest.to_string(), (from, to.to_string()));
    }

    pub(crate) fn moves(&self) -> Vec<(String, String, String)> {
        let taken = std::mem::take(&mut *self.moved.lock().unwrap_or_else(PoisonError::into_inner));
        taken.into_iter().map(|(manifest, (from, to))| (manifest, from, to)).collect()
    }

    /// A collector of this repository's own, beside the ones its Manifest's Checks name. Only a
    /// case plants one so far: no Manifest key adds a collector.
    #[cfg(test)]
    pub(crate) fn add(&self, manifest: &ManifestId, feeder: Feeder) {
        self.added
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((manifest.as_str().to_string(), feeder));
    }

    fn added_for(&self, manifest: &str) -> Vec<Feeder> {
        let added = self.added.lock().unwrap_or_else(PoisonError::into_inner);
        added.iter().filter(|(owner, _)| owner == manifest).map(|(_, feeder)| feeder.clone()).collect()
    }
}

/// Clears [`Feed::sleeping_bay`] however the pass ends.
struct Going(Arc<Feed>);

impl Drop for Going {
    fn drop(&mut self) {
        self.0.sleeping_bay.store(false, Ordering::SeqCst);
    }
}

/// A slot leased for a read, and what gives it back.
struct Bay {
    pool: adapter_traits::SlotPool,
    slot: u32,
    holder: String,
    spec: WorktreeSpec,
    path: String,
}

/// One stale test and the source whose edges are stale.
struct Stale {
    check: String,
    source: EdgeSource,
    test: String,
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
    /// Main's tip in `manifest`'s repository is `head`. Beside the clash watch's own reading of
    /// the same tip, so the forge is asked once; where it moved from a tip seen before, the move
    /// waits for [`Fleet::test_map_moves_fed`].
    pub(crate) fn main_tip_seen(&self, manifest: &str, head: &str) {
        let before = self.writes().main_tip(manifest);
        if self.writes().main_seen(manifest, head) {
            if let Some(before) = before {
                self.feed().moved(manifest, &before, head);
            }
        }
    }

    /// Start feeding the map from each move of main seen since the last tick. Each repository
    /// is fed on a task of its own, so the notice loop goes on.
    pub(crate) fn test_map_moves_fed(self: &Arc<Self>) {
        for (manifest, from, to) in self.feed().moves() {
            let fleet = Arc::clone(self);
            tokio::spawn(async move { fleet.main_moved_fed(&manifest, &from, &to).await });
        }
    }

    /// Start a sleeping pass unless one is under way. It runs on a task of its own: a measured
    /// test takes minutes, and the loop it rides on carries Sessions.
    pub(crate) fn test_map_slept_over(self: &Arc<Self>) {
        if self.feed().sleeping_bay.swap(true, Ordering::SeqCst) {
            return;
        }
        let (fleet, going) = (Arc::clone(self), Going(Arc::clone(self.feed())));
        tokio::spawn(async move {
            fleet.sleeping_bay_recorded().await;
            drop(going);
        });
    }

    /// Main went from `from` to `to`: the files between them make their edges stale, and each
    /// stale test a collector can read at no cost is read again at `to`.
    pub(crate) async fn main_moved_fed(&self, manifest: &str, from: &str, to: &str) {
        let Some(served) = self.repositories().serving(manifest) else { return };
        let repository = served.manifest().id().clone();
        let (root, held_from, held_to) = (served.root().to_string(), from.to_string(), to.to_string());
        let changed = tokio::task::spawn_blocking(move || clash::changed_between(Path::new(&root), &held_from, &held_to))
            .await
            .map_err(|why| why.to_string())
            .and_then(|read| read.map_err(|why| why.to_string()));
        let changed = match changed {
            Ok(changed) => changed,
            Err(why) => {
                self.said(Level::Warn, "test map not marked stale", &repository, [("said", FieldValue::Str(why))]);
                return;
            }
        };
        if let Err(why) = self.store().lock().await.mark_test_map_stale(&repository, &changed) {
            self.said(Level::Warn, "test map not marked stale", &repository, [("said", FieldValue::Str(why.to_string()))]);
            return;
        }
        self.cheap_stale_recorded(&served, to).await;
    }

    async fn cheap_stale_recorded(&self, served: &Served, tip: &str) {
        let repository = served.manifest().id().clone();
        let main = Path::new(served.root());
        let readable = |feeders: &[Feeder]| -> Vec<Feeder> {
            feeders.iter().filter(|feeder| !rebuilds(feeder) && costs_nothing(feeder)).cloned().collect()
        };
        let stale = self.stale_of(&repository).await;
        let cheap = readable(&self.feeders_in(served, main, false));
        if !cheap.iter().any(|feeder| !tests_of(&stale, feeder).is_empty()) {
            return;
        }
        let at_the_tip = {
            let held = main.to_path_buf();
            tokio::task::spawn_blocking(move || adapters::territory::head_of(&held, "HEAD").ok()).await.ok().flatten()
        };
        let (bay, checkout) = match at_the_tip.as_deref() == Some(tip) {
            true => (None, served.root().to_string()),
            false => match self.bay_leased(served).await {
                Some(bay) => {
                    let path = bay.path.clone();
                    (Some(bay), path)
                }
                None => return,
            },
        };
        let cheap = match bay {
            Some(_) => readable(&self.feeders_in(served, Path::new(&checkout), false)),
            None => cheap,
        };
        for feeder in &cheap {
            let tests = tests_of(&stale, feeder);
            if !tests.is_empty() {
                self.recorded_in(&repository, feeder, &checkout, tests).await;
            }
        }
        if let Some(bay) = bay {
            self.bay_given_back(bay).await;
        }
    }

    /// While sleep is on: each stale test of a collector that needs a rebuild is measured in
    /// a free bay at main's tip, one at a time, holding one of the machine's places and stopping
    /// as soon as sleep goes off. **A repository whose map holds no edges has no stale test**, so
    /// it is never seeded from here; that is the forge's recording.
    pub(crate) async fn sleeping_bay_recorded(&self) {
        for served in self.repositories().served() {
            if !self.sleeping().await {
                return;
            }
            self.rebuilding_recorded_in(&served).await;
        }
    }

    async fn rebuilding_recorded_in(&self, served: &Served) {
        let repository = served.manifest().id().clone();
        let stale = self.stale_of(&repository).await;
        let in_main = self.feeders_in(served, Path::new(served.root()), true);
        if !in_main.iter().any(|feeder| rebuilds(feeder) && !tests_of(&stale, feeder).is_empty()) {
            return;
        }
        // No free bay is no pass: the lease never waits.
        let Some(bay) = self.bay_leased(served).await else { return };
        let feeders = self.feeders_in(served, Path::new(&bay.path), true);
        let place = self.room_asking(Asking::Recording).place().await;
        'feeders: for feeder in feeders.iter().filter(|feeder| rebuilds(feeder)) {
            for test in tests_of(&stale, feeder) {
                if !self.sleeping().await {
                    break 'feeders;
                }
                self.recorded_in(&repository, feeder, &bay.path, vec![test]).await;
            }
        }
        drop(place);
        self.bay_given_back(bay).await;
    }

    async fn stale_of(&self, repository: &ManifestId) -> Vec<Stale> {
        let read = self.store().lock().await.stale_tests(repository);
        read.unwrap_or_default()
            .into_iter()
            .map(|one| Stale { check: one.check, source: one.source, test: one.test })
            .collect()
    }

    /// The collectors the Checks of the root Manifest and every workspace's name, built to record
    /// in the checkout at `root`, and the ones the repository added. **A Check's key is its
    /// name at the root and `<dir>:<name>` in a workspace**, as a Job's rows spell it. Coverage
    /// is built one wide for a sleeping bay and as wide as the Check runs elsewhere.
    pub(crate) fn feeders_in(&self, served: &Served, root: &Path, sleeping: bool) -> Vec<Feeder> {
        let workspaces = served.workspaces();
        let mut feeders = Vec::new();
        for manifest in std::iter::once(served.manifest()).chain(workspaces.iter()) {
            for name in manifest.check_names() {
                let Some(check) = manifest.check(&name) else { continue };
                let width = match sleeping {
                    true => CheckWidth::of(1),
                    false => self.places_width().narrowed_to(check.width()),
                };
                let key = match manifest.dir() {
                    "" => name.clone(),
                    dir => format!("{dir}:{name}"),
                };
                feeders.extend(adapters::collector_for(
                    &key,
                    check.run(),
                    check.runner(),
                    check.narrow(),
                    manifest.dir(),
                    root,
                    width,
                ));
            }
        }
        feeders.extend(self.feed().added_for(served.manifest().id().as_str()));
        feeders
    }

    /// Record `tests` of `feeder`'s Check in `checkout`, keep the edges, and say so. **Stamped
    /// with the commit `checkout` is at**, whatever the caller meant it to be at.
    async fn recorded_in(&self, repository: &ManifestId, feeder: &Feeder, checkout: &str, tests: Vec<String>) {
        let started = Instant::now();
        let named = tests.clone();
        let kept = self.edges_kept_in(repository, feeder, checkout, tests).await;
        let mut fields = vec![
            ("collector", FieldValue::Str(feeder.collector().name().to_string())),
            ("check", FieldValue::Str(feeder.check().to_string())),
            ("tests", FieldValue::List(named.into_iter().map(FieldValue::Str).collect())),
            ("took_ms", FieldValue::Int(started.elapsed().as_millis() as i64)),
        ];
        match kept {
            Ok(kept) => {
                fields.push(("edges", FieldValue::Int(kept as i64)));
                self.said(Level::Info, "test map recorded", repository, fields);
            }
            Err(why) => {
                fields.push(("said", FieldValue::Str(why.to_string())));
                self.said(Level::Warn, "test map not recorded", repository, fields);
            }
        }
    }

    async fn edges_kept_in(
        &self,
        repository: &ManifestId,
        feeder: &Feeder,
        checkout: &str,
        tests: Vec<String>,
    ) -> Result<usize, NotMapped> {
        let at = checkout.to_string();
        let commit = tokio::task::spawn_blocking(move || adapters::territory::head_of(Path::new(&at), "HEAD"))
            .await
            .map_err(|why| NotMapped::NoCommit { dir: checkout.to_string(), said: why.to_string() })?
            .map_err(|why| NotMapped::NoCommit { dir: checkout.to_string(), said: why.to_string() })?;
        let (collector, check, dir) = (feeder.collector().clone(), feeder.check().to_string(), feeder.dir_in(checkout));
        let (named, asked) = (collector.clone(), tests.clone());
        let laps = tokio::task::spawn_blocking(move || {
            named.record(&check, &dir, &adapter_traits::TestsToRecord::Named(asked))
        })
        .await
        .map_err(|why| NotMapped::Collector { name: collector.name().to_string(), said: why.to_string() })?
        .map_err(|why| NotMapped::Collector { name: collector.name().to_string(), said: why.0 })?;
        let edges = self.edges_of(repository, feeder.check(), source_of(collector.source()), &commit, &laps);
        let kept = edges.len();
        // **A test asked for by name that comes back with no lap is gone**, and so are its edges
        // from this source: nobody can read them again, and a stale edge for it would be asked
        // for on every later pass. Another source's edges for the name are its own to settle.
        let gone: Vec<String> = tests.into_iter().filter(|test| laps.iter().all(|lap| &lap.test != test)).collect();
        let mut store = self.store().lock().await;
        store.record_test_map_edges(&edges).map_err(|why| NotMapped::NotKept(why.to_string()))?;
        store
            .forget_test_map_tests(repository, feeder.check(), source_of(collector.source()), &gone)
            .map_err(|why| NotMapped::NotKept(why.to_string()))?;
        Ok(kept)
    }

    /// A free bay now, cut at the base's tip, or nothing. **Never waits**, and says nothing when
    /// the pool is full: a full pool is the pool being used.
    async fn bay_leased(&self, served: &Served) -> Option<Bay> {
        let millis = self.now().epoch_millis().unwrap_or_default();
        let holder = format!("test-map-{millis}");
        let spec = WorktreeSpec::for_job(served.root(), &holder).ok()?;
        let pool = crate::leasing::pool_of(served);
        let (vcs, taken_pool, taken_spec, taken_holder) =
            (Arc::clone(self.vcs()), pool.clone(), spec.clone(), holder.clone());
        let leased = tokio::task::spawn_blocking(move || vcs.lease_slot(&taken_pool, &taken_spec, &taken_holder))
            .await
            .ok()?;
        match leased {
            Ok(SlotLeased::Took { slot, worktree, .. }) => {
                Some(Bay { pool, slot, holder, spec, path: worktree.path().to_string() })
            }
            Ok(SlotLeased::Full) => None,
            Err(why) => {
                self.said(
                    Level::Warn,
                    "test map bay not leased",
                    served.manifest().id(),
                    [("said", FieldValue::Str(why.to_string()))],
                );
                None
            }
        }
    }

    /// The bay back in the pool and the branch it was cut on gone. **Parked rather than
    /// released**, since a build may leave files a release would refuse; the branch is deleted
    /// after, so nothing of the recording is kept.
    async fn bay_given_back(&self, bay: Bay) {
        let (vcs, pool, slot, holder) = (Arc::clone(self.vcs()), bay.pool.clone(), bay.slot, bay.holder.clone());
        let parked = tokio::task::spawn_blocking(move || vcs.park_slot(&pool, slot, &holder)).await;
        if !matches!(parked, Ok(Ok(_))) {
            return;
        }
        let (vcs, spec) = (Arc::clone(self.vcs()), bay.spec.clone());
        let _ = tokio::task::spawn_blocking(move || vcs.delete_repair_branch(&spec)).await;
    }

    /// A line in the envelope on Fleet's standard error: the repository and what the call adds.
    fn said(
        &self,
        level: Level,
        msg: &str,
        repository: &ManifestId,
        fields: impl IntoIterator<Item = (&'static str, FieldValue)>,
    ) {
        crate::transcript::note_aloud(&self.feed_envelope(level, msg, repository, fields));
    }

    pub(crate) fn feed_envelope(
        &self,
        level: Level,
        msg: &str,
        repository: &ManifestId,
        fields: impl IntoIterator<Item = (&'static str, FieldValue)>,
    ) -> Envelope {
        let mut envelope = Envelope::new(self.now(), level, Component::Fleet, self.run().clone(), msg)
            .with_field("repository", FieldValue::Str(repository.as_str().to_string()));
        for (key, value) in fields {
            envelope = envelope.with_field(key, value);
        }
        envelope
    }
}

fn rebuilds(feeder: &Feeder) -> bool {
    feeder.collector().needs_rebuild()
}

fn costs_nothing(feeder: &Feeder) -> bool {
    feeder.collector().lap_cost() == LapCost::Nothing
}

/// The stale tests this feeder's collector last read, by the Check it records.
fn tests_of(stale: &[Stale], feeder: &Feeder) -> Vec<String> {
    let source = source_of(feeder.collector().source());
    stale
        .iter()
        .filter(|one| one.check == feeder.check() && one.source == source)
        .map(|one| one.test.clone())
        .collect()
}
