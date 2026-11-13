//! A person's Bridge preferences in their older shape: five keys of
//! settings.json, read and saved one at a time. **No overlay to compute**:
//! nothing here is applied to a running Drone but `draft_pull_requests`, which
//! `crate::tuning` holds.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key, Resolved};
use ipc::{AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, SavePreference, SaveSettings, SettingValue, TakeIn};
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
/// A `save_preferences` for `time_format` carrying text that is not the three switches. A 422.
const UNACCEPTABLE_TIME_FORMAT: &str = "fleet.unacceptable_time_format";

/// A `save_preferences` for a closed set carrying a word that is not in it. A 422.
const UNACCEPTABLE_CHOICE: &str = "fleet.unacceptable_choice";

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
        time_format: text(settings.get(keys::TIME_FORMAT)),
        // A word the file holds that the set lacks reads as the default: the file is checked on save, not on read.
        take_in: TakeIn::from_word(&settings.get(keys::TAKE_IN)).unwrap_or_default(),
        answer_with: AnswerWith::from_word(&settings.get(keys::ANSWER_WITH)).unwrap_or_default(),
        first_run_answered: settings.get(keys::FIRST_RUN_ANSWERED),
        read_results: ReadResults::from_word(&settings.get(keys::READ_RESULTS)).unwrap_or_default(),
        look_results: LookResults::from_word(&settings.get(keys::LOOK_RESULTS)).unwrap_or_default(),
        look_questions: LookQuestions::from_word(&settings.get(keys::LOOK_QUESTIONS)).unwrap_or_default(),
        listen_end_summary: settings.get(keys::LISTEN_END_SUMMARY),
        listen_needs_you: settings.get(keys::LISTEN_NEEDS_YOU),
        listen_progress: settings.get(keys::LISTEN_PROGRESS),
        listen_operator: settings.get(keys::LISTEN_OPERATOR),
        listen_voice: settings.get(keys::LISTEN_VOICE),
        buttons_words: settings.get(keys::BUTTONS_WORDS),
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

    /// The word of a closed set as the file holds it, or a 422 naming the preference and what was sent.
    fn word_of<T>(&self, name: &str, text: &str, parse: fn(&str) -> Option<T>) -> Result<SettingValue, Refusal> {
        match parse(text) {
            Some(_) => Ok(SettingValue::Text(text.to_string())),
            None => Err(Refusal::Unacceptable(
                ipc::WireError::raised(UNACCEPTABLE_CHOICE, format!("`{text}` is not a choice of `{name}`"), self.run_id())
                    .with_field("preference", ipc::WireValue::Str(name.to_string()))
                    .with_field("choice", ipc::WireValue::Str(text.to_string())),
            )),
        }
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
            "time_format" => {
                if let Some(first) = ipc::time_format::problems(text).first().filter(|_| !text.is_empty()) {
                    return Err(unacceptable(
                        UNACCEPTABLE_TIME_FORMAT,
                        format!("the time format could not be read: {first}"),
                    ));
                }
                (keys::TIME_FORMAT.name(), json_change(text))
            }
            "take_in" => (keys::TAKE_IN.name(), Some(self.word_of("take_in", text, TakeIn::from_word)?)),
            "answer_with" => (keys::ANSWER_WITH.name(), Some(self.word_of("answer_with", text, AnswerWith::from_word)?)),
            "read_results" => (keys::READ_RESULTS.name(), Some(self.word_of("read_results", text, ReadResults::from_word)?)),
            "look_results" => (keys::LOOK_RESULTS.name(), Some(self.word_of("look_results", text, LookResults::from_word)?)),
            "look_questions" => (keys::LOOK_QUESTIONS.name(), Some(self.word_of("look_questions", text, LookQuestions::from_word)?)),
            // Blank takes the choice back: the Mac's own voice.
            "listen_voice" => (keys::LISTEN_VOICE.name(), (!text.trim().is_empty()).then(|| SettingValue::Text(text.trim().to_string()))),
            "first_run_answered" => (keys::FIRST_RUN_ANSWERED.name(), Some(SettingValue::Bool(save.value))),
            "listen_end_summary" => (keys::LISTEN_END_SUMMARY.name(), Some(SettingValue::Bool(save.value))),
            "listen_needs_you" => (keys::LISTEN_NEEDS_YOU.name(), Some(SettingValue::Bool(save.value))),
            "listen_progress" => (keys::LISTEN_PROGRESS.name(), Some(SettingValue::Bool(save.value))),
            "listen_operator" => (keys::LISTEN_OPERATOR.name(), Some(SettingValue::Bool(save.value))),
            "buttons_words" => (keys::BUTTONS_WORDS.name(), Some(SettingValue::Bool(save.value))),
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
