//! The dials settings.json moves while Fleet runs, held together so one read
//! takes them at one instant.
//!
//! **Shipped, overlaid by saved** — `crate::limits`' rule for every live dial.
//! The composition root hands in what ships as [`Fittings`](crate::daemon::Fittings);
//! a key the file holds replaces its dial at assembly and at every save or
//! hand edit, and a key an environment variable overrides never does, since
//! the variable is already in what was handed in.

use std::time::Duration;

use adapter_traits::{Effort, Model};
use config::settings::{self as keys, Resolved};

use crate::budget::CheckBudget;
use crate::headroom::Polling;
use crate::noticing::Noticing;
use crate::judging::JudgeBudget;

/// Every live dial below the four limits, which `crate::limits` holds under
/// the roster lock instead.
#[derive(Clone, Debug)]
pub(crate) struct Tuning {
    pub(crate) judge_budget: JudgeBudget,
    pub(crate) proposer_budget: JudgeBudget,
    pub(crate) budget: CheckBudget,
    pub(crate) polling: Polling,
    pub(crate) noticing: Noticing,
    pub(crate) run_log_retention: Duration,
    pub(crate) helm_authority: crate::helm::Authority,
    /// Whether a Session may follow a workflow: `features.workflows`.
    pub(crate) workflows: bool,
    pub(crate) judge_model: Model,
    pub(crate) proposer_model: Model,
    pub(crate) retro_model: Model,
    pub(crate) standing_rules_bytes: usize,
    /// The words of every prompt a person may override, read when one is built.
    pub(crate) prompts: crate::prompts::Prompts,
}

const DAY: u64 = 24 * 60 * 60;

impl Tuning {
    /// These dials with every value the file chose put over them.
    pub(crate) fn overlaid_by(&self, settings: &Resolved) -> Tuning {
        let seconds = |key, shipped: Duration| settings.chosen(key).unwrap_or(shipped);
        let days = |key, shipped: Duration| {
            settings.chosen(key).map_or(shipped, |n: i64| Duration::from_secs(u64::try_from(n).unwrap_or(0) * DAY))
        };
        let model = |name, effort, shipped: &Model| -> Model {
            let named: String = settings.chosen(name).unwrap_or_else(|| shipped.as_str().to_string());
            let thinking = settings.chosen(effort).map_or(shipped.effort(), |word: String| effort_of(&word));
            Model::named_at(&named, thinking).unwrap_or_else(|_| shipped.clone())
        };
        Tuning {
            judge_budget: JudgeBudget::of(seconds(keys::JUDGE_SECONDS, self.judge_budget.duration())),
            proposer_budget: JudgeBudget::of(seconds(keys::PROPOSER_SECONDS, self.proposer_budget.duration())),
            budget: CheckBudget::of(seconds(keys::CHECK_SECONDS, self.budget.duration())),
            polling: Polling::every(seconds(keys::RESOURCE_POLL_SECONDS, self.polling.interval())),
            noticing: Noticing::every(seconds(keys::MERGE_NOTICE_SECONDS, self.noticing.interval())),
            run_log_retention: days(keys::RUN_LOG_DAYS, self.run_log_retention),
            helm_authority: match settings.chosen(keys::HELM_CAN_ACT) {
                Some(true) => crate::helm::Authority::Acting,
                Some(false) => crate::helm::Authority::ReadOnly,
                None => self.helm_authority,
            },
            workflows: settings.chosen(keys::WORKFLOWS).unwrap_or(self.workflows),
            judge_model: model(keys::MODELS_JUDGE, keys::EFFORT_JUDGE, &self.judge_model),
            proposer_model: model(keys::MODELS_PROPOSER, keys::EFFORT_PROPOSER, &self.proposer_model),
            retro_model: Model::named_at(
                &settings.chosen(keys::MODELS_RETRO).unwrap_or_else(|| self.retro_model.as_str().to_string()),
                self.retro_model.effort(),
            )
            .unwrap_or_else(|_| self.retro_model.clone()),
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
        "xhigh" => Some(Effort::XHigh),
        "max" => Some(Effort::Max),
        _ => None,
    }
}

impl<H, V, W> crate::daemon::Fittings<H, V, W> {
    /// The live dials as handed in, and the ones no fitting carries at what
    /// ships — `Fleet::assembled`'s, before the file is put over them.
    pub(crate) fn shipped_tuning(&self) -> Tuning {
        Tuning {
            judge_budget: self.judge_budget,
            proposer_budget: self.proposer_budget,
            budget: self.budget,
            polling: self.polling,
            noticing: self.noticing,
            run_log_retention: self.run_log_retention,
            helm_authority: self.helm_authority,
            workflows: true,
            judge_model: self.judge_model.clone(),
            proposer_model: self.proposer_model.clone(),
            retro_model: self.retro_model.clone(),
            standing_rules_bytes: verification::STANDING_RULES,
            prompts: crate::prompts::Prompts::shipped(),
        }
    }
}
