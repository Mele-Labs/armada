//! `read_clash_file`: one file of a standing clash, as each side changed it. `docs/concepts/fleet.md`,
//! *Write clashes*.
//!
//! **Asked of git when a person opens the file, never kept**: a patch is too large for a table that
//! lives in memory. The worktrees are read as they stand through scratch indexes (`adapters::clash`),
//! so a Session's own index, branch and files are untouched. The clash is looked up first, so a
//! file is only ever read for a pair Fleet itself found, and the path is one of the pair's.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::clash::{self, FileDiffs, NotChecked, Verdict};
use api::Refusal;
use ipc::{ClashFile, ClashSide, WireError};

use super::checking::{base_of, changed_since_base};
use super::{Opposed, Side, Who};
use crate::daemon::Fleet;

/// The pair is not clashing now, or Fleet no longer knows where one of them writes. A 409.
const NOT_STANDING: &str = "fleet.clash_not_standing";
/// A file the clash does not list. A 422.
const PATH_NOT_IN_CLASH: &str = "fleet.clash_path_not_in_clash";
/// Git would not answer. A 500.
const UNREADABLE: &str = "fleet.clash_unreadable";

/// The pair's worktrees whose clash on `path` is what the clash names: one that both changed
/// themselves, as `checking::between_any` has it, since a worktree cut from a newer main carries
/// main's changes and git sets them against the older one's.
fn file_between_any(
    one: &[PathBuf],
    other: &[PathBuf],
    base: &str,
    path: &str,
) -> Result<Option<FileDiffs>, NotChecked> {
    let mut unknown = None;
    for a in one {
        for b in other.iter().filter(|b| *b != a) {
            match clash::between(a, b) {
                Ok(Verdict::Conflict(found)) if found.iter().any(|one| one == path) => {
                    match (changed_since_base(a, base), changed_since_base(b, base)) {
                        (Ok(mine), Ok(theirs)) if mine.contains(path) && theirs.contains(path) => {
                            return clash::file_between(a, b, path).map(Some);
                        }
                        (Err(why), _) | (_, Err(why)) => unknown = unknown.or(Some(why)),
                        _ => {}
                    }
                }
                Ok(_) => {}
                Err(why) => unknown = unknown.or(Some(why)),
            }
        }
    }
    unknown.map_or(Ok(None), Err)
}

/// The first of the holder's worktrees that conflicts with main's `tip` on `path`.
fn file_against_main(worktrees: &[PathBuf], tip: &str, path: &str) -> Result<Option<FileDiffs>, NotChecked> {
    let mut unknown = None;
    for worktree in worktrees {
        match clash::against(worktree, tip) {
            Ok(Verdict::Conflict(found)) if found.iter().any(|one| one == path) => {
                return clash::file_against(worktree, tip, path).map(Some);
            }
            Ok(_) => {}
            Err(why) => unknown = unknown.or(Some(why)),
        }
    }
    unknown.map_or(Ok(None), Err)
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
    fn clash_refusal(&self, code: &'static str, message: String) -> WireError {
        WireError::raised(code, message, self.run_id())
    }

    fn no_longer_standing(&self, path: &str) -> Refusal {
        Refusal::IllegalMove(self.clash_refusal(
            NOT_STANDING,
            format!("the clash over `{path}` no longer stands"),
        ))
    }

    /// `path` of the clash between `session_id` and the other holder, as each changed it.
    pub(crate) async fn clash_file(
        &self,
        session_id: &str,
        other_kind: ClashSide,
        other_id: &str,
        path: &str,
    ) -> Result<ClashFile, Refusal> {
        let who = Who::Session(session_id.to_string());
        let other = Who::Session(other_id.to_string());
        let opposed = match other_kind {
            ClashSide::Main => Opposed(who.clone(), Side::Main),
            ClashSide::Session if other != who => Opposed::pair(&who, &other),
            ClashSide::Session => return Err(self.no_longer_standing(path)),
        };
        let Some(reading) = self.writes().reading(&opposed) else {
            return Err(self.no_longer_standing(path));
        };
        if !reading.paths.contains(path) {
            return Err(Refusal::Unacceptable(self.clash_refusal(
                PATH_NOT_IN_CLASH,
                format!("`{path}` is not one of the files this clash is over"),
            )));
        }
        let Some(mine) = self.writing_in(&who).await else {
            return Err(self.no_longer_standing(path));
        };
        let Some(served) = self.repositories().serving(&mine.manifest) else {
            return Err(self.no_longer_standing(path));
        };
        let base = base_of(&served);
        let wanted = path.to_string();
        let read = match other_kind {
            ClashSide::Main => {
                let Some(tip) = self.writes().main_tip(&mine.manifest) else {
                    return Err(self.no_longer_standing(path));
                };
                tokio::task::spawn_blocking(move || file_against_main(&mine.worktrees, &tip, &wanted)).await
            }
            ClashSide::Session => {
                let Some(theirs) = self.writing_in(&other).await.filter(|theirs| theirs.manifest == mine.manifest) else {
                    return Err(self.no_longer_standing(path));
                };
                tokio::task::spawn_blocking(move || {
                    file_between_any(&mine.worktrees, &theirs.worktrees, &base, &wanted)
                })
                .await
            }
        }
        .unwrap_or_else(|why| Err(NotChecked(format!("the read did not finish: {why}"))));
        match read {
            Ok(Some(FileDiffs { ours, theirs })) => Ok(ClashFile { ours, theirs }),
            Ok(None) => Err(self.no_longer_standing(path)),
            Err(why) => Err(Refusal::Fault(self.clash_refusal(UNREADABLE, why.to_string()))),
        }
    }
}
