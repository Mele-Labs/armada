//! The pass: which pairs are due, what git says of each, and who is told. Run by the notice loop's
//! tick, so it costs a lock and an empty table where nothing was written.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::clash::{self, NotChecked, Verdict};
use core_model::{Component, Envelope, FieldValue, JobId, Level, Ulid};

use super::index::Changed;
use super::standing::Standing;
use super::{Opposed, Side, Who};
use crate::daemon::Fleet;

/// How many pairs one pass checks. A tree snapshot of a large repository is not free, and what is
/// left is checked on the next tick.
const PER_PASS: usize = 3;

/// How long a fetch of the base may take before main's tip is read as it already is here.
const FETCH_WITHIN: Duration = Duration::from_secs(20);

/// What a holder's changes did against main's tip.
enum Against {
    Clean,
    /// The conflicted files, and for the first the commit on main that changed it.
    Conflict(BTreeSet<String>, Option<String>),
    /// Nothing main changed is anything the holder wrote.
    Apart,
    Unknown(NotChecked),
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
    /// One pass: main's moves, the pairs whose wait is over, and anyone not yet told what was found.
    pub(crate) async fn clashes_checked(self: &Arc<Self>) {
        if self.writes().first_pass() {
            self.writes_seeded().await;
        }
        if self.writes().idle() {
            return;
        }
        self.main_moved_on().await;
        let due = self.writes().due(&self.now());
        for opposed in due.into_iter().take(PER_PASS) {
            self.pair_checked(&opposed).await;
        }
        self.clashes_told().await;
        self.notes_sent().await;
        self.tell_peers().await;
    }

    /// **The table is memory only**, so a restart forgets every file each holder wrote. The first
    /// pass credits each live Session and Job with what its worktrees changed since they left the
    /// base, so a pair that clashed before the restart is due again without either writing.
    async fn writes_seeded(&self) {
        let mut holders = Vec::new();
        {
            let store = self.store().lock().await;
            let live = store::SessionSearch {
                manifest_id: None,
                state: Some(store::SessionState::Live),
                text: None,
            };
            let sessions = store.find_sessions(&live).unwrap_or_default();
            holders.extend(sessions.into_iter().map(|one| Who::Session(one.id)));
            for served in self.repositories().served() {
                let slots = store.standing_of_kind("slot", served.manifest().id().as_str()).unwrap_or_default();
                holders.extend(
                    slots
                        .into_iter()
                        .filter(|row| row.holder.kind == store::HolderKind::Job)
                        .map(|row| Who::Job(JobId::carried(Ulid::carried(row.holder.id)))),
                );
            }
        }
        let now = self.now();
        for who in holders {
            let Some(standing) = self.writing_in(&who).await else {
                continue;
            };
            let Some(served) = self.repositories().serving(&standing.manifest) else {
                continue;
            };
            let base = served.manifest().base().unwrap_or("main").to_string();
            let worktrees = standing.worktrees.clone();
            let changed = tokio::task::spawn_blocking(move || {
                worktrees
                    .iter()
                    .flat_map(|worktree| {
                        clash::changed_here(worktree, &format!("origin/{base}"))
                            .or_else(|_| clash::changed_here(worktree, &base))
                            .unwrap_or_default()
                    })
                    .collect::<BTreeSet<String>>()
            })
            .await
            .unwrap_or_default();
            if !changed.is_empty() {
                self.writes().wrote(&who, &standing.manifest, changed, &now);
            }
        }
    }

    /// Check one pair now.
    pub(crate) async fn pair_checked(self: &Arc<Self>, opposed: &Opposed) {
        self.writes().taken(opposed);
        let Side::Holder(other) = &opposed.1 else {
            return self.holder_against_main(&opposed.0).await;
        };
        let (Some(one), Some(two)) = (
            self.writing_in_or_left(&opposed.0).await,
            self.writing_in_or_left(other).await,
        ) else {
            return;
        };
        if one.manifest != two.manifest {
            return;
        }
        let (a, b) = (one.worktrees.clone(), two.worktrees.clone());
        let found = tokio::task::spawn_blocking(move || between_any(&a, &b))
            .await
            .unwrap_or_else(|why| Err(NotChecked(format!("the check did not finish: {why}"))));
        self.verdict_kept(opposed, found, None).await;
    }

    /// A holder's standing, or nothing after it has been dropped from the table.
    async fn writing_in_or_left(&self, who: &Who) -> Option<Standing> {
        let standing = self.writing_in(who).await;
        if standing.is_none() {
            self.writes().left(who);
        }
        standing
    }

    /// Keep what a check found, and tell whoever it is new to.
    async fn verdict_kept(
        self: &Arc<Self>,
        opposed: &Opposed,
        found: Result<Option<BTreeSet<String>>, NotChecked>,
        landed: Option<String>,
    ) {
        let found = match found {
            Ok(found) => found,
            Err(why) => return self.not_checked(opposed, &why).await,
        };
        if let Changed::Set(_) = self.writes().found(opposed, found, landed, &self.now()) {
            self.clash_announced(opposed).await;
        }
    }

    /// Git would not say. **Not a clash and not a clear**: the last reading stands. It is written to
    /// the log of each Job in the pair; two Sessions have none to write it to, and the next write
    /// asks again.
    async fn not_checked(&self, opposed: &Opposed, why: &NotChecked) {
        let jobs = [Some(&opposed.0), match &opposed.1 {
            Side::Holder(other) => Some(other),
            Side::Main => None,
        }]
        .into_iter()
        .flatten()
        .filter_map(|who| match who {
            Who::Job(job) => Some(job),
            Who::Session(_) => None,
        });
        for job in jobs {
            let envelope = Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                "git would not say whether this Job's changes conflict with another's, so none was reported",
            )
            .in_job(job.as_ulid().clone())
            .with_field("cause", FieldValue::Str(why.to_string()));
            self.noted_in_the_log(job, &envelope);
        }
    }

    /// Main's tip moved: every standing pair is asked again, since one may have merged main in or
    /// landed, and each live holder is set against main's new tip.
    async fn main_moved_on(self: &Arc<Self>) {
        for (manifest, tip) in self.writes().main_moves() {
            let Some(served) = self.repositories().serving(&manifest) else {
                continue;
            };
            if !self.tip_held(served.root(), served.manifest().base(), &tip).await {
                continue;
            }
            let open: Vec<Opposed> = self
                .writes()
                .readings_in(&manifest)
                .into_iter()
                .map(|(opposed, _)| opposed)
                .filter(|opposed| matches!(opposed.1, Side::Holder(_)))
                .collect();
            for opposed in open {
                self.pair_checked(&opposed).await;
            }
            let holders: Vec<Who> = self
                .writes()
                .holders()
                .into_iter()
                .filter(|(_, held_in, _)| *held_in == manifest)
                .map(|(who, ..)| who)
                .collect();
            for who in holders {
                self.holder_against_main(&who).await;
            }
        }
    }

    /// Whether `tip` is a commit this machine holds, fetching the base once if not.
    async fn tip_held(&self, root: &str, base: Option<&str>, tip: &str) -> bool {
        let (here, owned) = (PathBuf::from(root), tip.to_string());
        let held = {
            let (here, owned) = (here.clone(), owned.clone());
            tokio::task::spawn_blocking(move || clash::holds(&here, &owned)).await.unwrap_or(false)
        };
        let Some(base) = base.filter(|_| !held) else {
            return held;
        };
        let (root, base) = (root.to_string(), base.to_string());
        tokio::task::spawn_blocking(move || {
            adapters::fetch_branch(&root, &base, FETCH_WITHIN).is_ok() && clash::holds(&here, &owned)
        })
        .await
        .unwrap_or(false)
    }

    /// One holder's changes against main's tip as last seen. **Told to that holder only.**
    pub(crate) async fn holder_against_main(self: &Arc<Self>, who: &Who) {
        let Some(standing) = self.writing_in_or_left(who).await else {
            return;
        };
        let Some(tip) = self.writes().main_tip(&standing.manifest) else {
            return;
        };
        let touched = self
            .writes()
            .holders()
            .into_iter()
            .find(|(held, ..)| held == who)
            .map(|(_, _, paths)| paths)
            .unwrap_or_default();
        let worktrees = standing.worktrees.clone();
        let reads = tokio::task::spawn_blocking(move || {
            worktrees.iter().map(|worktree| against_main(worktree, &tip, &touched)).collect::<Vec<_>>()
        })
        .await
        .unwrap_or_else(|why| vec![Against::Unknown(NotChecked(format!("the check did not finish: {why}")))]);
        let opposed = Opposed(who.clone(), Side::Main);
        // Any slot's conflict is the holder's; a slot git would not read leaves the last reading.
        let (mut paths, mut landed, mut unknown) = (BTreeSet::new(), None, None);
        for read in reads {
            match read {
                Against::Clean | Against::Apart => {}
                Against::Conflict(found, first) => {
                    paths.extend(found);
                    landed = landed.or(first);
                }
                Against::Unknown(why) => unknown = unknown.or(Some(why)),
            }
        }
        self.verdict_kept(&opposed, merged(paths, unknown), landed).await;
    }
}

/// Every pair of one holder's worktrees and the other's: the files any pair conflicts on.
fn between_any(one: &[PathBuf], other: &[PathBuf]) -> Result<Option<BTreeSet<String>>, NotChecked> {
    let (mut paths, mut unknown) = (BTreeSet::new(), None);
    for a in one {
        for b in other.iter().filter(|b| *b != a) {
            match clash::between(a, b) {
                Ok(Verdict::Clean) => {}
                Ok(Verdict::Conflict(found)) => paths.extend(found),
                Err(why) => unknown = unknown.or(Some(why)),
            }
        }
    }
    merged(paths, unknown)
}

/// What several worktrees' checks come to: **not checked** only where none conflicts and one
/// could not be read.
fn merged(paths: BTreeSet<String>, unknown: Option<NotChecked>) -> Result<Option<BTreeSet<String>>, NotChecked> {
    match (paths.is_empty(), unknown) {
        (false, _) => Ok(Some(paths)),
        (true, Some(why)) => Err(why),
        (true, None) => Ok(None),
    }
}

/// Whether anything main changed since it and the worktree met is a file the holder wrote, and if so
/// whether the two conflict.
fn against_main(worktree: &std::path::Path, tip: &str, touched: &BTreeSet<String>) -> Against {
    let changed = match clash::changed_on(worktree, tip) {
        Ok(changed) => changed,
        Err(why) => return Against::Unknown(why),
    };
    if !changed.iter().any(|path| touched.contains(path)) {
        return Against::Apart;
    }
    match clash::against(worktree, tip) {
        Ok(Verdict::Clean) => Against::Clean,
        Ok(Verdict::Conflict(paths)) => {
            let landed = paths.first().and_then(|path| clash::landed_in(worktree, tip, path));
            Against::Conflict(paths.into_iter().collect(), landed)
        }
        Err(why) => Against::Unknown(why),
    }
}
