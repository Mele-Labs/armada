//! A person's Bridge preferences, kept the way [`crate::limits`] are kept —
//! Fleet-wide, surviving a relaunch, and read back whole after every save.
//!
//! **`value` is a plain `bool` because a preference is a switch or a word.**
//! A word (`theme`, the closed sets of how he works with Jobs, `listen_voice`)
//! rides in [`SavePreference::text`]; a third shape is a decision for whoever
//! adds it, not a generality bought here on spec.

use serde::{Deserialize, Serialize};

/// Every preference Fleet knows, and what is in force for each.
///
/// **Flat, one field per known preference** — the five settings.json keys it reads,
/// carried across the wire rather than restated as a map: a name outside this
/// struct cannot be read, which is the closed set on this side of the seam.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub where_things_are_open: bool,
    /// Whether this machine offers a pull request as a draft unless the
    /// repository, the workflow or the Job says otherwise. **Since 23.68.**
    /// Absent is false, and from a Fleet before it, which offered every one ready.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft_pull_requests: bool,
    /// The theme Bridge draws with: a built-in's id or a mod's name. Fleet
    /// holds the word and Bridge decides what it names, so one that names
    /// nothing is Bridge falling back to the default. Left out while it is
    /// [`DEFAULT_THEME`], and absent reads as that from a Fleet before it.
    #[serde(default = "dark", skip_serializing_if = "is_default_theme")]
    pub theme: String,
    /// The owner's own layout choices, as a `layout.json` that Settings → Layout wrote. They
    /// outrank every layout mod. Left out while he has made none, and the same text a layout mod
    /// holds, so one parser reads both.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layout_choices: String,
    /// The owner's own key bindings, as the JSON Settings → Keyboard wrote: an act's id to the
    /// keys it answers in place of the registry's. Left out while he has changed none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key_bindings: String,
    /// How the owner has Bridge draw a time, as the JSON Settings, Time wrote: three switches,
    /// `hour12`, `seconds` and `date`. Left out while he has chosen none, which reads as the 24-hour
    /// clock with seconds and no date.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub time_format: String,
    /// What he takes a Job's work in: read it, look at it, or listen to it. Left out
    /// while it is `read`, and absent reads as that from a Fleet before it.
    #[serde(default, skip_serializing_if = "is_default")]
    pub take_in: TakeIn,
    /// How he answers a Job's question: type, talk or press a button. Left out while it is `type`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub answer_with: AnswerWith,
    /// Whether he has been asked how he works with Jobs. Left out while it is `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub first_run_answered: bool,
    /// Where a Job's result goes when he reads. Left out while it is `note_then_page`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub read_results: ReadResults,
    /// How a Job's result shows when he looks. Left out while it is `picture_every`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub look_results: LookResults,
    /// How a Job's question offers its options when he looks. Left out while it is `pictures_mix`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub look_questions: LookQuestions,
    /// Whether a Job that ends is read out in a summary when he listens. Left out while `true`.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub listen_end_summary: bool,
    /// Whether a Job that needs him is read out when he listens. Left out while `true`.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub listen_needs_you: bool,
    /// Whether a Job's progress is read out when he listens. Left out while `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub listen_progress: bool,
    /// Whether the Operator's words are read out when he listens. Left out while `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub listen_operator: bool,
    /// The voice he listens with, by the name the Mac gives it. Empty is the Mac's default voice, and
    /// is left out.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub listen_voice: String,
    /// Whether a text box shows under a question's options. Left out while `true`.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub buttons_words: bool,
}

fn yes() -> bool {
    true
}

fn is_yes(value: &bool) -> bool {
    *value
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

/// A closed set that rides in [`SavePreference::text`] as one word, which Fleet refuses by name when it is
/// not one of these. `WORDS` leads with the default.
macro_rules! words {
    ($($(#[$meta:meta])* $name:ident { $($variant:ident = $word:literal),+ })+) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            #[default]
            $($variant),+
        }

        impl $name {
            /// Every word, the default first.
            pub const WORDS: &'static [&'static str] = &[$($word),+];

            /// The set's variant for a word, or `None` where it is not one.
            pub fn from_word(word: &str) -> Option<$name> {
                serde_json::from_value(serde_json::Value::String(word.to_string())).ok()
            }
        }
    )+};
}

words! {
    /// What he takes a Job's work in.
    TakeIn { Read = "read", Look = "look", Listen = "listen" }
    /// How he answers a Job's question.
    AnswerWith { Type = "type", Talk = "talk", Buttons = "buttons" }
    /// Where a Job's result goes when he reads: a note in the thread and then the page, the page every
    /// time, or the thread alone.
    ReadResults { NoteThenPage = "note_then_page", AlwaysPage = "always_page", ThreadOnly = "thread_only" }
    /// How a Job's result shows when he looks: a picture of every change, of every screen, or the walk
    /// of what is visible.
    LookResults { PictureEvery = "picture_every", PictureScreens = "picture_screens", WalkVisible = "walk_visible" }
    /// How a Job's question offers its options when he looks: pictures he can mix parts of, or whole ones.
    LookQuestions { PicturesMix = "pictures_mix", PicturesWhole = "pictures_whole" }
}

/// The theme nobody has chosen away from.
pub const DEFAULT_THEME: &str = "dark";

fn dark() -> String {
    DEFAULT_THEME.to_string()
}

fn is_default_theme(theme: &String) -> bool {
    theme == DEFAULT_THEME
}

impl Default for Preferences {
    fn default() -> Preferences {
        Preferences {
            where_things_are_open: false,
            draft_pull_requests: false,
            theme: dark(),
            layout_choices: String::new(),
            key_bindings: String::new(),
            time_format: String::new(),
            take_in: TakeIn::default(),
            answer_with: AnswerWith::default(),
            first_run_answered: false,
            read_results: ReadResults::default(),
            look_results: LookResults::default(),
            look_questions: LookQuestions::default(),
            listen_end_summary: true,
            listen_needs_you: true,
            listen_progress: false,
            listen_operator: false,
            listen_voice: String::new(),
            buttons_words: true,
        }
    }
}

/// A save — `POST /preferences/save`. **One preference, not the set.**
/// `SaveLimits` sends every field it has an opinion on in one request because
/// the three limits are always read and saved together; a preference is read
/// as a whole struct but saved one at a time; leaving the rest is stated
/// rather than achieved by omission.
///
/// **`name` is a plain string, not a closed wire type.** The set it must
/// belong to is `store`'s own `CHECK`, and a name outside it is refused by
/// name — `fleet.unknown_preference` — rather than failing to decode, which is
/// what a closed wire type here would do instead and would say nothing about
/// which name was sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePreference {
    pub name: String,
    pub value: bool,
    /// The value of a preference that is text rather than a switch, which is
    /// `theme`, `layout_choices`, `key_bindings`, `time_format`, `listen_voice` and the five closed sets
    /// `take_in`, `answer_with`, `read_results`, `look_results` and `look_questions` (one word of
    /// the set, refused by name if it is not one). `value` is read for every other
    /// name and this is read for none of them. An empty `layout_choices`, `key_bindings`,
    /// `time_format` or `listen_voice` takes the owner's choices back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
