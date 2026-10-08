//! Triggers as the wire carries them. **The redaction step for a Trigger**: the
//! file's path and text cross because an editor needs them, and nothing a
//! Command printed does. `docs/concepts/trigger.md`.

use config::{ResolvedTrigger, ResolvedTriggers};
use core_model::{
    FrozenTrigger, StepId, TriggerFiring, TriggerResolution, TriggerRuns, TriggerSource,
    TriggerWhen,
};

fn file(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

fn runs(one: &ResolvedTrigger) -> ipc::TriggerRuns {
    match one.trigger().runs() {
        TriggerRuns::Command(name) => ipc::TriggerRuns::Command { name: name.clone() },
        TriggerRuns::Skill(name) => ipc::TriggerRuns::Skill { name: name.clone() },
    }
}

fn summary(one: &ResolvedTrigger) -> ipc::TriggerSummary {
    let trigger = one.trigger();
    ipc::TriggerSummary {
        name: trigger.name().to_string(),
        when: trigger.when().into(),
        workflow: trigger.workflow().map(ipc::WorkflowId::from),
        step: trigger.step().map(ipc::StepId::from),
        runs: runs(one),
        block: trigger.on_failure().block,
        repair: trigger.on_failure().repair,
        level: one.source().into(),
        file: file(one.path()),
        skipped: match one.resolution() {
            TriggerResolution::Skipped(why) => Some(why.into()),
            TriggerResolution::Command { .. } | TriggerResolution::Skill { .. } => None,
        },
        overrides: one
            .replaced()
            .iter()
            .map(|copy| ipc::OverriddenTrigger {
                level: copy.source.into(),
                file: file(&copy.path),
            })
            .collect(),
    }
}

/// What a repository runs, and every file set aside with why.
pub fn trigger_list(resolved: &ResolvedTriggers) -> ipc::TriggerList {
    ipc::TriggerList {
        triggers: resolved.triggers().iter().map(summary).collect(),
        left_out: resolved
            .left_out()
            .iter()
            .map(|left| ipc::LeftOutTrigger {
                level: left.source().into(),
                file: file(left.path()),
                said: left.to_string(),
            })
            .collect(),
    }
}

/// One definition: the copy that runs, or, with `level`, one it replaced.
pub fn trigger_definition(
    resolved: &ResolvedTriggers,
    when: TriggerWhen,
    step: Option<&StepId>,
    name: &str,
    level: Option<TriggerSource>,
) -> Option<ipc::TriggerDefinition> {
    let one = resolved.triggers().iter().find(|one| {
        let trigger = one.trigger();
        trigger.when() == when && trigger.step() == step && trigger.name() == name
    })?;
    let (source, path, text, overridden_by) = match level {
        None => (one.source(), one.path(), one.text(), None),
        Some(level) if level == one.source() => (one.source(), one.path(), one.text(), None),
        Some(level) => {
            let copy = one.replaced().iter().find(|copy| copy.source == level)?;
            (
                copy.source,
                copy.path.as_path(),
                copy.text.as_str(),
                Some(one.source().into()),
            )
        }
    };
    Some(ipc::TriggerDefinition {
        name: name.to_string(),
        when: when.into(),
        step: step.map(ipc::StepId::from),
        level: source.into(),
        file: file(path),
        definition: text.to_string(),
        overridden_by,
    })
}

/// A Job's Triggers: each firing, and each frozen Trigger no firing has reached
/// as `pending`. A step run again fires again, so a frozen one can show twice.
pub fn job_triggers(frozen: &[FrozenTrigger], firings: &[TriggerFiring]) -> Vec<ipc::JobTrigger> {
    let mut out = Vec::new();
    for one in frozen {
        let mine = firings.iter().filter(|firing| {
            firing.name == one.name
                && firing.when == one.when
                && firing.step == one.step
                && firing.source == one.source
        });
        let before = out.len();
        let drone = matches!(one.resolution, TriggerResolution::Skill { .. });
        out.extend(mine.map(|firing| ipc::JobTrigger {
            drone,
            ..ipc::JobTrigger::from(firing)
        }));
        if out.len() == before {
            out.push(ipc::JobTrigger::from(one));
        }
    }
    out
}
