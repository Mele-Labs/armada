//! What a Job starts with to cost fewer tokens, read from the settings each time one starts:
//! whether it reaches the person's connectors and which of their skills it is shown.
//! `crates/config/settings.toml`, *Skills switched off for Jobs* and *Connectors switched off*.
//!
//! **Lenient**: a value that is not the shape reads as nothing set, so a hand edit gone wrong
//! leaves a Job as it would have been rather than refusing it.

use std::path::Path;

use store::settings_file::SettingValue;

/// Where a repository keeps skills of its own, under its root.
const OWN_SKILLS: &str = ".claude/skills";

/// Whether a repository's Jobs reach the person's connectors. The person's switch for it wins,
/// and a repository with none follows its `armada.yml`, `armada_yml`.
pub(crate) fn connectors_on(off: Option<SettingValue>, repository: &str, armada_yml: bool) -> bool {
    match off {
        Some(SettingValue::Map(by)) => match by.get(repository) {
            Some(SettingValue::Bool(off)) => !off,
            _ => armada_yml,
        },
        _ => armada_yml,
    }
}

/// The skills switched off for `repository`, **without the repository's own and without Armada's**
/// even where the setting names them: every Job gets those. `root` is the repository's checkout,
/// where its own skills are found by name.
pub(crate) fn skills_off(off: Option<SettingValue>, repository: &str, root: Option<&Path>) -> Vec<String> {
    let Some(SettingValue::Map(by)) = off else { return Vec::new() };
    let Some(SettingValue::List(named)) = by.get(repository) else { return Vec::new() };
    named
        .iter()
        .filter_map(|name| match name {
            SettingValue::Text(name) => Some(name.clone()),
            _ => None,
        })
        // A plugin's skill is `<plugin>:<skill>`, and a repository's own has no prefix.
        .filter(|name| name.contains(':') || !root.is_some_and(|root| root.join(OWN_SKILLS).join(name).exists()))
        .collect()
}
