//! What one project's Operator may reach at the door, read off that project's settings at each call: the tools it has
//! switched off, what its level of acts holds until he has said so, and what its calls name outside the project.
//! `crates/fleet/src/on_your_words.rs` places the caller; the rule is here.
//!
//! **A "say"** is his press on one of that Operator's cards or his words delivered to it, and it is open for
//! [`SAY_OPEN_MILLIS`]. It is kept on the seat, in memory, by project.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::WordsReach;
use ipc::{ManifestId, OperatorActs, OperatorSettings};

use super::fleet_cards::FleetCardKind;
use crate::daemon::Fleet;

/// How long a say stays open: the card-settle window.
pub(crate) const SAY_OPEN_MILLIS: i64 = 15 * 60_000;

/// The refusal for a call that names another project.
pub(crate) const OTHER_PROJECT: &str = "fleet.operator_other_project";

/// The acts each tool covers. `file_issues` is a shell act, so only the gate knows it.
const START_JOBS: &[&str] = &["start_another_session", "dispatch_job"];
const END_JOBS: &[&str] = &["close_session"];
const MERGE: &[&str] = &["merge_pull_request_by_number", "enable_auto_merge", "set_session_auto_land"];

/// The Settings page's name for a tool, said in the refusal so he knows where to turn it on.
pub(crate) fn switch_off(tool: &str) -> String {
    format!("is off for this project's Operator: Settings, Operator, {tool}")
}

/// Why a level holds an act until he has said so.
pub(crate) fn said_needed(level: OperatorActs) -> String {
    format!("waits for him: this project's Operator acts at the level {} and he has said nothing in the last 15 minutes. He can say so on a card or in the talk, then call again", level.as_wire())
}

/// The acts `level` holds until a say is open: a start for `ask_before_starting`, every tool act for `only_when_told`.
fn held_by(level: Option<OperatorActs>) -> Vec<&'static str> {
    match level {
        Some(OperatorActs::AskBeforeStarting) => START_JOBS.to_vec(),
        Some(OperatorActs::OnlyWhenTold) => [START_JOBS, END_JOBS, MERGE].concat(),
        _ => Vec::new(),
    }
}

/// `reach` narrowed to what `settings` allow, with `said` for whether a say is open.
fn narrowed(reach: WordsReach, settings: &OperatorSettings, said: bool) -> WordsReach {
    let tools = settings.tools;
    let mut reach = reach;
    for (on, acts, name) in [(tools.start_jobs, START_JOBS, "Start Jobs"), (tools.merge, MERGE, "Merge pull requests"), (tools.end_jobs, END_JOBS, "End Jobs")] {
        if !on {
            reach = reach.without(acts, &switch_off(name));
        }
    }
    match said {
        true => reach,
        false => match settings.acts {
            level @ Some(OperatorActs::AskBeforeStarting | OperatorActs::OnlyWhenTold) => reach.until_said(&held_by(level), &said_needed(level.unwrap_or(OperatorActs::OnlyWhenTold))),
            _ => reach,
        },
    }
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
    /// He said something to `project`'s Operator now.
    pub(crate) fn operator_said(&self, project: &str) {
        self.operator_seat().said_at(project, self.now().epoch_millis().unwrap_or(0));
    }

    /// Whether he has said something to `project`'s Operator in the last [`SAY_OPEN_MILLIS`].
    pub(crate) fn operator_say_open(&self, project: &str) -> bool {
        let now = self.now().epoch_millis().unwrap_or(0);
        self.operator_seat().last_said(project).is_some_and(|at| now - at < SAY_OPEN_MILLIS)
    }

    /// The Operator session `session`'s reach, `reach` narrowed by its project's tools and level. **Read at each call**, so a
    /// switch he turned reaches the next one.
    pub(crate) async fn operator_reach_narrowed(&self, session: &str, reach: WordsReach) -> WordsReach {
        let Some(project) = self.operator_project_of(session).await else { return reach };
        let settings = self.operator_settings_of(&project).await;
        narrowed(reach, &settings, self.operator_say_open(&project))
    }

    /// Whether `from`'s Operator may act on repository `manifest`: it is in a project that holds it, or no project holds it.
    /// `Some(project)` is a project that holds it where `from` is not one of them.
    pub(crate) fn operator_repository_outside(&self, from: &str, manifest: &str) -> Option<String> {
        let manifest = self.repository_id_of(manifest).unwrap_or_else(|| manifest.to_string());
        let held = self.projects_held();
        let holding: Vec<String> = held.holding(&ManifestId::carried(manifest).to_domain()).into_iter().map(|one| one.id().as_str().to_string()).collect();
        match holding.iter().any(|one| one == from) {
            true => None,
            false => holding.into_iter().next(),
        }
    }

    /// The project outside `from` that Job `job` is in, where it is in one.
    pub(crate) async fn operator_job_outside(&self, from: &str, job: &str) -> Option<String> {
        self.project_of_job(job).await.filter(|project| project != from)
    }

    /// Hold a call of `from`'s Operator that names `to`: refused with [`OTHER_PROJECT`] unless he let it through, and a card
    /// raised to him. `None` where a grant stands.
    pub(crate) async fn operator_held_across(&self, session: &str, from: &str, to: &str, act: &str, words: &str) -> Option<String> {
        if self.operator_seat().grant_open(from, to, self.now().epoch_millis().unwrap_or(0)) {
            return None;
        }
        let kind = FleetCardKind::OtherProject { session: session.to_string(), from_project: from.to_string(), to_project: to.to_string(), act: act.to_string(), words: words.to_string() };
        self.fleet_card_raised(kind).await;
        let (from_name, to_name) = (self.operator_project_name(from), self.operator_project_name(to));
        Some(format!(
            "{OTHER_PROJECT}: held: he is asked. `{act}` names something in {to_name}, and this Operator is {from_name}'s. Nothing was done. \
             He is asked on a card, and you are told what he answers."
        ))
    }
}
