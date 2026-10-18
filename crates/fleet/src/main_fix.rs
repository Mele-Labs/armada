//! What Fleet keeps of main's red: who has taken it, the brief a fixer is handed, and the end of
//! every take once main is green. `docs/concepts/fleet.md`, *When main goes red*.
//!
//! **One record, `main_ci_fixes`**, which is what `hub.fixing` and a take's end are read from, and
//! what says once per red per fixer. A Session takes a red through `crate::taking_red`, and the
//! claim is made in the central store first (`crate::red_claim`), so a Fleet on another machine
//! refuses a second fix.

use adapter_traits::{AgentHarness, Delivery, Holder, Vcs, WorkProduct};
use store::{MainCi, MainFix, TakenBy, TakenHow};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::main_ci::{MainChange, MainChanged};
use crate::repositories::Served;

/// The last lines of the failing job's log a Session is handed, beside its address.
const LOG_LINES: usize = 60;

/// Why a red could not be handed to a Session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MainNotFixable {
    /// This Fleet does not serve the repository.
    NotServed,
    /// Main is not red, or Fleet has not read it.
    NotRed,
    /// A newer commit's CI is still running on main, and may already have
    /// fixed it.
    ChecksRunning,
    /// A Session is already working on this red.
    Taken(TakenBy),
    /// Another machine's Session holds this red in the central store.
    /// `held_for` is whole seconds.
    TakenElsewhere { holder: Holder, held_for: i64 },
    /// The central store could not be asked or said no, so the red was not
    /// claimed. A fix nobody else can see is the double fix the claim stops.
    ClaimNotMade(String),
}

impl std::fmt::Display for MainNotFixable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MainNotFixable::NotServed => write!(out, "this Fleet does not serve that repository"),
            MainNotFixable::NotRed => write!(out, "main is not red"),
            MainNotFixable::ChecksRunning => {
                write!(out, "new checks are running on main, and may have fixed it")
            }
            MainNotFixable::Taken(TakenBy::Session(id)) => {
                write!(out, "Job {id} is already working on this red")
            }
            MainNotFixable::TakenElsewhere { holder, held_for } => write!(
                out,
                "{} {} on {} has held this red for {}. Take it over to replace that claim",
                match holder.kind {
                    adapter_traits::HolderKind::Session => "Job",
                },
                holder.id,
                holder.machine,
                aged(*held_for)
            ),
            MainNotFixable::ClaimNotMade(said) => write!(out, "the red was not taken. {said}"),
        }
    }
}

/// A claim's age, to its largest whole unit.
fn aged(seconds: i64) -> String {
    match seconds {
        0..=59 => format!("{seconds} s"),
        60..=3599 => format!("{} min", seconds / 60),
        3600..=86_399 => format!("{} h", seconds / 3600),
        _ => format!("{} d", seconds / 86_400),
    }
}

/// The facts of a red, one line each, as the band shows them.
pub(crate) fn brief_of(main: &MainCi) -> String {
    let mut lines = Vec::new();
    for job in &main.failed {
        lines.push(format!(
            "{} fails on main.",
            job.check.as_deref().unwrap_or(&job.name)
        ));
        lines.extend(job.tests.iter().map(|test| format!("Test: {test}")));
    }
    if let Some(merge) = &main.merge {
        lines.push(format!(
            "Merged in #{}{}.",
            merge.number,
            merge
                .branch
                .as_ref()
                .map_or(String::new(), |branch| format!(" ({branch})"))
        ));
    }
    lines.join("\n")
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

    /// The reading, if main is red there and Fleet serves the repository.
    pub(crate) async fn red_of(&self, root: &str) -> Result<(Served, MainCi), Adrift> {
        let served = self
            .repositories()
            .served()
            .into_iter()
            .find(|served| served.root() == root)
            .ok_or_else(|| refused(root, MainNotFixable::NotServed))?;
        let main = self
            .store()
            .lock()
            .await
            .main_ci(root)
            .map_err(Adrift::Reading)?
            .filter(|main| main.red_at.is_some() && !main.failed.is_empty())
            .ok_or_else(|| refused(root, MainNotFixable::NotRed))?;
        if main.held() {
            return Err(refused(root, MainNotFixable::ChecksRunning));
        }
        Ok((served, main))
    }

    /// The Session working on this red, if one is: the newest take whose Session has not ended.
    pub(crate) async fn taking(&self, main: &MainCi) -> Option<TakenBy> {
        let store = self.store().lock().await;
        self.fixing_of(&store, main)
    }

    /// The brief, and under it the address of the failing job's log and its
    /// last lines as the forge gave them. **Absent where the forge would not
    /// give the text**: the address still goes.
    pub(crate) async fn with_the_log(&self, served: &Served, main: &MainCi, brief: String) -> String {
        let Some(job) = main.failed.first() else {
            return brief;
        };
        let (commit, name) = (main.red_commit().to_string(), job.name.clone());
        let text = async {
            let runs = self
                .forge_asked(served.root(), move |vcs: &V, root: &str| {
                    vcs.ci_runs_on(root, &commit)
                })
                .await?;
            let run = runs.into_iter().find(|run| {
                run.state == adapter_traits::CiState::Failed && run.name.as_written() == name
            })?;
            self.forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.ci_log(root, &run)
            })
            .await
        }
        .await;
        let mut out = brief;
        if let Some(url) = &job.log_url {
            out.push_str(&format!("\n\nLog: {url}"));
        }
        if let Some(text) = text {
            let lines: Vec<&str> = text.as_written().lines().collect();
            let tail = &lines[lines.len().saturating_sub(LOG_LINES)..];
            out.push_str(&format!("\n\n{}", tail.join("\n")));
        }
        out
    }

    /// `by`'s take of this red as a row, where main is red and says what failed.
    fn take_of(&self, main: &MainCi, by: TakenBy, how: TakenHow, forked_from: Option<&str>) -> Option<MainFix> {
        let (red_at, first) = (main.red_at.clone()?, main.failed.first()?);
        Some(MainFix {
            repository: main.repository.clone(),
            red_at,
            by,
            forked_from: forked_from.map(str::to_string),
            how,
            check: first.check.clone().unwrap_or_else(|| first.name.clone()),
            test: first.tests.first().cloned(),
            merge: main.merge.as_ref().map(|merge| merge.number),
            taken_at: self.now(),
            ended_at: None,
            fixed_in: None,
        })
    }

    /// Record that a Session has this red. **`forked_from` is the Session
    /// Fleet forked this one from**, which cannot be handed the red twice.
    pub(crate) async fn recorded_session_take(&self, main: &MainCi, session: &str, how: TakenHow, forked_from: Option<&str>) {
        let Some(take) = self.take_of(main, TakenBy::Session(session.to_string()), how, forked_from) else {
            return;
        };
        let _ = self.store().lock().await.keep_main_fix(&take);
    }

    /// What main's change asks of Fleet: a green ends every take. A red is left to the band's
    /// buttons and to the Session that owns the pull request that turned it.
    pub(crate) async fn main_acted_on(&self, changes: &[MainChanged]) {
        for changed in changes {
            if let MainChange::WentGreen = changed.change {
                self.fixed(&changed.now).await;
            }
        }
    }

    /// Main is green: every take ends, and the Session whose pull request put it
    /// there says it fixed main.
    async fn fixed(&self, main: &MainCi) {
        self.cleared_elsewhere(main).await;
        let commit = main.decided().to_string();
        let merged = self
            .forge_asked(&main.repository, move |vcs: &V, root: &str| {
                vcs.merged_by(root, &commit)
            })
            .await;
        let mut store = self.store().lock().await;
        let fixer = merged.and_then(|merged| {
            let served = self
                .repositories()
                .served()
                .into_iter()
                .find(|served| served.root() == main.repository)?;
            let manifest = served.manifest().id().as_str().to_string();
            let branch = merged
                .branch
                .as_ref()
                .map(|branch| branch.as_written().to_string())
                .unwrap_or_default();
            let owners = crate::slot_holders::sessions_on_pull(
                &store,
                &manifest,
                served.root(),
                merged.number,
                &branch,
                true,
            );
            let takes = store
                .main_fixes_of(&main.repository, main.red_at.as_ref()?)
                .ok()?;
            let fixer = takes
                .iter()
                .find(|take| take.ended_at.is_none() && owners.iter().any(|id| id == take.by.id()))?;
            Some((fixer.by.id().to_string(), merged.number))
        });
        let _ = store.end_main_fixes(
            &main.repository,
            &main.read_at,
            fixer.as_ref().map(|(session, number)| (session.as_str(), *number)),
        );
    }

    /// The Session working on main's red, for the hub and the guards: the
    /// newest take that has not ended, whose Session is still running.
    pub(crate) fn fixing_of(&self, store: &store::Store, main: &MainCi) -> Option<TakenBy> {
        let red_at = main.red_at.as_ref()?;
        store
            .main_fixes_of(&main.repository, red_at)
            .ok()?
            .into_iter()
            .filter(|take| take.ended_at.is_none())
            .map(|take| take.by)
            .find(|by| match by {
                TakenBy::Session(id) => store
                    .session(id)
                    .ok()
                    .flatten()
                    .is_some_and(|found| found.ended_at.is_none()),
            })
    }

}

pub(crate) fn refused(root: &str, why: MainNotFixable) -> Adrift {
    Adrift::MainNotFixable {
        root: root.to_string(),
        why,
    }
}
