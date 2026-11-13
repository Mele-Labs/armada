//! How he works with Jobs, as Fleet's preferences say (`GET /preferences`). Each is left out of Fleet's answer
//! while it holds its default, so `ipc::Preferences` already reads an absent one as the default and this
//! only holds what was read. Look is the way of taking a Job's work in as pictures: a Needs you item opened
//! whole, a question asked in pictures he can mix parts of, and a result that leads with the picture.

use ipc::{LookQuestions, LookResults, Preferences, TakeIn, WaitingOption};

/// What Fleet last said, for any view drawn on this thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ways {
    take_in: TakeIn,
    look_questions: LookQuestions,
    look_results: LookResults,
}

thread_local! {
    /// Read once a connection, and again after each save.
    static HELD: std::cell::Cell<Ways> = const { std::cell::Cell::new(Ways { take_in: TakeIn::Read, look_questions: LookQuestions::PicturesMix, look_results: LookResults::PictureEvery }) };
}

/// Take Fleet's preferences, on a read or after a save.
pub fn hold(preferences: &Preferences) {
    set(preferences.take_in, preferences.look_questions, preferences.look_results);
}

/// Hold the way directly: the mock scenario that takes its work in as pictures.
pub fn set(take_in: TakeIn, look_questions: LookQuestions, look_results: LookResults) {
    HELD.with(|held| held.set(Ways { take_in, look_questions, look_results }));
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let option = |label: &str| WaitingOption { label: label.into(), description: None, picture: None };
        let options = [option("Sidebar"), option("Stacked (Recommended)"), option("Split")];
        assert_eq!(pick_of(&options), Some(1));
        assert_eq!(name_of(&options[1].label), "Stacked");
        assert_eq!(name_of(&options[0].label), "Sidebar");
        assert_eq!(pick_of(&[option("Yes"), option("No")]), None);
    }
}
