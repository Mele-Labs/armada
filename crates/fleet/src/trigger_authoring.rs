//! Reading, saving and removing Triggers for a repository. `docs/concepts/trigger.md`.
//!
//! **Nothing is held, so nothing goes stale.** A workflow is catalogued and
//! read again after a save; a Trigger's files are read from where they are on
//! every call, which is what freezing a Job does too. A save answers with what
//! runs after it, read the same way.
//!
//! **A repository's file is written in the checkout and read from the base
//! branch**, so a save there answers `waits_for_main` until the base holds it.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{TriggerCatalogue, TriggerWritten};
use core_model::{StepId, TriggerResolution, TriggerSource, TriggerWhen};
use ipc::{WireError, WireValue};

use crate::daemon::Fleet;
use crate::repositories::{Served, TriggerNotRemoved, TriggerNotSaved};

const TRIGGER_UNFIT: &str = "fleet.trigger_unfit";
const TRIGGER_NOT_A_NAME: &str = "fleet.trigger_name_not_a_name";
const TRIGGER_EXISTS: &str = "fleet.trigger_exists";
const TRIGGER_UNWRITABLE: &str = "fleet.trigger_unwritable";
const NO_SUCH_TRIGGER: &str = "fleet.no_such_trigger";

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
    fn trigger_files(&self, served: &Served) -> Vec<TriggerWritten> {
        self.locating()
            .triggers(Path::new(served.root()), served.manifest().base())
    }

    fn triggers_in_force(
        &self,
        served: &Served,
    ) -> (Vec<TriggerWritten>, config::ResolvedTriggers) {
        let written = self.trigger_files(served);
        let resolved = TriggerCatalogue::of(written.clone()).resolve(served.manifest());
        (written, resolved)
    }

    pub(crate) fn list_triggers_of(&self, served: &Served) -> ipc::TriggerList {
        crate::trigger_list(&self.triggers_in_force(served).1)
    }

    pub(crate) fn get_trigger_of(
        &self,
        served: &Served,
        when: ipc::TriggerMoment,
        step: Option<ipc::StepId>,
        name: &str,
        level: Option<ipc::TriggerLevel>,
    ) -> Result<ipc::TriggerDefinition, Refusal> {
        let resolved = self.triggers_in_force(served).1;
        let step = step.map(|step| StepId::new(step.as_str()));
        let level = level.map(|level| match level {
            ipc::TriggerLevel::Armada => TriggerSource::Armada,
            ipc::TriggerLevel::Repository => TriggerSource::Repository,
            ipc::TriggerLevel::Machine => TriggerSource::Machine,
        });
        crate::trigger_definition(&resolved, when.into(), step.as_ref(), name, level).ok_or_else(
            || {
                Refusal::Unacceptable(
                    WireError::raised(
                        NO_SUCH_TRIGGER,
                        format!("no Trigger `{name}` is held at that moment and step in this repository"),
                        self.run_id(),
                    )
                    .with_field("name", WireValue::Str(name.to_string())),
                )
            },
        )
    }

    /// Check, write, and say what runs after.
    pub(crate) fn save_trigger_file(
        &self,
        asked: ipc::SaveTrigger,
        served: &Served,
    ) -> Result<ipc::TriggerSaved, Refusal> {
        let saved = self
            .locating()
            .save_trigger(Path::new(served.root()), served.manifest(), &asked)
            .map_err(|why| self.trigger_not_saved(why))?;
        let (written, resolved) = self.triggers_in_force(served);
        let trigger = saved.trigger.trigger();
        let identity = trigger.identity();
        let runs = resolved
            .triggers()
            .iter()
            .find(|one| one.trigger().identity() == identity);
        let waits_for_main = asked.scope == ipc::TriggerScope::Repository
            && !written.iter().any(|one| {
                one.source() == TriggerSource::Repository
                    && one.text().trim() == asked.definition.trim()
            });
        Ok(ipc::TriggerSaved {
            name: trigger.name().to_string(),
            when: trigger.when().into(),
            step: trigger.step().map(ipc::StepId::from),
            scope: asked.scope,
            file: saved.file,
            replaced: saved.replaced,
            runs_from: runs.map(|one| one.source().into()),
            waits_for_main,
            skipped: match saved.trigger.resolution() {
                TriggerResolution::Skipped(why) => Some(why.into()),
                _ => None,
            },
        })
    }

    pub(crate) fn remove_trigger_file(
        &self,
        asked: ipc::RemoveTrigger,
        served: &Served,
    ) -> Result<ipc::TriggerRemoved, Refusal> {
        let file = self
            .locating()
            .remove_trigger(Path::new(served.root()), &asked)
            .map_err(|why| match why {
                TriggerNotRemoved::NotHere => Refusal::Unacceptable(
                    WireError::raised(
                        NO_SUCH_TRIGGER,
                        format!("no Trigger `{}` is saved in that scope", asked.name),
                        self.run_id(),
                    )
                    .with_field("name", WireValue::Str(asked.name.clone())),
                ),
                TriggerNotRemoved::Unwritable { file, cause } => Refusal::Fault(
                    WireError::raised(
                        TRIGGER_UNWRITABLE,
                        format!("{file} could not be removed: {cause}"),
                        self.run_id(),
                    )
                    .with_field("file", WireValue::Str(file)),
                ),
            })?;
        let resolved = self.triggers_in_force(served).1;
        let when: TriggerWhen = asked.when.into();
        let step = asked.step.as_ref().map(|step| StepId::new(step.as_str()));
        let left = resolved.triggers().iter().find(|one| {
            let trigger = one.trigger();
            trigger.when() == when
                && trigger.step() == step.as_ref()
                && trigger.name() == asked.name
        });
        Ok(ipc::TriggerRemoved {
            scope: asked.scope,
            file,
            runs_from: left.map(|one| one.source().into()),
            waits_for_main: asked.scope == ipc::TriggerScope::Repository
                && left.is_some_and(|one| one.source() == TriggerSource::Repository),
        })
    }

    fn trigger_not_saved(&self, why: TriggerNotSaved) -> Refusal {
        match why {
            TriggerNotSaved::Unfit { why } => Refusal::Unacceptable(
                WireError::raised(TRIGGER_UNFIT, why.clone(), self.run_id())
                    .with_field("reason", WireValue::Str(why)),
            ),
            TriggerNotSaved::NotAName { name } => Refusal::Unacceptable(
                WireError::raised(
                    TRIGGER_NOT_A_NAME,
                    format!(
                        "`{name}` cannot be part of a file's name: a Trigger's name and step \
                         here are letters, digits, `-` and `_`"
                    ),
                    self.run_id(),
                )
                .with_field("name", WireValue::Str(name)),
            ),
            TriggerNotSaved::Exists { name, file } => Refusal::Unacceptable(
                WireError::raised(
                    TRIGGER_EXISTS,
                    format!(
                        "{file} already defines `{name}` and nothing was written; save again \
                         with overwrite to replace it"
                    ),
                    self.run_id(),
                )
                .with_field("name", WireValue::Str(name))
                .with_field("file", WireValue::Str(file)),
            ),
            TriggerNotSaved::Unwritable { file, cause } => Refusal::Fault(
                WireError::raised(
                    TRIGGER_UNWRITABLE,
                    format!("{file} could not be written: {cause}"),
                    self.run_id(),
                )
                .with_field("file", WireValue::Str(file)),
            ),
        }
    }
}
