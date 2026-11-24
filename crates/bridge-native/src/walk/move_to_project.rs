//! The move-to-project walk: ⇧↓ selects a second Job, the row menu's Move to project opens the picker, typing finds
//! Storefront and Enter moves both under its heading; then the Job's own menu takes one out to no project. The
//! mock Fleet's stand-in is the Jobs list's own, which sets the project on the records it holds.

use super::{step, Step, Verb};

pub static STEPS: [Step; 11] = [
    step("A live Job in Armada has the keys", Verb::Look, "row", "Fix the merge line freshness chip"),
    step("⇧↓ selects it and the next one with it", Verb::Key("shift-down"), "row", "Native Bridge foundation"),
    step("Shift+F10 opens the row's menu: Move to project is in it", Verb::Key("shift-f10"), "menuitem", "Move to project"),
    step("Move to project opens the picker: his projects, then No project", Verb::Keys(&["down", "down", "down", "enter"]), "searchbox", "Find a project"),
    step("Typing finds Storefront", Verb::Keys(&["s", "t", "o"]), "searchbox", "Find a project"),
    step("Enter moves both Jobs: the first now stands under Storefront", Verb::Key("enter"), "row", "Fix the merge line freshness chip"),
    step("And the second stands beside it", Verb::Look, "row", "Native Bridge foundation"),
    step("Enter opens the second Job", Verb::Key("enter"), "textbox", "Message"),
    step("The Job's own menu, from More", Verb::PressOn("button", "More"), "menuitem", "Move to project"),
    step("Move to project opens the picker with Storefront ticked", Verb::PressOn("menuitem", "Move to project"), "menuitemcheckbox", "No project"),
    step("No project takes the Job out: it leaves the Storefront heading for its repository's", Verb::PressOn("menuitemcheckbox", "No project"), "heading", "armada"),
];
