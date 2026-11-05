//! Keys that kept their value under a new name, so a settings.json written before the rename
//! still reads. The old spellings are the one place the retired word stays.

use store::settings_file::SettingsDocument;

use super::{ASK_HOLD_SECONDS, PROMPT_THOUGHT_ROUTE, WORDS_CAN_ACT};

/// `(old, new)`, for every key renamed when the Helm Job went.
pub const RENAMED: &[(&str, &str)] = &[
    ("features.helmCanAct", WORDS_CAN_ACT.0),
    ("timeouts.helmAskHoldSeconds", ASK_HOLD_SECONDS.0),
    ("prompts.helmRoute", PROMPT_THOUGHT_ROUTE.0),
];

/// `document` with each old key moved under its new name, and whether any was. **A value already
/// under the new name wins**: the old key is dropped either way, so it is read once.
pub fn carried_over(document: &mut SettingsDocument) -> bool {
    let mut moved = false;
    for (old, new) in RENAMED {
        if let Some(value) = document.remove(*old) {
            document.entry((*new).to_string()).or_insert(value);
            moved = true;
        }
    }
    moved
}
