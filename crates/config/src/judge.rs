//! `judge_checks` on a step: what the semantic tier is asked, read off a file.
//!
//! # `criteria[]`, not `question`
//!
//! `docs/concepts/judge.md` supersedes the single-`question` shape: one
//! question per step either makes the question broad, which the narrow-question
//! rule forbids, or silently drops conditions. So an entry declares
//! `criteria[]` and each criterion is its own call. **The seven JSON samples
//! under `core-model/domain/workflow-samples/` still carry `question`** and are
//! stale against that decision; nothing here reads one.
//!
//! # What is refused, and why each
//!
//! `prompt_key` names a prompt library that is sited nowhere — the contract
//! says so itself. `source_ref` points into `job.acceptance_criteria[]`, which
//! is frozen at Job creation and not yet joined to a step. `gates_advancement`
//! needs machinery that is not built. Each would be a promise the file makes
//! and the system does not keep.

use core_model::{JudgeCheck, JudgeCriterion, ModelName, OnRefusal};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::roster::{self, Roster};
use crate::yaml::{self, Table};

/// The keys read inside one `judge_checks` entry.
const JUDGE_KEYS: &[&str] = &[
    "enabled",
    "model",
    "panel_size",
    "panel_models",
    "criteria",
];
/// The keys read inside one criterion.
const CRITERION_KEYS: &[&str] = &["criterion_id", "question", "on_refusal"];
/// The legal values of `on_refusal`.
const ON_REFUSAL: &[&str] = &["ask", "refuse"];

/// Every judge check a step declares, in file order.
///
/// A `judge_checks: []` and an absent key are the same empty list, which is the
/// registry's own reading of an absent check and `enabled: false`.
pub(crate) fn checks(
    table: &mut Table<'_>,
    roster: &Roster,
    out: &mut Vec<Refusal>,
) -> Vec<JudgeCheck> {
    table
        .optional("judge_checks")
        .and_then(|value| yaml::list(&table.at("judge_checks"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| check(at, item, roster, out))
                .collect()
        })
        .unwrap_or_default()
}

fn check(at: &str, value: &Value, roster: &Roster, out: &mut Vec<Refusal>) -> Option<JudgeCheck> {
    let mut table = Table::open(at, value, out)?;

    let enabled = table
        .optional("enabled")
        .and_then(|value| yaml::flag(&table.at("enabled"), value, out))
        .unwrap_or(true);
    let model_key = table.at("model");
    // **Refused unless the roster offers it**, where it used to be refused only
    // for being blank. A typo parsed, froze onto the Job and reached the Judge
    // adapter — a gate that cannot rule, asked for after the Drone has run and
    // the work is done, so it costs the step rather than the spawn. It is not
    // the step's `model` and does not fall back to it; `crate::roster::offered`
    // reads both and says why one roster covers them.
    let model = table
        .optional("model")
        .and_then(|value| yaml::text(&model_key, value, out))
        .and_then(|named| roster::offered(&model_key, named, roster, out));
    let panel_size = table
        .optional("panel_size")
        .and_then(|value| yaml::positive(&table.at("panel_size"), value, out))
        .unwrap_or(1);
    // One model per member, each refused unless the roster offers it, for
    // `model`'s reason. **A panel of one model is not a second opinion**, so
    // this is what lets an agreed refusal go back to the Drone — #206.
    let members: Vec<ModelName> = table
        .optional("panel_models")
        .and_then(|value| yaml::list(&table.at("panel_models"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| {
                    yaml::text(at, item, out)
                        .and_then(|named| roster::offered(at, named, roster, out))
                })
                .collect()
        })
        .unwrap_or_default();
    // The list sizes the panel, so a `panel_size` beside it is either the same
    // number twice or two answers to one question.
    if let Some(declared) = table.optional("panel_size").and_then(Value::as_u64) {
        let named = table
            .optional("panel_models")
            .and_then(Value::as_sequence)
            .map(Vec::len);
        if let Some(named) = named.filter(|named| *named as u64 != declared) {
            out.push(Refusal::new(
                table.at("panel_size"),
                Fault::PanelSizeAndModelsDisagree {
                    size: declared,
                    models: named,
                },
            ));
        }
    }
    let criteria: Vec<JudgeCriterion> = table
        .optional("criteria")
        .and_then(|value| yaml::list(&table.at("criteria"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| criterion(at, item, out))
                .collect()
        })
        .unwrap_or_default();
    table.close(JUDGE_KEYS, out);

    // A disabled check reads as no criteria rather than as a check carrying
    // some it will not ask — one representation, so nothing downstream has two
    // ways to be off.
    match enabled {
        true => Some(JudgeCheck::declared(model, panel_size, criteria).with_members(members)),
        false => Some(JudgeCheck::declared(model, panel_size, Vec::new())),
    }
}

fn criterion(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<JudgeCriterion> {
    let mut table = Table::open(at, value, out)?;
    let criterion_id = table
        .required("criterion_id", out)
        .and_then(|value| yaml::text(&table.at("criterion_id"), value, out));
    let question = table
        .required("question", out)
        .and_then(|value| yaml::text(&table.at("question"), value, out));
    // Absent means `Ask`, per `docs/concepts/judge.md`'s asking design: every
    // criterion asks by default, and `refuse` is what a step author opts
    // into where stopping the step outright is the only sane answer.
    let on_refusal = match table
        .optional("on_refusal")
        .and_then(|value| yaml::text(&table.at("on_refusal"), value, out))
    {
        None => OnRefusal::Ask,
        Some(named) => match OnRefusal::from_wire(&named) {
            Some(on_refusal) => on_refusal,
            None => {
                out.push(Refusal::new(
                    table.at("on_refusal"),
                    Fault::NotInTheSchema {
                        value: named,
                        legal: ON_REFUSAL,
                    },
                ));
                OnRefusal::Ask
            }
        },
    };
    table.close(CRITERION_KEYS, out);
    Some(JudgeCriterion {
        criterion_id: core_model::CriterionId::new(criterion_id?),
        question: question?,
        on_refusal,
    })
}
