//! The dials settings.json moves while Fleet runs, held together so one read
//! takes them at one instant.
//!
//! **Shipped, overlaid by saved** — `crate::limits`' rule for every live dial.
//! The composition root hands in what ships as [`Fittings`](crate::daemon::Fittings);
//! a key the file holds replaces its dial at assembly and at every save or
//! hand edit, and a key an environment variable overrides never does, since
//! the variable is already in what was handed in. A step's or a repository's
//! own value still wins where it always did: [`crate::allowance`] and
//! [`crate::silence`] fold those over whatever this holds.

use std::num::NonZeroU8;
use std::time::Duration;

use adapter_traits::{Effort, Model};
use config::settings::{self as keys, Resolved};

use crate::allowance::{Allowance, Micros};
use crate::asked_run::AskedRuns;
use crate::budget::CommandBudget;
use crate::converging::StepNorms;
use crate::fixing::Fixes;
use crate::gate::CheckBudget;
use crate::headroom::Polling;
use crate::holding::Reclaiming;
use crate::judging::JudgeBudget;
use crate::noticing::Noticing;
use crate::permitting::UnansweredAskLimit;
use crate::silence::Liveness;

/// Every live dial below the four limits, which `crate::limits` holds under
/// the roster lock instead.
#[derive(Clone, Debug)]
pub(crate) struct Tuning {
    pub(crate) budget: CheckBudget,
    pub(crate) norms: StepNorms,
    pub(crate) liveness: Liveness,
    pub(crate) asked_runs: AskedRuns,
    pub(crate) fixes: Fixes,
    pub(crate) judge_budget: JudgeBudget,
    pub(crate) proposer_budget: JudgeBudget,
    pub(crate) command_budget: CommandBudget,
    pub(crate) unanswered_ask_limit: UnansweredAskLimit,
    pub(crate) allowance: Allowance,
    pub(crate) polling: Polling,
    pub(crate) noticing: Noticing,
    pub(crate) reclaiming: Reclaiming,
    pub(crate) run_log_retention: Duration,
    pub(crate) helm_authority: crate::helm::Authority,
    pub(crate) helm_session_retention: Duration,
    pub(crate) judge_model: Model,
    pub(crate) second_opinion_model: Model,
    pub(crate) proposer_model: Model,
    pub(crate) retro_model: Model,
    /// This machine's tier of the pull request mode, beneath the workflow's and
    /// the repository's — `crate::pr_mode`.
    pub(crate) draft_pull_requests: bool,
    /// What a step's Drone thinks at where neither the step nor the approval
    /// said. `None` is the harness picking.
    pub(crate) default_effort: Option<Effort>,
    pub(crate) clearing_sends: u32,
    pub(crate) judge_read_turns: NonZeroU8,
    pub(crate) standing_rules_bytes: usize,
    /// The words of every prompt a person may override, read when one is built.
    pub(crate) prompts: crate::prompts::Prompts,
}

const DAY: u64 = 24 * 60 * 60;

impl Tuning {
    /// These dials with every value the file chose put over them.
    pub(crate) fn overlaid_by(&self, settings: &Resolved) -> Tuning {
        let whole = |key, shipped: u32| settings.chosen(key).map_or(shipped, |n: i64| u32::try_from(n).unwrap_or(shipped));
        let seconds = |key, shipped: Duration| settings.chosen(key).unwrap_or(shipped);
        let days = |key, shipped: Duration| {
            settings.chosen(key).map_or(shipped, |n: i64| Duration::from_secs(u64::try_from(n).unwrap_or(0) * DAY))
        };
        let model = |name, effort, shipped: &Model| -> Model {
            let named: String = settings.chosen(name).unwrap_or_else(|| shipped.as_str().to_string());
            let thinking = settings.chosen(effort).map_or(shipped.effort(), |word: String| effort_of(&word));
            Model::named_at(&named, thinking).unwrap_or_else(|_| shipped.clone())
        };
        let budget = match settings.chosen(keys::CHECK_SECONDS) {
            Some(chosen) => CheckBudget::of(chosen),
            None => self.budget,
        };
        Tuning {
            budget,
            norms: StepNorms::of(
                whole(keys::TOOL_CALLS_PER_STEP, self.norms.calls()),
                seconds(keys::STEP_WALL_CLOCK_SECONDS, self.norms.wall_clock()),
                seconds(keys::STEP_GRACE_SECONDS, self.norms.report_grace()),
            ),
            liveness: Liveness::of(
                seconds(keys::DRONE_QUIET_AFTER_SECONDS, self.liveness.quiet_after()),
                whole(keys::DRONE_POKE_LIMIT, self.liveness.pokes()),
            ),
            asked_runs: AskedRuns::of(whole(keys::ASKED_RUNS_PER_STEP, self.asked_runs.allowed())),
            fixes: Fixes::of(whole(keys::FIXES_PER_STEP, self.fixes.allowed())),
            judge_budget: JudgeBudget::of(seconds(keys::JUDGE_SECONDS, self.judge_budget.duration())),
            proposer_budget: JudgeBudget::of(seconds(keys::PROPOSER_SECONDS, self.proposer_budget.duration())),
            command_budget: CommandBudget::of(seconds(keys::COMMAND_SECONDS, self.command_budget.duration())),
            unanswered_ask_limit: UnansweredAskLimit::of(seconds(
                keys::UNANSWERED_ASK_SECONDS,
                self.unanswered_ask_limit.duration(),
            )),
            allowance: Allowance::of(
                settings
                    .chosen(keys::COST_CAP_DOLLARS_PER_JOB)
                    .map_or(self.allowance.cost(), |n| Micros::dollars(u64::try_from(n).unwrap_or(0))),
                settings
                    .chosen(keys::TURN_CAP_PER_JOB)
                    .map_or(self.allowance.turns(), |n| u64::try_from(n).unwrap_or(0)),
            ),
            polling: Polling::every(seconds(keys::RESOURCE_POLL_SECONDS, self.polling.interval())),
            noticing: Noticing::every(seconds(keys::MERGE_NOTICE_SECONDS, self.noticing.interval())),
            reclaiming: Reclaiming::every(seconds(keys::RECLAIM_SWEEP_SECONDS, self.reclaiming.interval())),
            run_log_retention: days(keys::RUN_LOG_DAYS, self.run_log_retention),
            helm_authority: match settings.chosen(keys::HELM_CAN_ACT) {
                Some(true) => crate::helm::Authority::Acting,
                Some(false) => crate::helm::Authority::ReadOnly,
                None => self.helm_authority,
            },
            helm_session_retention: days(keys::HELM_SESSION_DAYS, self.helm_session_retention),
            judge_model: model(keys::MODELS_JUDGE, keys::EFFORT_JUDGE, &self.judge_model),
            second_opinion_model: model(keys::MODELS_SECOND_OPINION, keys::EFFORT_JUDGE, &self.second_opinion_model),
            proposer_model: model(keys::MODELS_PROPOSER, keys::EFFORT_PROPOSER, &self.proposer_model),
            retro_model: Model::named_at(
                &settings.chosen(keys::MODELS_RETRO).unwrap_or_else(|| self.retro_model.as_str().to_string()),
                self.retro_model.effort(),
            )
            .unwrap_or_else(|_| self.retro_model.clone()),
            draft_pull_requests: settings.chosen(keys::DRAFT_PULL_REQUESTS).unwrap_or(self.draft_pull_requests),
            default_effort: settings
                .chosen(keys::EFFORT_DEFAULT)
                .map_or(self.default_effort, |word| effort_of(&word)),
            clearing_sends: whole(keys::CONFLICT_CLEARING_SENDS, self.clearing_sends),
            judge_read_turns: settings
                .chosen(keys::JUDGE_READ_TURNS)
                .and_then(|n| u8::try_from(n).ok())
                .and_then(NonZeroU8::new)
                .unwrap_or(self.judge_read_turns),
            standing_rules_bytes: settings
                .chosen(keys::STANDING_RULES_KIB)
                .map_or(self.standing_rules_bytes, |kib| usize::try_from(kib).unwrap_or(0) * 1024),
            prompts: self.prompts.overlaid_by(settings),
        }
    }
}

/// An effort as the setting spells it, `None` for the harness's own.
pub fn effort_of(word: &str) -> Option<Effort> {
    match word {
        "low" => Some(Effort::Low),
        "medium" => Some(Effort::Medium),
        "high" => Some(Effort::High),
        _ => None,
    }
}

impl<H, V, W> crate::daemon::Fittings<H, V, W> {
    /// The live dials as handed in, and the ones no fitting carries at what
    /// ships — `Fleet::assembled`'s, before the file is put over them.
    pub(crate) fn shipped_tuning(&self) -> Tuning {
        Tuning {
            budget: self.budget,
            norms: self.norms,
            liveness: self.liveness,
            asked_runs: self.asked_runs,
            fixes: self.fixes,
            judge_budget: self.judge_budget,
            proposer_budget: self.proposer_budget,
            command_budget: self.command_budget,
            unanswered_ask_limit: self.unanswered_ask_limit,
            allowance: self.allowance,
            polling: self.polling,
            noticing: self.noticing,
            reclaiming: self.reclaiming,
            run_log_retention: self.run_log_retention,
            helm_authority: self.helm_authority,
            helm_session_retention: self.helm_session_retention,
            judge_model: self.judge_model.clone(),
            second_opinion_model: self.second_opinion_model.clone(),
            proposer_model: self.proposer_model.clone(),
            retro_model: self.retro_model.clone(),
            draft_pull_requests: false,
            default_effort: None,
            clearing_sends: crate::conflict_resolution::CLEARING_SENDS,
            judge_read_turns: crate::judging::JUDGE_READ_TURNS,
            standing_rules_bytes: verification::STANDING_RULES,
            prompts: crate::prompts::Prompts::shipped(),
        }
    }
}
