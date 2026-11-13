//! How he works with Jobs, as Fleet's preferences say (`GET /preferences`). Each is left out of Fleet's answer
//! while it holds its default, so `ipc::Preferences` already reads an absent one as the default and this
//! only holds what was read. Buttons is the way of answering that draws a question's options as presses.
//! Look is the way of taking a Job's work in as pictures: a Needs you item opened
//! whole, a question asked in pictures he can mix parts of, and a result that leads with the picture.

use ipc::{AnswerWith, LookQuestions, LookResults, Preferences, TakeIn, WaitingOption};

/// What Fleet last said, for any view drawn on this thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ways {
    answer_with: AnswerWith,
    buttons_words: bool,
    take_in: TakeIn,
    look_questions: LookQuestions,
    look_results: LookResults,
}

thread_local! {
    /// Read once a connection, and again after each save.
    static HELD: std::cell::Cell<Ways> = const {
        std::cell::Cell::new(Ways {
            answer_with: AnswerWith::Type,
            buttons_words: true,
            take_in: TakeIn::Read,
            look_questions: LookQuestions::PicturesMix,
            look_results: LookResults::PictureEvery,
        })
    };
}

/// Take Fleet's preferences, on a read or after a save.
pub fn hold(preferences: &Preferences) {
    set(preferences.answer_with, preferences.buttons_words);
    set_look(preferences.take_in, preferences.look_questions, preferences.look_results);
}

/// Hold the way of answering directly: the mock scenario that answers in buttons.
pub fn set(answer_with: AnswerWith, buttons_words: bool) {
    HELD.with(|held| held.set(Ways { answer_with, buttons_words, ..held.get() }));
}

/// Hold the way of taking work in directly: the mock scenario that takes its work in as pictures.
pub fn set_look(take_in: TakeIn, look_questions: LookQuestions, look_results: LookResults) {
    HELD.with(|held| held.set(Ways { take_in, look_questions, look_results, ..held.get() }));
}

/// Whether a question's options are drawn as presses.
pub fn answers_with_buttons() -> bool {
    HELD.with(std::cell::Cell::get).answer_with == AnswerWith::Buttons
}

/// Whether Answer in words stands under the buttons.
pub fn words_under_buttons() -> bool {
    HELD.with(std::cell::Cell::get).buttons_words
}

/// Whether he takes a Job's work in as pictures. At its default, `read`, nothing is drawn differently.
pub fn looks() -> bool {
    HELD.with(std::cell::Cell::get).take_in == TakeIn::Look
}

/// Whether a picture's parts can be taken across the options. Off, a press takes the whole option and no mix is built.
pub fn mixes_parts() -> bool {
    looks() && HELD.with(std::cell::Cell::get).look_questions == LookQuestions::PicturesMix
}

/// How a result shows when he looks. Held as Fleet says; the done card leads with the last picture whichever it is.
#[allow(dead_code)]
pub fn look_results() -> LookResults {
    HELD.with(std::cell::Cell::get).look_results
}

/// The mark an asker puts at the end of the option it would pick.
const MARK: &str = "(recommended)";

/// The option the Job picks: the first whose label ends in "(Recommended)", as Fleet's Autopilot and Classic read it.
pub fn pick_of(options: &[WaitingOption]) -> Option<usize> {
    options.iter().position(|option| marked(&option.label))
}

fn marked(label: &str) -> bool {
    label.trim_end().to_lowercase().ends_with(MARK)
}

/// The option's name: its label without the mark.
pub fn name_of(label: &str) -> &str {
    let trimmed = label.trim_end();
    if marked(trimmed) { trimmed[..trimmed.len() - MARK.len()].trim_end() } else { trimmed }
}

/// What a button holds: the option's name, and "My pick: " before the Job's pick. Nothing else rides on it.
pub fn button_word(options: &[WaitingOption], at: usize) -> String {
    let name = options.get(at).map(|option| name_of(&option.label)).unwrap_or_default();
    if pick_of(options) == Some(at) { format!("My pick: {name}") } else { name.to_string() }
}

/// The sentence in the button's tooltip: what choosing the option means, where the asker said.
pub fn button_tip(options: &[WaitingOption], at: usize) -> Option<String> {
    options.get(at).and_then(|option| option.description.as_deref()).map(str::trim).filter(|said| !said.is_empty()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(label: &str, description: Option<&str>) -> WaitingOption {
        WaitingOption { label: label.into(), description: description.map(str::to_string), picture: None }
    }

    #[test]
    fn a_fleet_that_left_both_out_answers_in_type_with_words_under() {
        let read: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
        hold(&read);
        assert!(!answers_with_buttons());
        assert!(words_under_buttons());
    }

    #[test]
    fn buttons_and_no_words_are_read_from_the_preferences() {
        let read: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":false,"answer_with":"buttons","buttons_words":false}"#).expect("decodes");
        hold(&read);
        assert!(answers_with_buttons());
        assert!(!words_under_buttons());
        let talk: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":false,"answer_with":"talk"}"#).expect("decodes");
        hold(&talk);
        assert!(!answers_with_buttons());
        assert!(words_under_buttons());
    }

    #[test]
    fn only_the_option_marked_recommended_reads_my_pick() {
        let options = [option("Keep it a week", Some("Both answer.")), option("Remove it now (Recommended)", Some("One less to carry.")), option("Ask me later", None)];
        assert_eq!(pick_of(&options), Some(1));
        assert_eq!(button_word(&options, 0), "Keep it a week");
        assert_eq!(button_word(&options, 1), "My pick: Remove it now");
        assert_eq!(button_word(&options, 2), "Ask me later");
    }

    #[test]
    fn a_question_with_no_mark_has_no_pick_and_the_first_mark_wins() {
        let plain = [option("Yes", None), option("No", None)];
        assert_eq!(pick_of(&plain), None);
        assert_eq!(button_word(&plain, 0), "Yes");
        let two = [option("A (recommended)", None), option("B (Recommended)", None)];
        assert_eq!(pick_of(&two), Some(0));
        assert_eq!(button_word(&two, 1), "B");
    }

    #[test]
    fn the_tooltip_is_the_options_sentence_and_nothing_where_it_gave_none() {
        let options = [option("Branch", Some("  Stops at the commit.  ")), option("Pull request", None), option("Other", Some("  "))];
        assert_eq!(button_tip(&options, 0).as_deref(), Some("Stops at the commit."));
        assert_eq!(button_tip(&options, 1), None);
        assert_eq!(button_tip(&options, 2), None);
    }

    #[test]
    fn a_fleet_that_left_all_three_out_takes_work_in_as_words() {
        let read: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":true}"#).expect("an older Fleet's");
        hold(&read);
        assert!(!looks());
        assert!(!mixes_parts());
        assert_eq!(look_results(), LookResults::PictureEvery);
    }

    #[test]
    fn look_is_read_from_the_preferences_and_the_whole_switch_turns_the_mix_off() {
        let look: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":false,"take_in":"look"}"#).expect("decodes");
        hold(&look);
        assert!(looks());
        assert!(mixes_parts());
        let whole: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":false,"take_in":"look","look_questions":"pictures_whole","look_results":"walk_visible"}"#).expect("decodes");
        hold(&whole);
        assert!(looks());
        assert!(!mixes_parts());
        assert_eq!(look_results(), LookResults::WalkVisible);
        // The question way means nothing while he reads.
        let read: Preferences = ipc::decode("preferences", br#"{"where_things_are_open":false,"look_questions":"pictures_mix"}"#).expect("decodes");
        hold(&read);
        assert!(!mixes_parts());
    }

    #[test]
    fn the_pick_is_the_option_marked_recommended_and_its_name_drops_the_mark() {
        let options = [option("Sidebar", None), option("Stacked (Recommended)", None), option("Split", None)];
        assert_eq!(pick_of(&options), Some(1));
        assert_eq!(name_of(&options[1].label), "Stacked");
        assert_eq!(name_of(&options[0].label), "Sidebar");
        assert_eq!(pick_of(&[option("Yes", None), option("No", None)]), None);
    }
}
