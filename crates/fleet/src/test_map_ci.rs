//! Laps recorded by main's CI run, fetched from the forge and kept as edges of the commit they were
//! read at. `docs/concepts/test-map.md`, *How it stays current*, and `docs/practices/ci.md`, *The
//! test map's laps*.
//!
//! **Fleet never measures here.** A collector that needs a rebuild records only on the forge's runner
//! (`armada test-map record`), so a fresh map is seeded from main's run and never by an instrumented
//! suite on this machine.
//!
//! **Asked when main moves, and again while the run goes on.** The artifact appears minutes after the
//! push, so an ask that finds the run still going is repeated every [`ASK_AGAIN_AFTER`], up to
//! [`ASKS`] times. Only [`Pulled::NotYet`] asks again: a run that ended without the file, a forge that
//! could not be asked and a file that would not read are each left. A repository with no such
//! workflow on its forge is the second, on the first ask, so it costs one.
//!
//! **A newer tip ends the asking.** The laps of a commit that is no longer main's tip would stand in
//! for newer edges of the same tests, so a late artifact for an old tip is dropped unread.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Lap, LapGrain, LapsOnForge, Vcs, WorkProduct};
use core_model::{EdgeGrain, ManifestId};

use crate::daemon::Fleet;

/// How long between asks for a run that is still going.
pub(crate) const ASK_AGAIN_AFTER: Duration = Duration::from_secs(5 * 60);

/// How many times a commit's laps are asked for. Three hours, past the two the job is allowed.
pub(crate) const ASKS: u32 = 36;

/// What one ask came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pulled {
    /// The laps were kept as this many edges.
    Kept { edges: usize },
    /// The map already holds this commit's laps, so the file was not written again.
    AlreadyKept,
    /// The run is not there yet or is still going. The only answer that is asked again.
    NotYet,
    /// The run ended without them.
    Never,
    /// Main has moved on from this commit.
    Superseded,
    /// The forge could not be asked.
    NotAsked(String),
    /// The file is not the laps of this commit.
    Unreadable(String),
    /// The store would not keep them.
    NotKept(String),
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
    /// Main in `repository`, at `root`, moved to `tip`: ask the forge for that commit's laps until
    /// they come, are never coming, or main moves again. Returns at once; the asking is its own task.
    pub(crate) fn test_map_laps_followed(self: &Arc<Self>, repository: ManifestId, root: &str, tip: &str) {
        let (fleet, root, tip) = (Arc::clone(self), root.to_string(), tip.to_string());
        tokio::spawn(async move {
            fleet.test_map_laps_asked_until_settled(&repository, &root, &tip, ASK_AGAIN_AFTER).await;
        });
    }

    /// [`Fleet::test_map_laps_pulled`] until it says anything but not yet, or [`ASKS`] have been made.
    pub(crate) async fn test_map_laps_asked_until_settled(
        &self,
        repository: &ManifestId,
        root: &str,
        tip: &str,
        every: Duration,
    ) -> Pulled {
        let mut pulled = Pulled::NotYet;
        for ask in 0..ASKS {
            if ask > 0 {
                tokio::time::sleep(every).await;
            }
            pulled = self.test_map_laps_pulled(repository, root, tip).await;
            if pulled != Pulled::NotYet {
                break;
            }
        }
        pulled
    }

    /// One ask: the laps of `tip` from the forge, kept as edges at `tip` unless they were kept
    /// already or main is no longer there.
    pub(crate) async fn test_map_laps_pulled(&self, repository: &ManifestId, root: &str, tip: &str) -> Pulled {
        if self.writes().main_tip(repository.as_str()).is_some_and(|now| now != tip) {
            return Pulled::Superseded;
        }
        let commit = tip.to_string();
        let asked = self.forge_asked(root, move |vcs: &V, root: &str| vcs.recorded_laps(root, &commit)).await;
        match asked {
            Err(adapter_traits::NotAsked(said)) => Pulled::NotAsked(said),
            Ok(LapsOnForge::NotYet) => Pulled::NotYet,
            Ok(LapsOnForge::Never) => Pulled::Never,
            Ok(LapsOnForge::Found(bytes)) => self.test_map_laps_kept(repository, tip, &bytes).await,
        }
    }

    async fn test_map_laps_kept(&self, repository: &ManifestId, tip: &str, bytes: &[u8]) -> Pulled {
        let recorded: ipc::RecordedLaps = match ipc::decode("recorded laps", bytes) {
            Ok(recorded) => recorded,
            Err(why) => return Pulled::Unreadable(why.to_string()),
        };
        if recorded.commit != tip {
            return Pulled::Unreadable(format!("the laps were read at {}, not at {tip}", recorded.commit));
        }
        let source = recorded.source.domain();
        match self
            .store()
            .lock()
            .await
            .holds_test_map_edges_at(repository, &recorded.check, source, &recorded.commit)
        {
            Ok(true) => return Pulled::AlreadyKept,
            Ok(false) => {}
            Err(why) => return Pulled::NotKept(why.to_string()),
        }
        let laps: Vec<Lap> = recorded
            .laps
            .into_iter()
            .map(|lap| Lap {
                member: lap.member,
                test: lap.test,
                grain: match lap.grain.domain() {
                    EdgeGrain::Test => LapGrain::Test,
                    EdgeGrain::File => LapGrain::File,
                },
                files: lap.files,
            })
            .collect();
        let edges = self.edges_of(repository, &recorded.check, source, &recorded.commit, &laps);
        let kept = edges.len();
        match self.store().lock().await.record_test_map_edges(&edges) {
            Ok(()) => Pulled::Kept { edges: kept },
            Err(why) => Pulled::NotKept(why.to_string()),
        }
    }
}
