//! The generated vocabulary says what `crates/core-model/domain/` says.
//!
//! The registries are gated and what the native Bridge renders from them is checked in as Rust, so a
//! registry change nobody regenerated against left a file that compiles clean and renders the words
//! the registry has already moved off. Every other gate stayed green, because none of them reads it.
//!
//! **The comparison is against the registries**, read by [`crate::vocabulary`], which is the one
//! emitter there is and is Rust like the gate. `verify-tokens` does the same for the tokens.
//!
//! **The rule refuses; it does not rewrite.** A gate that silently regenerates means nobody learns
//! the step exists. `cargo xtask verify-vocabulary --write` is the rewrite.

use std::fs;
use std::path::Path;

use crate::vocabulary::{outputs, WRITE};
use crate::Report;

/// Rule: the generated vocabulary says what the registries say.
pub fn the_generated_vocabulary_says_what_the_registries_say(root: &Path) -> Report {
    let mut report = Report::new("the generated vocabulary says what the registries say");

    let emitted = match outputs(root) {
        Ok(emitted) => emitted,
        Err(why) => {
            report.fail(format!("xtask/src/vocabulary.rs — {why}"));
            return report;
        }
    };

    for (rel, want) in &emitted {
        match fs::read_to_string(root.join(rel)) {
            Ok(have) if &have == want => {}
            Ok(_) => report.fail(format!(
                "STALE {rel} — it is not what the registries in crates/core-model/domain/ \
                 say. Run `{WRITE}` and commit what it writes"
            )),
            Err(_) => report.fail(format!(
                "{rel} — it is emitted and it is not checked in. Run `{WRITE}`"
            )),
        }
    }

    report
}

pub mod readers;

#[cfg(test)]
mod tests;
