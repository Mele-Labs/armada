//! How the owner works with Jobs, said to a Job in words: the note every Job starts with, the block a
//! plan step adds, and the sentences of the checks that hold a Job to them.
//!
//! **A rule reaches an agent by mechanism, never as a list it chooses to read.** The note is put in the
//! start note and in every helper's brief, and a change is told to the Jobs running; the checks are
//! `waiting_for` and `all_done` refusing by name (`ways_checking.rs`). Every sentence here is for the
//! Job and is fixed words, so a test can hold it.

use ipc::{AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, TakeIn};

/// The first line of the note.
pub(crate) const HEADER: &str = "How the owner works with Jobs, from his Settings. Fleet checks it.";

/// The note: the header, one line for how he takes work in, one for how he answers. **At every
/// default it is the header and the read line**, which is what a Fleet with nothing saved says.
pub(crate) fn note(prefs: &Preferences) -> String {
    let mut lines = vec![HEADER, take_in_line(prefs)];
    if prefs.take_in == TakeIn::Look {
        lines.push(match prefs.look_questions {
            LookQuestions::PicturesMix => "Questions with options that look different: draw each option as a mock and offer them as pictures, with the parts he might take from one and not another named by data-part.",
            LookQuestions::PicturesWhole => "Questions with options that look different: draw each option as a mock and offer them as pictures.",
        });
    }
    lines.extend(answer_line(prefs.answer_with));
    lines.join("\n")
}

fn take_in_line(prefs: &Preferences) -> &'static str {
    match prefs.take_in {
        TakeIn::Read => match prefs.read_results {
            ReadResults::NoteThenPage => "Results: a short note in the thread for small work. A page under .armada/documents/, conclusion first, for work with more than one decision.",
            ReadResults::AlwaysPage => "Results: every result is a page under .armada/documents/, conclusion first.",
            ReadResults::ThreadOnly => "Results: in the thread as text. No separate pages.",
        },
        TakeIn::Look => match prefs.look_results {
            LookResults::PictureEvery => "Results: lead every result with a picture through show_sketch, a mock for a screen and a diagram for anything else, with two lines of text under it. all_done is refused until a picture is shown. The result picture rides on the done card, so it asks no approval of its own.",
            LookResults::PictureScreens => "Results: lead a result that changes a screen with a mock through show_sketch. Other results may be text.",
            LookResults::WalkVisible => "Results: open anything he would see as a walk with show_window, and a diagram through show_sketch for the rest. all_done is refused until a picture is shown. The result picture rides on the done card, so it asks no approval of its own.",
        },
        TakeIn::Listen => "Results: Bridge reads your all_done summary aloud. Write it as two or three spoken sentences, saying what changed and what waits on him. No paths, code or tables in it.",
    }
}

fn answer_line(answer: AnswerWith) -> Option<&'static str> {
    match answer {
        AnswerWith::Type => None,
        AnswerWith::Talk => Some("Answers: he answers by voice. Ask questions he can answer in a few spoken words."),
        AnswerWith::Buttons => Some("Answers: ask every question with two to four options, your pick first, a cost on each, one decision a question. A question without options is sent back."),
    }
}

/// What a step that settles a plan or approach with him is told to do, by how he takes work in, and the
/// Buttons line where he answers by buttons. **Empty at every default**, and where reading him means
/// the thread alone, since a page first would be a page he said he does not want.
pub(crate) fn plan_block(prefs: &Preferences) -> String {
    if prefs.take_in == TakeIn::Read && prefs.answer_with == AnswerWith::Type {
        return String::new();
    }
    let take_in = match prefs.take_in {
        TakeIn::Look => Some("Lead with pictures: sketch the shape with show_sketch before you ask."),
        TakeIn::Read if prefs.read_results == ReadResults::ThreadOnly => None,
        TakeIn::Read => Some("Put a page first: the approach as a page under .armada/documents/, conclusion first, then ask."),
        TakeIn::Listen => Some("Open with a short version he can hear: two or three plain sentences in your reply before any detail."),
    };
    let buttons = (prefs.answer_with == AnswerWith::Buttons).then_some(
        "Settle this with him as a quiz: one decision a question through AskUserQuestion, two to four options, your pick first, a cost on each, until the shape is agreed.",
    );
    take_in.into_iter().chain(buttons).collect::<Vec<_>>().join(" ")
}

/// Whether he wants every question asked with options, so Fleet sends one back that has none.
pub(crate) fn buttons(prefs: &Preferences) -> bool {
    prefs.answer_with == AnswerWith::Buttons
}

/// Whether `all_done` is refused until the Job has shown a picture: he looks, and not only at screens.
pub(crate) fn picture_owed(prefs: &Preferences) -> bool {
    prefs.take_in == TakeIn::Look && prefs.look_results != LookResults::PictureScreens
}

/// What the agent reads when `waiting_for` is refused for a question with no options, or too many.
pub(crate) fn options_refusal(id: &str, options: usize) -> String {
    let found = match options {
        0 => "has no options".to_string(),
        many => format!("has {many} options"),
    };
    format!(
        "Refused: item \"{id}\" is a question and {found}. The owner answers by buttons. Ask it with two to four options, \
         your pick first and a cost on each, one decision a question, then call waiting_for again."
    )
}

/// What the agent reads when `all_done` is refused for a Job that has shown no picture.
pub(crate) const PICTURE_REFUSAL: &str =
    "The owner looks at results and this Job has shown no picture. Lead the result with a picture: show_sketch, show_window or show_file with an image. It rides on the done card and asks no approval of its own. Then call all_done again.";
