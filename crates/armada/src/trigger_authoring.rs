//! Saving and removing a Trigger file: checked, then written, in the scope named.
//!
//! **`config::fit_trigger` is the loader's own parse and resolve**, so what is
//! saved is what would load and a refusal carries the loader's sentence. Nothing
//! is created on the way to a refusal.
//!
//! **A repository's file may not name a Command its `armada.yml` lacks.** The
//! loader skips such a Trigger, because the same file is right in the next
//! repository; a repository's own folder is not copied to the next, so a typo
//! there is refused. A machine's is saved, and the answer says it is skipped here.
//!
//! **Found by the identity a file declares, not by its name**, as a workflow's is.

use std::path::{Path, PathBuf};

use config::{Manifest, TriggerWritten};
use core_model::{TriggerIdentity, TriggerResolution};
use fleet::repositories::{SavedTrigger, TriggerNotRemoved, TriggerNotSaved};
use ipc::{RemoveTrigger, SaveTrigger, TriggerScope};

const REPOSITORY: &str = ".armada/triggers";

/// This machine's own Triggers, under Fleet's data directory.
pub(crate) fn machine_folder(machine: &Path) -> PathBuf {
    machine.join("machine").join("triggers")
}

fn folder(root: &Path, machine: &Path, scope: TriggerScope) -> PathBuf {
    match scope {
        TriggerScope::Repository => root.join(REPOSITORY),
        TriggerScope::Machine => machine_folder(machine),
    }
}

/// The `.yml` and `.yaml` files in `dir`, sorted. Absent is none.
pub(crate) fn listed(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yml" | "yaml")
                )
        })
        .collect();
    found.sort();
    found
}

fn holding(dir: &Path, identity: &TriggerIdentity) -> Vec<PathBuf> {
    listed(dir)
        .into_iter()
        .filter(|path| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| config::parse_trigger(path, &text).ok())
                .is_some_and(|trigger| &trigger.identity() == identity)
        })
        .collect()
}

fn safe(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Check `asked` against `manifest`, and write it in its scope.
pub(crate) fn save(
    root: &Path,
    machine: &Path,
    manifest: &Manifest,
    asked: &SaveTrigger,
) -> Result<SavedTrigger, TriggerNotSaved> {
    let dir = folder(root, machine, asked.scope);
    let text = asked.definition.trim();
    let unnamed = dir.join("(definition).yml");
    let fitted =
        config::fit_trigger(&unnamed, text, manifest).map_err(|why| TriggerNotSaved::Unfit {
            why: why.to_string(),
        })?;
    if let (TriggerScope::Repository, TriggerResolution::Skipped(why)) =
        (asked.scope, fitted.resolution())
    {
        return Err(TriggerNotSaved::Unfit {
            why: format!(
                "{why}; a Trigger saved in the repository names a Command its armada.yml declares"
            ),
        });
    }
    let trigger = fitted.trigger();
    let identity = trigger.identity();

    let holders = holding(&dir, &identity);
    let (target, replaced) = match holders.as_slice() {
        [] => {
            let step = trigger.step().map(|step| step.as_str().to_string());
            let parts = [
                Some(identity.when.as_wire().to_string()),
                step,
                Some(identity.name.clone()),
            ];
            let parts: Vec<String> = parts.into_iter().flatten().collect();
            if let Some(bad) = parts.iter().find(|part| !safe(part)) {
                return Err(TriggerNotSaved::NotAName { name: bad.clone() });
            }
            (dir.join(format!("{}.yml", parts.join("--"))), false)
        }
        [only] if asked.overwrite => (only.clone(), true),
        [only, ..] if !asked.overwrite => {
            return Err(TriggerNotSaved::Exists {
                name: identity.name,
                file: only.display().to_string(),
            })
        }
        several => {
            let files: Vec<String> = several.iter().map(|f| f.display().to_string()).collect();
            return Err(TriggerNotSaved::Unfit {
                why: format!(
                    "{} all define `{}`, and a place holds one of each: remove all but one, \
                     then save again",
                    files.join(", "),
                    identity.name
                ),
            });
        }
    };
    if !replaced && target.exists() {
        return Err(TriggerNotSaved::Unfit {
            why: format!(
                "{} exists and does not define `{}`; it is not replaced",
                target.display(),
                identity.name
            ),
        });
    }
    write(&dir, &target, text).map_err(|cause| TriggerNotSaved::Unwritable {
        file: target.display().to_string(),
        cause,
    })?;
    Ok(SavedTrigger {
        trigger: fitted,
        file: target.display().to_string(),
        replaced,
    })
}

/// Delete every file in the scope that defines the identity, and answer the first.
pub(crate) fn remove(
    root: &Path,
    machine: &Path,
    asked: &RemoveTrigger,
) -> Result<String, TriggerNotRemoved> {
    let identity = TriggerIdentity {
        when: asked.when.into(),
        step: asked
            .step
            .as_ref()
            .map(|step| core_model::StepId::new(step.as_str())),
        name: asked.name.clone(),
    };
    let held = holding(&folder(root, machine, asked.scope), &identity);
    let Some(first) = held.first() else {
        return Err(TriggerNotRemoved::NotHere);
    };
    for path in &held {
        std::fs::remove_file(path).map_err(|cause| TriggerNotRemoved::Unwritable {
            file: path.display().to_string(),
            cause,
        })?;
    }
    Ok(first.display().to_string())
}

fn write(dir: &Path, target: &Path, text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    // An extension a Trigger file does not carry, so a read in between never
    // takes the half-written file for one.
    let beside = target.with_extension("saving");
    std::fs::write(&beside, format!("{text}\n"))?;
    std::fs::rename(&beside, target).inspect_err(|_| {
        let _ = std::fs::remove_file(&beside);
    })
}

/// The files of this machine's folder, as a loader reads them.
pub(crate) fn on_machine(machine: &Path) -> Vec<TriggerWritten> {
    listed(&machine_folder(machine))
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            Some(TriggerWritten::on_machine(path, text))
        })
        .collect()
}
