//! The pass: which pairs are due, what git says of each, and who is told. Run by the notice loop's
//! tick, so it costs a lock and an empty table where nothing was written.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::clash::{self, NotChecked, Verdict};
use core_model::{Component, Envelope, FieldValue, Level};

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
        let (a, b) = (one.worktree.clone(), two.worktree.clone());
        let verdict = tokio::task::spawn_blocking(move || clash::between(&a, &b))
            .await
            .unwrap_or_else(|why| Err(NotChecked(format!("the check did not finish: {why}"))));
        self.verdict_kept(opposed, verdict.map(|verdict| match verdict {
            Verdict::Clean => None,
            Verdict::Conflict(paths) => Some(paths.into_iter().collect()),
        }), None)
        .await;
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

    /// Git would not say. **Not a clash and not a clear**: the last reading stands.
    async fn not_checked(&self, opposed: &Opposed, why: &NotChecked) {
        let job = match (&opposed.0, &opposed.1) {
            (Who::Job(job), _) | (_, Side::Holder(Who::Job(job))) => Some(job),
            _ => None,
        };
        match job {
            Some(job) => {
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
            None => eprintln!("a write clash could not be checked: {why}"),
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
        let worktree = standing.worktree.clone();
        let read = tokio::task::spawn_blocking(move || against_main(&worktree, &tip, &touched))
            .await
            .unwrap_or_else(|why| Against::Unknown(NotChecked(format!("the check did not finish: {why}"))));
        let opposed = Opposed(who.clone(), Side::Main);
        let (found, landed) = match read {
            Against::Clean | Against::Apart => (Ok(None), None),
            Against::Conflict(paths, landed) => (Ok(Some(paths)), landed),
            Against::Unknown(why) => (Err(why), None),
        };
        self.verdict_kept(&opposed, found, landed).await;
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
