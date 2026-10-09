//! Putting a mod on a branch of a repository, without touching the checkout the
//! owner works in.
//!
//! The way a Job gets a worktree: a slot is leased from the repository's pool
//! with a branch cut fresh from the base, the files are written there, those
//! paths are committed, and the slot is given back. The owner's checkout is never
//! the place written, and **nothing is pushed**: the branch is local until a
//! person says otherwise.

use std::path::Path;

use adapter_traits::{
    AgentHarness, CommitTime, Committed, Delivery, SlotLeased, Vcs, WorkProduct, WorktreeSpec,
};
use api::Refusal;
use ipc::{ModPromoted, PromoteMod, WireError};

use super::serving::MOD_NOT_PROMOTABLE;
use super::{examine, Files, Payload, MANIFEST};
use crate::daemon::Fleet;

/// Where a promoted mod lives in the repository.
const PLACE: &str = "packages/mods";

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
    pub(super) async fn mod_promoted(&self, promote: PromoteMod) -> Result<ModPromoted, Refusal> {
        let name = promote.name;
        self.mod_named(&name)?;
        let dir = self.mods_dir();
        let asked = name.clone();
        let found = tokio::task::spawn_blocking(move || examine(&dir, &asked))
            .await
            .map_err(|why| self.mod_fault(format!("the mod could not be read: {why}")))?;
        let Some(files) = found.files else {
            return Err(self.cannot_promote(
                &name,
                format!("`{name}` is not valid: {}", found.problems.join("; ")),
            ));
        };
        let served = self.served_named(promote.manifest_id.as_ref())?;
        if !Path::new(served.root()).join("packages").is_dir() {
            return Err(self.cannot_promote(
                &name,
                String::from("this repository has no packages/ folder to put a mod in"),
            ));
        }
        let millis = self.now().epoch_millis().unwrap_or_default();
        let holder = format!("mod-{name}-{millis}");
        let spec = WorktreeSpec::for_job(served.root(), &holder)
            .map_err(|why| self.mod_fault(format!("no branch could be named: {why:?}")))?;
        let pool = crate::leasing::pool_of(&served);
        let (slot, worktree) = match self.vcs().lease_slot(&pool, &spec, &holder) {
            Ok(SlotLeased::Took { slot, worktree, .. }) => (slot, worktree),
            Ok(SlotLeased::Full) => {
                return Err(self.cannot_promote(
                    &name,
                    String::from("every worktree slot is in use; try again when one is free"),
                ))
            }
            Err(why) => return Err(self.mod_fault(format!("a slot could not be leased: {why}"))),
        };
        let at = CommitTime::seconds_since_epoch(millis.div_euclid(1_000));
        let made = write_and_commit(self, &worktree, &name, &files, at);
        // Given back whether or not the commit was made: it holds nothing but this.
        let parked = self.vcs().park_slot(&pool, slot, &holder);
        let outcome = made.and_then(|committed| match committed {
            Committed::Made { commit } => Ok(commit),
            Committed::NothingToCommit => Err(Refused::Nothing),
        });
        match (outcome, parked) {
            (Ok(commit), Ok(_)) => Ok(ModPromoted {
                name,
                branch: worktree.branch().to_string(),
                commit,
            }),
            (Ok(_), Err(why)) => Err(self.mod_fault(format!(
                "the mod is on `{}` but the slot would not be given back: {}",
                worktree.branch(),
                why.said()
            ))),
            (Err(refused), _) => {
                // The branch was cut for this and holds nothing of the owner's.
                let _ = self.vcs().delete_repair_branch(&spec);
                Err(match refused {
                    Refused::Nothing => self.cannot_promote(
                        &name,
                        String::from("this repository already holds the mod exactly as it is"),
                    ),
                    Refused::Failed(why) => self.mod_fault(why),
                })
            }
        }
    }

    fn cannot_promote(&self, name: &str, said: String) -> Refusal {
        Refusal::IllegalMove(
            WireError::raised(MOD_NOT_PROMOTABLE, said, self.run_id())
                .with_field("name", ipc::WireValue::Str(name.to_string())),
        )
    }
}

enum Refused {
    Nothing,
    Failed(String),
}

/// The two files into `packages/mods/<name>/` in the slot, and those two paths
/// committed. **Written from the bytes that were checked**, not read again.
fn write_and_commit<H, V, W>(
    fleet: &Fleet<H, V, W>,
    worktree: &adapter_traits::Worktree,
    name: &str,
    files: &Files,
    at: CommitTime,
) -> Result<Committed, Refused>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let relative = format!("{PLACE}/{name}");
    let inside = Path::new(worktree.path()).join(&relative);
    std::fs::create_dir_all(&inside)
        .map_err(|why| Refused::Failed(format!("{relative} could not be made: {why}")))?;
    let file = files.payload.file();
    for (file, text) in [(MANIFEST, files.manifest.as_str()), (file, files.payload.text())] {
        std::fs::write(inside.join(file), text)
            .map_err(|why| Refused::Failed(format!("{relative}/{file} could not be written: {why}")))?;
    }
    let paths = [format!("{relative}/{MANIFEST}"), format!("{relative}/{file}")];
    fleet
        .vcs()
        .commit_paths(
            worktree,
            &[paths[0].as_str(), paths[1].as_str()],
            &format!("Add the {name} {} mod", match files.payload {
                Payload::Css(_) => "theme",
                Payload::Layout(_) => "layout",
            }),
            at,
        )
        .map_err(|why| Refused::Failed(format!("the mod could not be committed: {why}")))
}
