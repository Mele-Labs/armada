//! The files of a test another Job is fixing, held off every other Job that hit
//! it, until the fix has landed and the Job's own copy has it. #1673.
//!
//! `docs/concepts/fleet.md`, *A test another Job is fixing*, has which files,
//! the places a Job is held, and why the hold outlives the merge.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{under, BreakageClaim, JobId, LandedHold, RepoPath};
use verification::TheBaseMoved;

use config::settings as keys;

use crate::daemon::Fleet;
use crate::fixing::FixStands;
use crate::peers::News;
use crate::prompts::Prompts;

/// One fix holding files off this Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hold {
    pub(crate) title: String,
    pub(crate) handle: String,
    pub(crate) test: String,
    pub(crate) paths: Vec<RepoPath>,
    /// The fix has landed and this Job's copy has not taken it yet.
    pub(crate) landed: bool,
}

/// Everything held off one Job. **Empty holds nothing**, which is every Job
/// that hit no test another Job is fixing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldOff(Vec<Hold>);

impl HeldOff {
    pub(crate) fn of(holds: Vec<Hold>) -> HeldOff {
        HeldOff(
            holds
                .into_iter()
                .filter(|hold| !hold.paths.is_empty())
                .collect(),
        )
    }

    /// Every held path once, in the order the holds name them.
    pub(crate) fn paths(&self) -> Vec<RepoPath> {
        let mut paths: Vec<RepoPath> = Vec::new();
        for path in self.0.iter().flat_map(|hold| &hold.paths) {
            if !paths.contains(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// The first fix holding any of `asked`, and which of them it holds. Either
    /// side may be the directory: one declared over a held file reaches it.
    pub(crate) fn refusing(&self, asked: &[RepoPath]) -> Option<(&Hold, Vec<RepoPath>)> {
        self.0.iter().find_map(|hold| {
            let held: Vec<RepoPath> = asked
                .iter()
                .filter(|path| {
                    hold.paths.iter().any(|kept| {
                        under(kept.as_str(), path.as_str()) || under(path.as_str(), kept.as_str())
                    })
                })
                .cloned()
                .collect();
            (!held.is_empty()).then_some((hold, held))
        })
    }

    /// The opening brief's block, where anything is held.
    pub(crate) fn text(&self, prompts: &Prompts) -> Option<String> {
        if self.0.is_empty() {
            return None;
        }
        let mut text = String::from(prompts.get(keys::PROMPT_FIX_HELD_OFF));
        text.push('\n');
        for hold in &self.0 {
            let doing = match hold.landed {
                false => prompts.fill(keys::PROMPT_FIX_HELD_OFF_FIXING, &[("test", &hold.test)]),
                true => prompts.fill(keys::PROMPT_FIX_HELD_OFF_LANDED, &[("test", &hold.test)]),
            };
            text.push_str(&format!(
                "\n- \"{}\" ({}) {doing}: {}.",
                hold.title,
                hold.handle,
                listed(&hold.paths)
            ));
        }
        Some(text)
    }
}

/// Every path, back-quoted. **All of them**, unlike a peer turn's three: a path
/// left out of this list is a path the Drone will edit.
pub(crate) fn listed(paths: &[RepoPath]) -> String {
    paths
        .iter()
        .map(|path| format!("`{}`", path.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
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
    /// The paths one claim holds: the files the reporter named, then what the
    /// fix has declared. A fix that cannot be read holds its named files.
    pub(crate) async fn held_by(&self, claim: &BreakageClaim) -> Vec<RepoPath> {
        let mut paths = claim.files.clone();
        let declared = match self.load(&claim.fix).await {
            Ok(fix) => self.scope_claims(&fix).await.unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        for path in declared.iter().flat_map(|claimed| &claimed.paths) {
            if !paths.contains(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// What is held off this Job now: each standing claim it reported or is
    /// pointed at and is not the fix for, and each landed fix its copy lacks.
    /// **A read that will not answer holds nothing**, `pointed_at_fixes`' rule.
    pub(crate) async fn held_off(&self, job: &JobId) -> HeldOff {
        let (claims, landed) = {
            let store = self.store().lock().await;
            let mut claims = store.breakages_reported_by(job).unwrap_or_default();
            for pointer in store.fixes_waited_on_by(job).unwrap_or_default() {
                let standing = store
                    .breakage_claimed(&pointer.repository, &pointer.check, &pointer.test)
                    .ok()
                    .flatten();
                if let Some(claim) = standing.filter(|claim| !claims.contains(claim)) {
                    claims.push(claim);
                }
            }
            claims.retain(|claim| claim.fix != *job);
            (claims, store.landed_holds_on(job).unwrap_or_default())
        };
        let mut holds = Vec::with_capacity(claims.len() + landed.len());
        for claim in &claims {
            let (title, handle) = self.named(&claim.fix).await;
            holds.push(Hold {
                title,
                handle,
                test: claim.breakage.test.clone(),
                paths: self.held_by(claim).await,
                landed: false,
            });
        }
        for hold in landed {
            let (title, handle) = self.named(&hold.fix).await;
            holds.push(Hold {
                title,
                handle,
                test: hold.test,
                paths: hold.paths,
                landed: true,
            });
        }
        HeldOff::of(holds)
    }

    /// A fix settled: where it landed, what each of its claims held stays held
    /// off the Jobs on the claim until their copies take it; where it ended
    /// without landing, the files are free now. Each is told which. **Before
    /// the claims are given back**, since it reads them. The Jobs told, so
    /// `fix_settled` does not tell them a second time.
    pub(crate) async fn settled_holds(&self, fix: &JobId, landed: bool) -> Vec<JobId> {
        let (claims, waiting) = {
            let store = self.store().lock().await;
            (
                store.breakages_claimed_by(fix).unwrap_or_default(),
                store.waiting_on_fix(fix).unwrap_or_default(),
            )
        };
        let (title, handle) = self.named(fix).await;
        let mut told = Vec::new();
        for claim in claims {
            let paths = self.held_by(&claim).await;
            if paths.is_empty() {
                continue;
            }
            let mut held = vec![claim.reported_by.clone()];
            for waiter in &waiting {
                if waiter.check == claim.breakage.check
                    && waiter.test == claim.breakage.test
                    && !held.contains(&waiter.waiting)
                {
                    held.push(waiter.waiting.clone());
                }
            }
            for job in held.into_iter().filter(|job| job != fix) {
                if landed {
                    let hold = LandedHold {
                        held: job.clone(),
                        fix: fix.clone(),
                        test: claim.breakage.test.clone(),
                        paths: paths.clone(),
                    };
                    let now = self.now();
                    let _ = self.store().lock().await.hold_until_caught_up(&hold, &now);
                }
                let stands = match landed {
                    true => FixStands::Landed,
                    false => FixStands::Gone,
                };
                self.owe(
                    &job,
                    News::Fix {
                        title: title.clone(),
                        handle: handle.clone(),
                        test: claim.breakage.test.clone(),
                        stands,
                        files: words(&paths),
                    },
                )
                .await;
                told.push(job);
            }
        }
        told
    }

    /// This Job's branch was just caught up as a Drone is put on it: what
    /// landed fixes held is free unless git could not put the branch on the
    /// base, and the Drone hears the fix is in its copy. `None` was not behind.
    pub(crate) async fn holds_caught_up(&self, job: &JobId, moved: Option<&TheBaseMoved>) {
        if matches!(moved, Some(TheBaseMoved::CouldNotFollow { .. })) {
            return;
        }
        let landed = {
            let mut store = self.store().lock().await;
            let landed = store.landed_holds_on(job).unwrap_or_default();
            if landed.is_empty() || store.release_landed_holds(job).is_err() {
                return;
            }
            landed
        };
        for hold in landed {
            let (title, handle) = self.named(&hold.fix).await;
            self.unowe_landed(job, &hold.test).await;
            self.owe(
                job,
                News::Fix {
                    title,
                    handle,
                    test: hold.test,
                    stands: FixStands::InYourCopy,
                    files: words(&hold.paths),
                },
            )
            .await;
        }
    }

    /// A Job's title and handle, or its bare id where it has been forgotten.
    async fn named(&self, job: &JobId) -> (String, String) {
        match self.load(job).await {
            Ok(record) => (record.title().as_str().to_string(), record.handle()),
            Err(_) => (String::new(), job.as_str().to_string()),
        }
    }
}

fn words(paths: &[RepoPath]) -> Vec<String> {
    paths.iter().map(|path| path.as_str().to_string()).collect()
}
