//! What a Job starts with to cost fewer tokens, read from the settings each time one starts:
//! whether it reaches the person's connectors and which of their skills it is shown.
//! `crates/config/settings.toml`, *Skills switched off for Jobs* and *Connectors switched off*.
//!
//! **Lenient**: a value that is not the shape reads as nothing set, so a hand edit gone wrong
//! leaves a Job as it would have been rather than refusing it.

use store::settings_file::SettingValue;

/// Whether a repository's Jobs reach the person's connectors. The person's switch for it wins,
/// and a repository with none follows its `armada.yml`, `armada_yml`.
pub fn connectors_on(off: Option<SettingValue>, repository: &str, armada_yml: bool) -> bool {
    match off {
        Some(SettingValue::Map(by)) => match by.get(repository) {
            Some(SettingValue::Bool(off)) => !off,
            _ => armada_yml,
        },
        _ => armada_yml,
    }
}

/// The skills switched off for `repository`, **without the repository's own and without Armada's**
/// even where the setting names them: every Job gets those. `own` says whether the repository has a
/// skill of that name itself.
pub fn skills_off(off: Option<SettingValue>, repository: &str, own: impl Fn(&str) -> bool) -> Vec<String> {
    let Some(SettingValue::Map(by)) = off else { return Vec::new() };
    let Some(SettingValue::List(named)) = by.get(repository) else { return Vec::new() };
    named
        .iter()
        .filter_map(|name| match name {
            SettingValue::Text(name) => Some(name.clone()),
            _ => None,
        })
        // A plugin's skill is `<plugin>:<skill>`, and a repository's own has no prefix.
        .filter(|name| name.contains(':') || !own(name))
        .collect()
}
