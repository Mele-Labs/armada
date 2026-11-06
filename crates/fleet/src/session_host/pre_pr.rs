//! The Checks a repository keeps for the moment a pull request opens, run on a Job's changes when
//! its agent goes to open one. `docs/contracts/configuration.md`, *Where a Check runs*.
//!
//! **Which Checks, and for which paths, is `config::pre_pr_checks`'s** and so `armada covers`'s: a
//! workspace's Check runs when the change reaches that workspace, and not otherwise.
//!
//! **Quiet where it cannot tell**: a Job with no slot, or changes that will not read, answers
//! `passed`. The pull request is then opened as it was before this existed, and CI is the gate.
//!
//! **Not where the slot has uncommitted changes**: the Checks read files on disk, so they would
//! pass on work the pull request will not carry. The agent is told to commit, and no Check runs.

use std::path::Path;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use checks_runner::Attempt;
use ipc::{CheckBeforePr, ManifestId, PrePrCheckRan, PrePrChecked};
use verification::Exit;

use crate::daemon::Fleet;

const PRE_PR_UNNAMED: &str = "fleet.pre_pr_unnamed";

/// How long one pre-PR Check is given before it counts as failed.
const BUDGET: Duration = Duration::from_secs(4 * 60);
/// The last lines of a Check's output the agent is told.
const TAIL_LINES: usize = 40;

/// One Check to run: its key, its command, the directory it runs in and the exit it expects.
pub(crate) struct ToRun {
    pub name: String,
    pub run: String,
    pub dir: std::path::PathBuf,
    pub expect: i64,
}

/// Run each in turn, and say what they came to. **Every Check runs, so a second failure is in the
/// same answer as the first** and the agent fixes both before it asks again.
pub(crate) async fn ran(checks: Vec<ToRun>) -> PrePrChecked {
    let mut ran: Vec<PrePrCheckRan> = Vec::new();
    for check in checks {
        let attempt = checks_runner::run(&check.run, &check.dir, BUDGET).await;
        ran.push(PrePrCheckRan {
            passed: passed(&attempt.exit, check.expect),
            tail: tail(&check.run, &attempt),
            name: check.name,
        });
    }
    PrePrChecked { passed: ran.iter().all(|one| one.passed), says: says(&ran), checks: ran }
}

/// How many dirty paths the agent is told by name.
const DIRTY_LISTED: usize = 10;

/// **The Checks read the slot's files, and the pull request carries its commits**: an agent that
/// formatted and did not commit would pass here and open a pull request with the unformatted code.
/// So a slot with any uncommitted change that git does not ignore (untracked files too, since a new
/// file left out of the commit is left out of the pull request) answers `passed: false` and runs
/// nothing. `None` where the slot is clean or git will not say, so the Checks run as before.
pub(crate) fn commit_first(slot: &Path) -> Option<PrePrChecked> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(slot)
        .args(["--no-optional-locks", "status", "--porcelain", "-z", "--untracked-files=all"])
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut dirty: Vec<String> = Vec::new();
    let mut entries = text.split('\0');
    while let Some(entry) = entries.next() {
        if entry.len() > 3 {
            dirty.push(entry[3..].to_string());
        }
        // A rename or copy names its old path in the entry after.
        if entry.starts_with('R') || entry.starts_with('C') {
            entries.next();
        }
    }
    if dirty.is_empty() {
        return None;
    }
    let mut says = String::from(
        "Not opened: this repository checks a change before its pull request opens, and the Checks read the files \
         on disk while the pull request carries only what is committed. Commit these changes, then run gh pr create again:\n",
    );
    for path in dirty.iter().take(DIRTY_LISTED) {
        says.push_str(&format!("  {path}\n"));
    }
    if dirty.len() > DIRTY_LISTED {
        says.push_str(&format!("  and {} more\n", dirty.len() - DIRTY_LISTED));
    }
    Some(PrePrChecked { passed: false, says, checks: Vec::new() })
}

/// `ran`, once the slot's changes are committed. No Check to run asks nothing of the slot.
pub(crate) async fn ran_in(slot: std::path::PathBuf, checks: Vec<ToRun>) -> PrePrChecked {
    if checks.is_empty() {
        return ran(checks).await;
    }
    let held = tokio::task::spawn_blocking(move || commit_first(&slot)).await.ok().flatten();
    match held {
        Some(answer) => answer,
        None => ran(checks).await,
    }
}

fn passed(exit: &Exit, expect: i64) -> bool {
    matches!(exit, Exit::Code(code) if i64::from(*code) == expect)
}

/// The command and the last lines of what it printed, or why it never printed anything.
fn tail(command: &str, attempt: &Attempt) -> String {
    let printed = format!("{}{}", attempt.output.stdout, attempt.output.stderr);
    let lines: Vec<&str> = printed.lines().collect();
    let kept = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    let ended = match &attempt.exit {
        Exit::Code(code) => format!("exited {code}"),
        Exit::Signalled { signal } => format!("ended by signal {signal}"),
        Exit::TimedOut { after } => format!("timed out after {}s", after.as_secs()),
        Exit::NeverRan(why) => format!("never ran: {why:?}"),
    };
    format!("$ {command}\n{kept}\n({ended})")
}

/// What the agent reads where something failed: empty where all passed.
fn says(ran: &[PrePrCheckRan]) -> String {
    let failed: Vec<&PrePrCheckRan> = ran.iter().filter(|one| !one.passed).collect();
    if failed.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "Not opened: this repository checks a change before its pull request opens, and the Checks below failed. \
         Fix them, commit, and run gh pr create again.\n",
    );
    for one in failed {
        out.push_str(&format!("\n{} failed:\n{}\n", one.name, one.tail));
    }
    out
}

impl Fleet
{
    /// `check_before_pr`: the pre-PR Checks the Job's changes reach, run in its slot. **The Job is
    /// placed by the connection** where Fleet hosts it, else by the body, as `show_window`'s is.
    pub(crate) async fn pre_pr_checked(
        &self,
        caller: Option<api::Caller>,
        asked: CheckBeforePr,
    ) -> Result<PrePrChecked, Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| asked.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(PRE_PR_UNNAMED, "a job whose pull request is to open"))?;
        let (_, hosting) = self.session_and_hosting(&id).await?;
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)))?;
        let nothing = PrePrChecked { passed: true, ..PrePrChecked::default() };
        if hosting.lease_slot.is_none() {
            return Ok(nothing);
        }
        let slot = Self::directory_of(&served, &hosting);
        let base = crate::clashing::base_of(&served);
        let read = slot.clone();
        let changed = tokio::task::spawn_blocking(move || crate::clashing::changed_since_base(Path::new(&read), &base))
            .await;
        let Ok(Ok(changed)) = changed else { return Ok(nothing) };
        let changed: Vec<String> = changed.into_iter().collect();
        let workspaces = served.workspaces();
        let to_run: Vec<ToRun> = config::pre_pr_checks(served.manifest(), &workspaces, &changed)
            .into_iter()
            .filter_map(|(manifest, name)| {
                let check = manifest.check(name)?;
                Some(ToRun {
                    name: match manifest.dir() {
                        "" => name.to_string(),
                        dir => format!("{dir}:{name}"),
                    },
                    run: check.run().to_string(),
                    dir: Path::new(&slot).join(manifest.dir()),
                    expect: check.expect_exit_code(),
                })
            })
            .collect();
        Ok(ran_in(std::path::PathBuf::from(&slot), to_run).await)
    }
}
