//! The project filter walk: the switcher picks a project and the Jobs list narrows to its Jobs, ⌘T starts a Job in
//! that project and it stands in the list, and All brings every Job back. The Side Job is `terminal::shells::Mock`'s.

use super::{step, Step, Verb};

/// The switcher on a wide bar, its rows in the menu on a narrow one: either way Find has the keys.
const SWITCHER: Verb = Verb::PressOnFirst(&[("button", "Project"), ("button", "Menu")]);

pub static STEPS: [Step; 10] = [
    step("On All the list holds every project's Jobs, each under its name", Verb::Look, "heading", "Armada"),
    step("The project switcher, or below 900 the menu that holds its rows: the keys are in Find", SWITCHER, "searchbox", "Find a project"),
    step("Typing finds Storefront", Verb::Keys(&["s", "t", "o"]), "searchbox", "Find a project"),
    step("Enter picks it: the list holds Storefront's Jobs only", Verb::Key("enter"), "row", "Checkout page loads twice"),
    step("Armada's Jobs are gone from it", Verb::Absent("row", "Fix the merge line freshness chip"), "row", "Dark mode for the settings page"),
    step("⌘T starts a Job in Storefront, on its thread, the keys in its message box", Verb::Key("cmd-t"), "textbox", "Message"),
    step("It stands in the list among Storefront's Jobs", Verb::Look, "row", "New Job"),
    step("The switcher again, on the row the window is on", SWITCHER, "searchbox", "Find a project"),
    step("Typing narrows the rows, and All stays first", Verb::Keys(&["a"]), "searchbox", "Find a project"),
    step("Enter picks it: every Job is back, under its project", Verb::Key("enter"), "heading", "Armada"),
];
