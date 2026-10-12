//! The prompts a person may override in settings.json, as Fleet ships them and
//! as they stand.
//!
//! **The wording stays where it always lived**, beside the code that builds the
//! prompt and the tests that hold it to `docs/contracts/agent-prompt.md`; this
//! module only gathers it, hands it to the table as each key's default, and
//! answers what is in force. [`Prompts`] rides [`crate::tuning::Tuning`], so a
//! save reaches the next prompt built and never one already sent.
//!
//! **A template's placeholders are filled in one pass**: what fills one is
//! never read again for another, so a request or a record that happens to
//! spell `{record}` reaches the model as written.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use config::settings::{self as keys, Key, Resolved, Supplied, Words};

use crate::helm::Authority;

mod judge;
mod shipped;

/// Every prompt as it ships, read from the module that owns its wording.
fn shipped_texts() -> Vec<(Words, String)> {
    vec![
        (
            keys::PROMPT_DRONE_BASELINE,
            crate::briefing::BASELINE.to_string(),
        ),
        (keys::PROMPT_SCOUT_ASK, crate::scout::shipped_ask()),
        (keys::PROMPT_SCOUT_READ_IN, crate::scout::shipped_read_in()),
        (keys::PROMPT_SCOUT_RESCUE, crate::scout::shipped_rescue()),
        (keys::PROMPT_PROPOSER, crate::proposing::shipped()),
        (keys::PROMPT_RETRO_JOB, crate::retro::QUESTION.to_string()),
        (
            keys::PROMPT_RETRO_SESSION,
            crate::retro::SESSION_QUESTION.to_string(),
        ),
        (
            keys::PROMPT_RETRO_WHERE_FIXES_LAND,
            crate::retro::LANDS_IN.to_string(),
        ),
        (
            keys::PROMPT_RETRO_HOW_TO_WRITE,
            crate::retro::TEXTS.to_string(),
        ),
        (keys::PROMPT_RETRO_REVIEW, crate::retro::REVIEW.to_string()),
        (keys::PROMPT_RETRO_ASK, crate::retro::ASK.to_string()),
        (
            keys::PROMPT_CROSSING_REDIRECT,
            crate::crossing::REDIRECTED.to_string(),
        ),
        (
            keys::PROMPT_CROSSING_OVERTAKEN,
            crate::crossing::OVERTAKEN.to_string(),
        ),
        (
            keys::PROMPT_CROSSING_CONFLICTS,
            crate::crossing::CLEARING_CONFLICTS.to_string(),
        ),
        (keys::PROMPT_HELM, crate::helm::shipped(Authority::Acting)),
        (
            keys::PROMPT_HELM_READ_ONLY,
            crate::helm::shipped(Authority::ReadOnly),
        ),
        (
            keys::PROMPT_TRIGGER_REPAIR,
            crate::trigger_repair::SHIPPED.to_string(),
        ),
    ]
    .into_iter()
    .chain(
        shipped::drone()
            .into_iter()
            .map(|(key, text)| (key, text.to_string())),
    )
    .chain(
        judge::judge()
            .into_iter()
            .map(|(key, piece)| (key, piece.shipped.to_string())),
    )
    .collect()
}

/// `supplied` with every prompt's shipped text as its key's default, which is
/// what Bridge shows and what "Reset to shipped" puts back.
pub fn supplied(supplied: Supplied) -> Supplied {
    shipped_texts()
        .into_iter()
        .fold(supplied, |supplied, (key, text)| supplied.words(key, text))
}

static SHIPPED: LazyLock<Prompts> = LazyLock::new(|| {
    Prompts(Arc::new(
        shipped_texts()
            .into_iter()
            .map(|(key, text)| (key.name(), text))
            .collect(),
    ))
});

/// The words of every overridable prompt at one instant. Cheap to clone, so a
/// prompt being built holds the words it started with.
#[derive(Clone, Debug)]
pub struct Prompts(Arc<BTreeMap<&'static str, String>>);

impl Prompts {
    /// Every prompt as Armada ships it.
    pub fn shipped() -> Prompts {
        SHIPPED.clone()
    }

    /// These prompts with every one the file holds put over them. The table
    /// has already refused a saved prompt that drops a placeholder.
    pub(crate) fn overlaid_by(&self, settings: &Resolved) -> Prompts {
        let mut words = (*self.0).clone();
        for key in keys::prompt_keys() {
            if let Some(chosen) = settings.chosen(key) {
                words.insert(key.name(), chosen);
            }
        }
        Prompts(Arc::new(words))
    }

    /// The words of `key` in force.
    pub fn get(&self, key: Words) -> &str {
        self.0.get(key.name()).map_or("", String::as_str)
    }

    /// The words of `key` with its placeholders filled. See [`fill`].
    pub fn fill(&self, key: Words, values: &[(&str, &str)]) -> String {
        fill(self.get(key), values)
    }

    /// The Judge's pieces as these prompts word them, for `verification` to
    /// lay its briefs in. That crate knows no settings; Fleet owns the override.
    pub(crate) fn wording(&self) -> verification::Wording {
        judge::judge()
            .into_iter()
            .fold(verification::Wording::shipped(), |wording, (key, piece)| {
                wording.with(piece, self.get(key))
            })
    }

    /// Helm's brief for `authority`: one template where it acts, another where it reads.
    pub(crate) fn helm(&self, authority: Authority) -> &str {
        match authority {
            Authority::Acting => self.get(keys::PROMPT_HELM),
            Authority::ReadOnly => self.get(keys::PROMPT_HELM_READ_ONLY),
        }
    }
}

/// `template` with each `{name}` in `values` replaced by its value, in one
/// pass. `verification::fill`, the one implementation of it.
pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
    verification::fill(template, values)
}
