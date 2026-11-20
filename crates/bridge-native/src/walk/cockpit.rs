//! The Cockpit's walk: the Jobs list at the left and the Operator's page at the right with no Job open, a Job opened over the
//! whole right side, and the page brought back by Esc, by the Cockpit's tab and by the same row pressed again. Fleet is the mock
//! in `operator/scenes.rs`. The key map is the last step's.

use super::{press_on, step, Step, Verb};

const FRONT: &str = "Two Jobs are changing paging.rs";
const JOB: &str = "Fix the merge line freshness chip";
const THREAD: &str = "Thread: Fix the merge line freshness chip";

pub static STEPS: &[Step] = &[
    step("The window opens on the Cockpit: its tab first in the strip, then Reviews, Kit and Features. There is no Operator tab and no Jobs tab", Verb::Look, "tab", "Cockpit"),
    step("The Jobs list is at the left and the Operator's page at the right, with no Job open: its header row says the state once, and the card in front is below it", Verb::Look, "article", FRONT),
    step("A press on a Job's row opens that Job over the whole right side, and the page is gone", Verb::Open("row", JOB), "row", THREAD),
    step("Esc with the keys in the Job's box gives them to the list, as it does in any Job, and Esc again brings the Operator back", Verb::Keys(&["escape", "escape"]), "article", FRONT),
    step("Open the Job again", Verb::Open("row", JOB), "row", THREAD),
    press_on("The Cockpit's tab pressed again brings the Operator back", "tab", "Cockpit", "article", FRONT),
    step("Open the Job once more", Verb::Open("row", JOB), "row", THREAD),
    press_on("The same Job's row pressed again brings the Operator back, with the keys left on the list", "row", JOB, "article", FRONT),
    step("⌘J on the Cockpit with no Job open puts the caret in the Operator's box. With a Job open it opens that Job's chat, as it does anywhere in the deck", Verb::Key("cmd-j"), "textbox", "Message"),
];
