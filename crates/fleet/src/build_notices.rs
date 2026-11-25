//! Telling sessions which build Fleet and Bridge now run, so the one that restarted them is not
//! the only one who knows. `docs/concepts/fleet.md`, *Telling sessions the build they now run*.
//!
//! **Read on the sweep, never at boot**: `scripts/restart` writes `restart-commit` after the new
//! Fleet answers, so a boot would read the build before it. A round runs when the commit on file
//! is not the one this process last settled, and not while `restart-build.status` says a restart
//! is under way.
//!
//! **Told once**: the key is (build commit, session) in the store, so a later Fleet does not
//! repeat it. A session that cannot be told now is asked again next reading, with nothing kept.
//! Who is told: a session whose branch the build newly holds, holds newer commits of, or dropped,
//! with those branches only: a rebuild that leaves its branches where they were says nothing.
//! A wait for a restart is never settled by this: a fact is told, and the item stays the person's.
//! A branch the preview left out is told with the reason `scripts/preview` recorded for it, and
//! that is news the first time even to a session never told the branch was in.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::BranchInBuild;
use ipc::BuildSource;
use store::{AttachmentState, HolderKind, SessionState, Store};

use crate::building::{broke_in, commit_in, left_out_in, restarting_in, source_in, Broke, LeftOut};
use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::pull_notices::short;
use crate::repositories::Served;
use crate::session_host::address_of;

/// What a session is told, in bare facts. `why` is what the preview recorded of each branch it left out.
fn said(
    build: BuildSource,
    commit: &str,
    branches: &[(String, BranchInBuild)],
    why: &BTreeMap<String, LeftOut>,
) -> String {
    let which = match build {
        BuildSource::Preview => "the preview",
        BuildSource::Main => "main",
    };
    let mut out = format!("Fleet and Bridge now run {which} at {}.", short(commit));
    for (branch, standing) in branches {
        out.push('\n');
        out.push_str(&match standing {
            BranchInBuild::In { .. } => format!("{branch} is in it."),
            BranchInBuild::NotIn => match why.get(branch) {
                Some(left) => format!("{branch} is not in it: {}.", left.why),
                None => format!("{branch} is not in it."),
            },
            BranchInBuild::Partly { newest, since: 1 } => {
                format!("{branch} is in it up to {}; 1 commit since is not.", short(newest))
            }
            BranchInBuild::Partly { newest, since } => {
                format!("{branch} is in it up to {}; {since} commits since are not.", short(newest))
            }
        });
    }
    out
}

/// What a session is told of a preview round that did not build. `others` are the other sessions told the same.
fn said_broke(round: &Broke, mine: &[&(String, String, String)], others: &[String]) -> String {
    let mut out = format!("The preview at {} did not build: {}.", short(&round.commit), round.error.trim_end_matches('.'));
    for (branch, _, what) in mine {
        out.push_str(&format!("\n{branch}: {what}."));
    }
    if !others.is_empty() {
        out.push_str(&format!("\nAlso told: {}. Message them and agree which of you fixes it.", others.join(", ")));
    }
    out
}

/// What a session was last told of a branch, as kept: none of it in the build, or the newest
/// commit of it the build holds.
const OUT: &str = "out";

fn kept_as(standing: &BranchInBuild) -> String {
    match standing {
        BranchInBuild::NotIn => OUT.to_string(),
        BranchInBuild::In { tip } => tip.clone(),
        BranchInBuild::Partly { newest, .. } => newest.clone(),
    }
}

/// The branches each live Session of `served` stands on, as the slot pool allows, by Session.
pub(crate) fn branches_held(store: &Store, served: &Served) -> BTreeMap<String, BTreeSet<String>> {
    let manifest = served.manifest().id().as_str();
    let live = |id: &str| store.session(id).ok().flatten().is_some_and(|one| one.state != SessionState::Ended);
    let mut held: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in store.standing_of_kind("branch", manifest).unwrap_or_default() {
        if row.holder.kind == HolderKind::Session
            && row.state == AttachmentState::Standing
            && live(&row.holder.id)
            && crate::slot_holders::pool_allows_branch(store, manifest, served.root(), &row.holder.id, &row.target)
        {
            held.entry(row.holder.id).or_default().insert(row.target);
        }
    }
    held
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
    /// The build on file, once an interval. Its own gate, beside the pull requests'.
    pub(crate) async fn builds_told_when_due(self: &Arc<Self>) {
        let now = self.now();
        {
            let mut sweep = self.sweeping().lock().await;
            if let Some(last) = sweep.builds_told_last.as_ref() {
                if elapsed(last, &now) < self.noticing().interval() {
                    return;
                }
            }
            sweep.builds_told_last = Some(now);
        }
        self.broken_builds_told().await;
        self.builds_told().await;
    }

    /// Tell each live Session holding a branch a failed preview round named, once per failed commit.
    /// **Not held back by a restart under way**: a round can fail and the script go again while the status
    /// says running, and the Jobs should hear at once. Nothing here fails the turn.
    pub(crate) async fn broken_builds_told(self: &Arc<Self>) {
        let support = crate::runtime::support_dir(&self.host().home);
        let broke = broke_in(&support);
        if broke.is_empty() {
            return;
        }
        for served in self.repositories().served() {
            let held = branches_held(&*self.store().lock().await, &served);
            for round in &broke {
                self.broke_told_in(&held, round).await;
            }
        }
    }

    /// One repository's sessions for one failed round. A session that cannot be told keeps nothing, to be asked again.
    async fn broke_told_in(self: &Arc<Self>, held: &BTreeMap<String, BTreeSet<String>>, round: &Broke) {
        let named: BTreeMap<&String, Vec<&(String, String, String)>> = held
            .iter()
            .map(|(session, branches)| (session, round.branches.iter().filter(|(branch, ..)| branches.contains(branch)).collect::<Vec<_>>()))
            .filter(|(_, mine)| !mine.is_empty())
            .collect();
        let key = format!("broke:{}", round.commit);
        for (session, mine) in &named {
            if self.store().lock().await.build_notice_kept(&key, session).unwrap_or(true) {
                continue;
            }
            let others = named.iter().filter(|(other, _)| other != &session).map(|(other, theirs)| {
                let branches: Vec<&str> = theirs.iter().map(|(branch, ..)| branch.as_str()).collect();
                format!("{} ({})", address_of(other), branches.join(", "))
            });
            let text = said_broke(round, mine, &others.collect::<Vec<_>>());
            if !matches!(self.told_news_by_fleet(session, &text, ipc::Notice::BuildMoved).await, Ok(true)) {
                continue;
            }
            let at = self.now();
            let _ = self.store().lock().await.keep_build_notice(&key, session, &at);
        }
    }

    /// An item that asks him to refresh or restart Fleet settles itself once Fleet runs a build made after the item
    /// was raised. **The build is the one this process came from**, as the runtime file names it
    /// (`runtime::kept_build_of`), and not whatever `builds/current` points at: a build kept and not yet swapped
    /// onto is not the one he runs. Fleet tells the Job as a dismissal does, so it does not raise the item again.
    pub(crate) async fn build_asks_settled(self: &Arc<Self>) {
        let support = crate::runtime::support_dir(&self.host().home);
        let running = std::env::current_exe().ok().and_then(|exe| crate::runtime::kept_build_of(&exe, &support));
        self.build_asks_settled_on(running).await;
    }

    /// [`Self::build_asks_settled`] for the build named: an item raised before it was made goes, one raised on or
    /// after stays. Nothing here fails the turn.
    pub(crate) async fn build_asks_settled_on(self: &Arc<Self>, running: Option<String>) {
        let support = crate::runtime::support_dir(&self.host().home);
        let Some(running) = running.and_then(|name| crate::kept_builds::kept_named(&support.join(crate::kept_builds::BUILDS), &name)) else {
            return;
        };
        let Some(built) = core_model::Timestamp::from_rfc3339(running.built_at).epoch_millis() else {
            return;
        };
        let asks = self.store().lock().await.waiting_run_items().unwrap_or_default();
        for (session, item, since, command) in asks {
            let restart = matches!(command.trim(), "scripts/preview --restart" | "scripts/restart" | "scripts/preview");
            let before = core_model::Timestamp::from_rfc3339(since).epoch_millis().is_some_and(|raised| raised < built);
            if !(restart && before) {
                continue;
            }
            let dismiss = ipc::DismissWaiting {
                session_id: ipc::SessionId::carried(&session),
                item_id: item,
                reason: Some("Fleet is running a build made after you asked, so it is done".into()),
                misread: false,
            };
            let _ = Arc::clone(self).dismiss_waited(dismiss).await;
        }
    }

    /// Tell the sessions that care about the build on file, unless this process settled it
    /// already. **Nothing here fails the turn.**
    pub(crate) async fn builds_told(self: &Arc<Self>) {
        let support = crate::runtime::support_dir(&self.host().home);
        let (Ok(build), Some(commit)) = (source_in(&support), commit_in(&support)) else {
            return;
        };
        if restarting_in(&support, SystemTime::now()) {
            return;
        }
        self.build_kept(&support, build, &commit).await;
        if self.sweeping().lock().await.build_told.as_deref() == Some(commit.as_str()) {
            return;
        }
        let why = match build {
            BuildSource::Preview => left_out_in(&support, &commit),
            BuildSource::Main => BTreeMap::new(),
        };
        let mut everyone = true;
        for served in self.repositories().served() {
            everyone &= self.build_told_in(&served, build, &commit, &why).await;
        }
        if everyone {
            self.sweeping().lock().await.build_told = Some(commit);
        }
    }

    /// One repository's sessions. **False where one could not be told**, to be asked again.
    async fn build_told_in(
        self: &Arc<Self>,
        served: &Served,
        build: BuildSource,
        commit: &str,
        why: &BTreeMap<String, LeftOut>,
    ) -> bool {
        let mut everyone = true;
        let held = branches_held(&*self.store().lock().await, served);
        for (session, branches) in held {
            let kept = self.store().lock().await.build_notice_kept(commit, &session).unwrap_or(true);
            if kept {
                continue;
            }
            let standing = self.branches_in_build(served, commit, &branches).await;
            let moved = self.branches_moved(&session, &standing, why).await;
            if moved.is_empty() {
                continue;
            }
            let told = matches!(self.told_news_by_fleet(&session, &said(build, commit, &moved, why), ipc::Notice::BuildMoved).await, Ok(true));
            if !told {
                everyone = false;
                continue;
            }
            let at = self.now();
            {
                let mut store = self.store().lock().await;
                let _ = store.keep_build_notice(commit, &session, &at);
                for (branch, one) in &standing {
                    let _ = store.keep_build_branch_told(&session, branch, &kept_as(one), &at);
                }
            }
        }
        everyone
    }

    /// The branches whose standing is news to this session: newly in the build or newer commits of
    /// it, or out of it after it was told it was in, or out of it with a reason in `why` and not
    /// told out already. Out of a build with no reason, never told in, is not.
    async fn branches_moved(
        &self,
        session: &str,
        standing: &[(String, BranchInBuild)],
        why: &BTreeMap<String, LeftOut>,
    ) -> Vec<(String, BranchInBuild)> {
        let store = self.store().lock().await;
        standing
            .iter()
            .filter(|(branch, one)| {
                let Ok(last) = store.build_branch_told(session, branch) else {
                    return false;
                };
                let now = kept_as(one);
                last.as_deref() != Some(now.as_str()) && (now != OUT || last.is_some() || why.contains_key(branch))
            })
            .cloned()
            .collect()
    }

    /// Where each branch a session holds stands in the build. A branch git will not say is left out.
    async fn branches_in_build(
        &self,
        served: &Served,
        commit: &str,
        branches: &BTreeSet<String>,
    ) -> Vec<(String, BranchInBuild)> {
        let (root, commit, branches) = (served.root().to_string(), commit.to_string(), branches.clone());
        tokio::task::spawn_blocking(move || {
            branches
                .into_iter()
                .filter_map(|branch| {
                    adapters::branch_in_build(&root, &commit, &branch).map(|standing| (branch, standing))
                })
                .collect()
        })
        .await
        .unwrap_or_default()
    }
}
