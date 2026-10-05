//! A step's tuning at the approval press, read against the step it names
//! (23.20). **Pure**, for `crate::approving`'s reason: refused or answered
//! before anything is kept.

use std::collections::BTreeMap;
use std::fmt;

use core_model::{FrozenWorkflow, ModelName, StepId, StepTuning, MANIFEST_CHECK};

/// Why a step's tuning was refused, **and nothing was kept**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Untunable {
    /// Tuning names a step the workflow does not have.
    NoStepToTune {
        step: String,
    },
    /// One step tuned twice in one body.
    TunedTwice {
        step: String,
    },
    BlankStepModel {
        step: String,
    },
    /// `judges: 0`: a panel of nobody reads as judged and asks nothing.
    PanelOfNobody {
        step: String,
    },
    /// `judges` on a step that asks the Judge nothing, or whose Judge a
    /// gate box took away.
    NoPanelToSize {
        step: String,
    },
    /// `checks_off` names a Check the step does not run.
    NoSuchCheck {
        step: String,
        check: String,
    },
    /// `checks_off` names one of Fleet's own looks, which is what the step is
    /// rather than a gate on it.
    CheckIsTheStep {
        step: String,
        check: String,
    },
}

impl fmt::Display for Untunable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Untunable::NoStepToTune { step } => {
                write!(out, "the workflow has no step `{step}` to tune")
            }
            Untunable::TunedTwice { step } => write!(out, "step `{step}` is tuned twice"),
            Untunable::BlankStepModel { step } => {
                write!(out, "step `{step}` names a blank model")
            }
            Untunable::PanelOfNobody { step } => write!(
                out,
                "a panel of no Judges on step `{step}` would read as judged and ask nothing"
            ),
            Untunable::NoPanelToSize { step } => write!(
                out,
                "step `{step}` asks the Judge nothing, so there is no panel to size"
            ),
            Untunable::NoSuchCheck { step, check } => {
                write!(out, "step `{step}` runs no Check `{check}` to turn off")
            }
            Untunable::CheckIsTheStep { step, check } => write!(
                out,
                "`{check}` is what step `{step}` is, not a gate on it, so it cannot be turned off"
            ),
        }
    }
}

/// Each step as a person tuned it, after its gate: a Check a gate box already
/// dropped is not there to turn off, and a Judge it took away has no panel.
pub(crate) fn tuned(
    workflow: FrozenWorkflow,
    tuning: &[ipc::StepTuning],
) -> Result<FrozenWorkflow, Untunable> {
    let mut by_step: BTreeMap<StepId, StepTuning> = BTreeMap::new();
    for sent in tuning {
        let named = sent.step_id.to_domain();
        let step = || sent.step_id.as_str().to_string();
        let Some(declared) = workflow.step(&named) else {
            return Err(Untunable::NoStepToTune { step: step() });
        };
        if by_step.contains_key(&named) {
            return Err(Untunable::TunedTwice { step: step() });
        }
        let model = match &sent.model {
            Some(named) => Some(
                ModelName::new(named.trim())
                    .map_err(|_| Untunable::BlankStepModel { step: step() })?,
            ),
            None => None,
        };
        match sent.judges {
            Some(0) => return Err(Untunable::PanelOfNobody { step: step() }),
            Some(_) if !declared.asks_the_judge() => {
                return Err(Untunable::NoPanelToSize { step: step() })
            }
            _ => {}
        }
        for off in &sent.checks_off {
            let check = || off.clone();
            match declared.checks().iter().find(|held| held.label() == off) {
                None => {
                    return Err(Untunable::NoSuchCheck {
                        step: step(),
                        check: check(),
                    })
                }
                Some(held) if held.kind() != MANIFEST_CHECK => {
                    return Err(Untunable::CheckIsTheStep {
                        step: step(),
                        check: check(),
                    })
                }
                Some(_) => {}
            }
        }
        let context = sent
            .context
            .as_deref()
            .map(str::trim)
            .filter(|words| !words.is_empty())
            .map(str::to_string);
        by_step.insert(
            named,
            StepTuning {
                model,
                effort: sent.effort.map(ipc::Effort::domain),
                context,
                judges: sent.judges,
                checks_off: sent.checks_off.clone(),
            },
        );
    }
    Ok(workflow.regated(|step| match by_step.get(step.id()) {
        Some(tuning) => step.tuned_by_person(tuning),
        None => step,
    }))
}
