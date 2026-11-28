//! A Job's kept looks set against its head commit. `crate::approved_looks`.
//!
//! **The Job's commit is built by the walk build queue** (an ask that writes no row, `walk_builds::Answer`)
//! and each walk is played with `--against` the folder he approved. The binary's `report.json` is read
//! with `ipc::decode`. Three answers: the screen is as he approved, it drifted, or it could not be compared.
//!
//! **Fleet only tells the Job.** A drift is the Job's to fix, or to show again for a new approval.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::LookReport;
use store::KeptLook;

use crate::approved_looks::{played, root_of, HOLD_MS};
use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::repositories::Served;
use crate::walk_builds::Ask;

/// How long before its pull request opens a Job waits for its commit to build: the mod gives up on Fleet
/// at five minutes, and a build that goes on is found kept by the Job's next try.
pub(crate) const BUILD_WAIT: Duration = Duration::from_secs(150);

/// How long a push waits for the build of what was pushed: nobody is waiting on it.
const PUSH_BUILD_WAIT: Duration = Duration::from_secs(40 * 60);

/// The pull request notice key that a push was checked, once a commit and Job.
const CHECKED: &str = "look_checked";

/// What one kept look came to against the Job's commit.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Compared {
    /// The screen is as he approved it.
    Same,
    /// It drifted: what the Job is told, with the paths of the two pictures of each step.
    Drifted(String),
    /// It could not be compared: why.
    Unrun(String),
}

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

/// A report's path as the Job can open it: one written relative to `--out` is made whole.
fn path_in(out: &Path, written: &str) -> String {
    let path = Path::new(written);
    if path.is_absolute() { written.to_string() } else { out.join(path).display().to_string() }
}

/// What the Job is told of a report that drifted.
fn drift_told(look: &KeptLook, commit: &str, report: &LookReport, out: &Path) -> String {
    let mut text = format!("{} drifted from the look he approved at {}, in the build of {}:", look.url, short(&look.commit), short(commit));
    for step in report.steps.iter().filter(|step| step.drifted || step.missing) {
        text.push_str(&format!("\n  step {} ({}): ", step.step, step.file));
        if step.missing {
            text.push_str("no picture to set against the one he approved");
        } else {
            text.push_str(&format!("{:.0}% of the picture differs, mean {:.3}", step.cells * 100.0, step.mean));
        }
        if let Some(both) = &step.side_by_side {
            text.push_str(&format!("\n    side by side: {}", path_in(out, both)));
        }
        if let Some(difference) = &step.difference {
            text.push_str(&format!("\n    difference: {}", path_in(out, difference)));
        }
    }
    text.push_str("\nFix the screen to look as he approved it, or show the walk again with show_window for a new approval.");
    text
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
    /// Every look the Job kept, against `commit` (the Job's head where `None`). **Nothing runs for a Job
    /// that kept none.**
    pub(crate) async fn looks_compared(&self, session: &str, commit: Option<&str>, wait: Duration) -> Vec<(KeptLook, Compared)> {
        let kept = self.store().lock().await.kept_looks(session).unwrap_or_default();
        if kept.is_empty() {
            return Vec::new();
        }
        let (place, head) = match self.walk_place(session).await {
            Ok(found) => found,
            Err(why) => return kept.into_iter().map(|look| (look, Compared::Unrun(why.clone()))).collect(),
        };
        let commit = commit.map(str::to_string).unwrap_or(head);
        let mut all = Vec::new();
        for look in kept {
            let answer = self.look_compared(session, &look, &place, &commit, wait).await;
            all.push((look, answer));
        }
        all
    }

    async fn look_compared(&self, session: &str, look: &KeptLook, place: &crate::walk_builds::Place, commit: &str, wait: Duration) -> Compared {
        let (sender, mut receiver) = tokio::sync::watch::channel::<Option<Result<PathBuf, String>>>(None);
        self.walk_builds().asked(Ask {
            session: session.to_string(),
            url: look.url.clone(),
            commit: commit.to_string(),
            place: place.clone(),
            answer: Some(Arc::new(sender)),
        });
        let built = match tokio::time::timeout(wait, receiver.wait_for(Option::is_some)).await {
            Ok(Ok(built)) => built.clone().expect("waited for it"),
            Ok(Err(_)) => Err("the build was dropped".to_string()),
            Err(_) => Err(format!("the build of {} is still going: run it again once it has built", short(commit))),
        };
        let binary = match built {
            Ok(binary) => binary,
            Err(why) => return Compared::Unrun(format!("{} was not compared with the look he approved: {why}", look.url)),
        };
        let out = root_of(&self.host().home, session).join(format!("{}.against", look.walk));
        let _ = std::fs::remove_dir_all(&out);
        if let Err(why) = std::fs::create_dir_all(&out) {
            return Compared::Unrun(format!("{} could not be made: {why}", out.display()));
        }
        let args: Vec<String> = [
            "--walk", &look.walk, "--check", "--shots", &out.join("shots").display().to_string(), "--against", &look.folder, "--out",
            &out.display().to_string(), "--hold", HOLD_MS,
        ]
        .map(String::from)
        .to_vec();
        let played = match played(&binary, &args).await {
            Ok(played) => played,
            Err(why) => return Compared::Unrun(format!("{} was not compared with the look he approved: {why}", look.url)),
        };
        let code = match played.code {
            Some(code @ (0 | 1)) => code,
            code => {
                return Compared::Unrun(format!(
                    "the walk {} did not play at {} (exit {code:?}), so {} was not compared with the look he approved: {}\nFix it, or show the walk again for a new approval.",
                    look.walk,
                    short(commit),
                    look.url,
                    played.stderr
                ));
            }
        };
        let report = std::fs::read(out.join("report.json")).ok().and_then(|bytes| ipc::decode::<LookReport>("a look report", &bytes).ok());
        match (report, code) {
            (Some(report), _) if report.drifted => Compared::Drifted(drift_told(look, commit, &report, &out)),
            (Some(_), _) | (None, 0) => Compared::Same,
            (None, _) => Compared::Drifted(format!(
                "{} drifted from the look he approved at {}, and the walk wrote no report of where.\n\
                 Fix the screen to look as he approved it, or show the walk again with show_window for a new approval.",
                look.url,
                short(&look.commit)
            )),
        }
    }

    /// **A push to a pull request that is open**, read at its head: each look its owner kept is checked
    /// there, once a commit, and a drift is told to the Job. Fleet writes nothing to the forge.
    pub(crate) async fn pull_looks_told(self: &Arc<Self>, served: &Served, number: u64, branch: &str, head: &str) {
        for Owner::Session(id) in self.owners_of(served, number, branch, false).await {
            let recipient = Owner::Session(id.clone()).key();
            let first = {
                let mut store = self.store().lock().await;
                if store.kept_looks(&id).map_or(true, |kept| kept.is_empty()) {
                    continue;
                }
                store.keep_pull_notice(served.root(), number, head, CHECKED, &recipient, &self.now()).unwrap_or(false)
            };
            if !first {
                continue;
            }
            let (fleet, head) = (Arc::clone(self), head.to_string());
            tokio::spawn(async move {
                for (_, answer) in fleet.looks_compared(&id, Some(&head), PUSH_BUILD_WAIT).await {
                    if let Compared::Drifted(text) = answer {
                        let text = format!("#{number} was pushed at {}. {text}", short(&head));
                        let _ = fleet.told_by_fleet(&id, &text, ipc::Notice::WalkFailed).await;
                    }
                }
            });
        }
    }
}
