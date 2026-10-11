//! A person's Bridge preferences in their older shape: five keys of
//! settings.json, read and saved one at a time. **No overlay to compute**:
//! nothing here is applied to a running Drone but `draft_pull_requests`, which
//! `crate::tuning` holds.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key, Resolved};
use ipc::{Preferences, SavePreference, SaveSettings, SettingValue};
use store::settings_file;
use store::WriteError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A `save_preferences` for `theme` carrying nothing a theme can be called. A 422.
const UNACCEPTABLE_THEME: &str = "fleet.unacceptable_theme";
/// A `save_preferences` for `layout_choices` carrying text `layout.json` would refuse. A 422.
const UNACCEPTABLE_LAYOUT: &str = "fleet.unacceptable_layout";
/// A `save_preferences` for `key_bindings` carrying text Bridge could not read. A 422.
const UNACCEPTABLE_KEY_BINDINGS: &str = "fleet.unacceptable_key_bindings";

/// The preferences settings.json holds, as the older wire spells them. A JSON
/// preference is carried as its text, which is what Bridge parses.
pub(crate) fn preferences_of(settings: &Resolved) -> Preferences {
    let text = |value: Option<settings_file::SettingValue>| value.map(|v| v.to_json_text()).unwrap_or_default();
    Preferences {
        where_things_are_open: settings.get(keys::WHERE_THINGS_ARE_OPEN),
        draft_pull_requests: settings.get(keys::DRAFT_PULL_REQUESTS),
        theme: settings.get(keys::THEME),
        layout_choices: text(settings.get(keys::LAYOUT)),
        key_bindings: text(settings.get(keys::KEY_BINDINGS)),
    }
}

/// Text of a JSON preference as the value the file holds; empty removes it.
fn json_change(text: &str) -> Option<SettingValue> {
    match text.is_empty() {
        true => None,
        false => settings_file::SettingValue::from_json_text(text).ok().map(|value| crate::settings::wire_value(&value)),
    }
}

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
    /// Every preference in force, off the last good read of the file.
    pub(crate) fn preferences_in_force(&self) -> Preferences {
        preferences_of(&self.machine_settings().now())
    }

    /// Save one preference into settings.json, then answer with every
    /// preference now in force. Each refusal keeps the code it always had.
    pub(crate) async fn save_preference_now(&self, save: SavePreference) -> Result<Preferences, Refusal> {
        let text = save.text.as_deref().unwrap_or_default();
        let unacceptable = |code: &str, why: String| Refusal::Unacceptable(ipc::WireError::raised(code, why, self.run_id()));
        let (key, value) = match save.name.as_str() {
            "theme" => {
                if let Some(why) = crate::mods::theme_id_problem(text) {
                    return Err(Refusal::Unacceptable(
                        ipc::WireError::raised(UNACCEPTABLE_THEME, why, self.run_id())
                            .with_field("theme", ipc::WireValue::Str(text.to_string())),
                    ));
                }
                (keys::THEME.name(), Some(SettingValue::Text(text.to_string())))
            }
            "layout_choices" => {
                // Empty takes the choices back; anything else is a layout.json or it is refused.
                if let Some(first) = ipc::layout::problems(text).first().filter(|_| !text.is_empty()) {
                    return Err(unacceptable(
                        UNACCEPTABLE_LAYOUT,
                        format!("the layout choices are not a layout.json: {first}"),
                    ));
                }
                (keys::LAYOUT.name(), json_change(text))
            }
            "key_bindings" => {
                if let Some(first) = ipc::key_bindings::problems(text).first().filter(|_| !text.is_empty()) {
                    return Err(unacceptable(
                        UNACCEPTABLE_KEY_BINDINGS,
                        format!("the key bindings could not be read: {first}"),
                    ));
                }
                (keys::KEY_BINDINGS.name(), json_change(text))
            }
            "where_things_are_open" => (keys::WHERE_THINGS_ARE_OPEN.name(), Some(SettingValue::Bool(save.value))),
            "draft_pull_requests" => (keys::DRAFT_PULL_REQUESTS.name(), Some(SettingValue::Bool(save.value))),
            other => {
                return Err(self.refusal(Adrift::Writing(WriteError::UnknownPreference { name: other.to_string() })))
            }
        };
        let change = SaveSettings { changes: [(key.to_string(), value)].into_iter().collect() };
        self.save_settings_now(change).await?;
        Ok(self.preferences_in_force())
    }
}
