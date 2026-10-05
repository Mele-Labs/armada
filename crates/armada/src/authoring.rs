//! Saving a workflow definition: checked, then written, in the scope named.
//!
//! **Checked with the loader's own rules and written only if they pass.**
//! `config::fit` is the two steps a catalogue takes for one file, so what is
//! saved is what would load, and a refusal carries the loader's own sentence.
//! Nothing is created on the way to a refusal — not the folder, not a
//! temporary file.
//!
//! **Replacing is something the caller says.** A definition that already holds
//! the id in that scope is found by the id it declares, not by the file's name,
//! because the repository's own place is strict at start and two files naming
//! one id would refuse the repository. Without `overwrite` that is a refusal;
//! with it, the file that held the id is the one replaced.
//!
//! **It is written beside and renamed over**, so a watch or a start reading the
//! folder sees the old file or the new and never half of one.

use std::path::{Path, PathBuf};

use config::{Manifest, Roster, WorkflowDef};
use fleet::repositories::{SavedWorkflow, WorkflowNotSaved};
use ipc::{SaveWorkflow, WorkflowScope};

use crate::setup::{listed, KIT_WORKFLOWS, WORKFLOWS};

/// Check `asked` against `manifest` and `roster`, and write it in its scope.
pub fn save(
    root: &Path,
    kit: &Path,
    manifest: &Manifest,
    roster: &Roster,
    asked: &SaveWorkflow,
) -> Result<SavedWorkflow, WorkflowNotSaved> {
    let dir = match asked.scope {
        WorkflowScope::Repository => root.join(WORKFLOWS),
        WorkflowScope::Kit => kit.join(KIT_WORKFLOWS),
    };
    let text = asked.definition.trim();

    // A name for the loader's refusals to cite: the file is not named until the
    // definition has said what it is.
    let unnamed = dir.join("(definition).json");
    let fitted =
        config::fit(&unnamed, text, roster, manifest).map_err(|why| WorkflowNotSaved::Unfit {
            why: why.to_string(),
        })?;
    let id = fitted.as_str().to_string();
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(WorkflowNotSaved::NotAName { id });
    }

    let holding = holding(&dir, &id, roster).map_err(|why| WorkflowNotSaved::Unfit { why })?;
    let (target, replaced) = match holding.as_slice() {
        [] => (dir.join(format!("{id}.json")), false),
        [only] if asked.overwrite => (only.clone(), true),
        [only, ..] if !asked.overwrite => {
            return Err(WorkflowNotSaved::Exists {
                id,
                file: only.display().to_string(),
            })
        }
        several => {
            let files: Vec<String> = several.iter().map(|f| f.display().to_string()).collect();
            return Err(WorkflowNotSaved::Unfit {
                why: format!(
                    "{} all define `{id}`, and a place holds one definition of an id: remove \
                     all but one, then save again",
                    files.join(", ")
                ),
            });
        }
    };
    // A file named for the id that defines something else is not this id's to
    // replace, whatever `overwrite` says.
    if !replaced && target.exists() {
        return Err(WorkflowNotSaved::Unfit {
            why: format!(
                "{} exists and does not define `{id}`; it is not replaced",
                target.display()
            ),
        });
    }

    write(&dir, &target, text).map_err(|cause| WorkflowNotSaved::Unwritable {
        file: target.display().to_string(),
        cause,
    })?;
    Ok(SavedWorkflow {
        id: fitted,
        file: target.display().to_string(),
        replaced,
    })
}

/// The files in `dir` that define `id`, by what they declare.
///
/// A file that will not parse defines nothing here: it is left out of a
/// catalogue for its own reason and is not this save's to find.
fn holding(dir: &Path, id: &str, roster: &Roster) -> Result<Vec<PathBuf>, String> {
    let files = listed(dir).map_err(|why| why.to_string())?;
    Ok(files
        .into_iter()
        .filter(|path| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| WorkflowDef::parse(path, &text, roster).ok())
                .is_some_and(|def| def.id().as_str() == id)
        })
        .collect())
}

fn write(dir: &Path, target: &Path, text: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    // An extension a definition does not carry, so a read of the folder in
    // between never takes the half-written file for one.
    let beside = target.with_extension("saving");
    std::fs::write(&beside, format!("{text}\n"))?;
    std::fs::rename(&beside, target).inspect_err(|_| {
        let _ = std::fs::remove_file(&beside);
    })
}
