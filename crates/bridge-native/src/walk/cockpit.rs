//! The Cockpit's walk: the Jobs list at the left and the Operator's page at the right with no Job open, a Job opened over the
//! whole right side, and the page brought back by Esc, by the Cockpit's tab and by the same row pressed again. Fleet is the mock
//! in `operator/scenes.rs`. The key map is the last step's.

use super::{answer, press_on, step, Step, Verb};

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

const WORKING: &str = "Native Bridge foundation, Working";
const NEEDS: &str = "Fix the merge line freshness chip, Needs you";

/// The Cockpit with the Operator switched off: the home in the page's place, a slim strip over it holding Switch on.
/// The blocks are the preset's, as Fleet keeps no layout on the mock; the library is the mock's (`features/mock.rs`,
/// `floor_library`), its Jobs the list's.
pub static HOME_STEPS: &[Step] = &[
    answer("The Operator is off: the Cockpit's right side is the home, under a strip that says the Operator is off, with Edit and Switch on. The first block is Needs you", "off", "region", "Needs you"),
    step("System overview is the second block, across the whole width as it is on the Project board, with the panel of the part beside its graph", Verb::Look, "region", "System overview"),
    step("Floor plan is the third: the feature map as one block, each area a card with a header band in its own hue, each feature a row. The home scrolls to it", Verb::Wheel("group", "Home", 700), "region", "Floor plan"),
    step("A chip at the right of a feature is a Job working on it, filled in the colour its group wears in the list", Verb::Look, "button", WORKING),
    step("A press on a Job's chip opens that Job over the whole right side, and the home is gone", Verb::Open("button", NEEDS), "row", THREAD),
    step("Esc with the keys in the Job's box gives them to the list, and Esc again brings the home back", Verb::Keys(&["escape", "escape"]), "region", "Needs you"),
    step("Edit puts the blocks in edit mode: each header can be dragged, each block resized, and Add block appears in the strip", Verb::Open("button", "Edit"), "button", "Done"),
    step("Hide takes a block off the home", Verb::Gone, "button", "Hide Floor plan"),
    step("Add block lists the kinds that are off", Verb::Open("button", "Add block"), "menu", "Blocks to add"),
    step("A pick puts that block back", Verb::Open("menuitem", "Floor plan"), "region", "Floor plan"),
    step("Hide each block in turn", Verb::Gone, "button", "Hide Needs you"),
    step("The last one", Verb::Gone, "button", "Hide Floor plan"),
    step("And the last", Verb::Gone, "button", "Hide System overview"),
    step("With every block hidden the home says Nothing here yet, with Add block beside it", Verb::Look, "status", "Nothing here yet"),
    step("Add block brings the kinds back", Verb::Open("button", "Add block"), "menu", "Blocks to add"),
    step("Needs you comes back at the top", Verb::Open("menuitem", "Needs you"), "region", "Needs you"),
    press_on("Switch on turns the Operator on and its page is at the right again, with no restart", "button", "Switch the Operator on", "textbox", "Message"),
];
