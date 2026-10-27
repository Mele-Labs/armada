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
//! Who is told: a live session waiting on the person to restart onto this build — the act Bridge
//! draws as Refresh preview or Update to main — as the answer to that wait, with every branch it
//! holds. And a session whose branch the build newly holds, holds newer commits of, or dropped,
//! with those branches only: a rebuild that leaves its branches where they were says nothing.
//! A branch the preview left out is told with the reason `scripts/preview` recorded for it, and
//! that is news the first time even to a session never told the branch was in.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::BranchInBuild;
use ipc::{AnswerWaiting, BuildSource, SessionId, WaitingActKind};
use store::{AttachmentState, HolderKind, SessionSearch, SessionState};

use crate::building::{build_word, commit_in, left_out_in, restarting_in, source_in, LeftOut};
use crate::clock::elapsed;
use crate::daemon::Fleet;
use crate::pull_notices::short;
use crate::repositories::Served;

/// The build a restart command asks for, and whether it adopts a working Drone. `None` where the
/// command is anything else, `--dry-run` among them. **The same reading as Bridge's
/// `build-change.ts`**: a restart handed over as a command is the build switch, pressed.
pub(crate) fn restart_asked(command: &str) -> Option<(BuildSource, bool)> {
    let mut words = command.split_whitespace();
    let script = words.next().unwrap_or_default();
    let flags: Vec<&str> = words.collect();
    let named = |name: &str| {
        script
            .strip_suffix(name)
            .is_some_and(|before| before.is_empty() || before.ends_with('/'))
    };
    let (build, flag) = if named("scripts/preview") {
        (BuildSource::Preview, "--restart")
    } else if named("scripts/restart") {
        (BuildSource::Main, "--main")
    } else {
        return None;
    };
    let known = flags.iter().all(|one| *one == flag || *one == "--adopt");
    (flags.contains(&flag) && known).then(|| (build, flags.contains(&"--adopt")))
}

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

fn answer(session: &str, item: String, text: &str) -> AnswerWaiting {
    AnswerWaiting {
        session_id: SessionId::carried(session),
        item_id: item,
        choice: None,
        text: Some(text.to_string()),
        mode: None,
        fixed: None,
    }
}

/// A session's reasons to be told: the branches it holds, and the waits that ask for a restart.
#[derive(Default)]
struct Reasons {
    branches: BTreeSet<String>,
    restarts: Vec<(String, BuildSource)>,
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
        self.builds_told().await;
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
        for (session, reasons) in self.build_reasons(served).await {
            let kept = self.store().lock().await.build_notice_kept(commit, &session).unwrap_or(true);
            if kept {
                continue;
            }
            let standing = self.branches_in_build(served, commit, &reasons.branches).await;
            let moved = self.branches_moved(&session, &standing, why).await;
            let mut settling = reasons.restarts.into_iter().filter(|(_, asked)| *asked == build).map(|(item, _)| item);
            // A wait for this build is answered with the notice itself, so the session wakes once.
            let told = match settling.next() {
                Some(item) => {
                    let text = said(build, commit, &standing, why);
                    Arc::clone(self).settle_waiting(answer(&session, item, &text)).await.is_ok()
                }
                None if moved.is_empty() => continue,
                None => matches!(self.told_news_by_fleet(&session, &said(build, commit, &moved, why), ipc::Notice::BuildMoved).await, Ok(true)),
            };
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
            let words = format!("Restarted: {} at {}", build_word(build), short(commit));
            for item in settling {
                let _ = Arc::clone(self).settle_waiting(answer(&session, item, &words)).await;
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

    /// The live sessions of a repository with a reason to be told, by session.
    async fn build_reasons(&self, served: &Served) -> BTreeMap<String, Reasons> {
        let manifest = served.manifest().id().as_str().to_string();
        let mut reasons: BTreeMap<String, Reasons> = BTreeMap::new();
        let mut reads = Vec::new();
        {
            let store = self.store().lock().await;
            let live = |id: &str| {
                store
                    .session(id)
                    .ok()
                    .flatten()
                    .is_some_and(|one| one.state != SessionState::Ended)
            };
            for row in store.standing_of_kind("branch", &manifest).unwrap_or_default() {
                if row.holder.kind == HolderKind::Session
                    && row.state == AttachmentState::Standing
                    && live(&row.holder.id)
                    && crate::slot_holders::pool_allows_branch(&store, &manifest, served.root(), &row.holder.id, &row.target)
                {
                    reasons.entry(row.holder.id).or_default().branches.insert(row.target);
                }
            }
            let search = SessionSearch {
                manifest_id: Some(&manifest),
                state: Some(SessionState::Live),
                text: None,
            };
            for session in store.find_sessions(&search).unwrap_or_default() {
                if let Ok(taken) = self.ledger_reads(&store, &session) {
                    reads.push((session, taken));
                }
            }
        }
        for (session, taken) in reads {
            for item in self.ledger_waiting(&session, taken) {
                let Some(act) = item.act.filter(|act| act.kind == WaitingActKind::Run) else {
                    continue;
                };
                if let Some((asked, _)) = restart_asked(&act.target) {
                    reasons.entry(session.id.clone()).or_default().restarts.push((item.id, asked));
                }
            }
        }
        reasons
    }
}
