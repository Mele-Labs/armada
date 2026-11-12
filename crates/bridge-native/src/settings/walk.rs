//! `native-settings`: the overlay opened each way, searched, and each kind of control used, on the mock Fleet in
//! `mock.rs`. Its rows are Classic's mock settings, so the shots compare against Classic's on the same values.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const fn press_on(say: &'static str, role: &'static str, name: &'static str, then_role: &'static str, then: &'static str) -> Step {
    step(say, Verb::PressOn(role, name), then_role, then)
}

pub static STEPS: &[Step] = &[
    step("⌘, opens Settings over the app, at nine tenths of the window, on Limits", Verb::Key("cmd-,"), "dialog", "Settings"),
    step("Search sits at the top left, over the groups", Verb::Look, "searchbox", "Search Settings"),
    step("A figure in a short range is dragged as well as typed", Verb::Look, "textbox", "Drones at once"),
    step("A setting changed from what ships says so, with Reset", Verb::Look, "button", "Reset: Drones at once"),
    step(
        "Enter goes into the page, Tab reaches the slider, and → moves the figure a step; Save comes up beside it",
        Verb::Keys(&["enter", "tab", "tab", "right"]),
        "button",
        "Save Drones at once",
    ),
    press_on("Save sends it to Fleet, and the figure stands", "button", "Save Drones at once", "button", "Reset: Drones at once"),
    step("Type past the range", Verb::Type("9"), "textbox", "Drones at once"),
    step("A figure out of range says the range, and Save does not send it", Verb::Look, "alert", "Between 1 and 8."),
    step("⌘F goes to the search", Verb::Key("cmd-f"), "searchbox", "Search Settings"),
    step("Typing finds a page by any setting it holds", Verb::Typed("ghostty"), "region", "Terminal"),
    step("Only the pages it found are listed", Verb::Absent("button", "Limits"), "button", "Terminal"),
    press_on("A choice opens its options under it", "combobox", "Terminal", "menuitemcheckbox", "Ghostty"),
    press_on("Picking one saves it", "menuitemcheckbox", "Ghostty", "combobox", "Terminal"),
    step("Esc clears the search first, and the page picked before comes back", Verb::Key("escape"), "region", "Limits"),
    press_on("Features: a switch per feature, then the folders a Job's card always allows or blocks", "button", "Features", "region", "Folders jobs reach"),
    press_on("A switch saves when pressed", "switch", "Interface sounds", "switch", "Interface sounds"),
    press_on("A setting Fleet reads at start says it waits for a restart", "button", "Timeouts", "status", "Restart Fleet to apply."),
    press_on(
        "A setting set where Fleet runs says the value here is not the one in force",
        "button",
        "Harness",
        "status",
        "Overridden by $ARMADA_AGENT_BINARY where Fleet runs, so the value here is not the one in force.",
    ),
    step("A list is typed a line each", Verb::Look, "textbox", "Drone PATH"),
    press_on("A prompt is a box of its own", "button", "Prompts", "textbox", "Drone baseline"),
    press_on("Keyboard shortcuts: every act with a key, under where you stand when you press it", "button", "Keyboard shortcuts", "region", "Keyboard shortcuts"),
    press_on("A press on a key waits for the next one", "button", "Command palette: ⌘K", "button", "Command palette: press a key, Esc to cancel, Delete to unbind"),
    step("A key another act answers is kept, and the row says which", Verb::Key("cmd-,"), "button", "Command palette: ⌘,"),
    press_on("Reset puts the shipped key back", "button", "Reset Command palette", "button", "Command palette: ⌘K"),
    step("⌘F, then words find an act inside the page", Verb::Key("cmd-f"), "searchbox", "Search Settings"),
    step("Only the acts it found are drawn", Verb::Typed("palette"), "group", "Command palette"),
    step("Esc clears the search, and every act is back", Verb::Key("escape"), "group", "Open Settings"),
    press_on("Theme: one field holding the theme in force, with its colours, and the theme mods under it", "button", "Theme", "region", "Theme"),
    step("A theme mod is a row of its own: its kind, its switch, and Promote", Verb::Look, "switch", "dusk"),
    press_on("The field opens the themes by tone, each with its colours", "combobox", "Theme", "group", "Dark"),
    step("Typing narrows them to the titles that hold it", Verb::Typed("nord"), "option", "Nord"),
    step("Enter picks it, and the window takes it at once", Verb::Key("enter"), "combobox", "Theme"),
    step("↓ opens the list again, on the theme in force", Verb::Key("down"), "option", "Nord"),
    step("⌘A selects what the field holds, so typing replaces it", Verb::Key("cmd-a"), "combobox", "Theme"),
    step("The mods' themes are a group of their own", Verb::Typed("dusk"), "group", "From mods"),
    step("Enter picks a mod's theme as it does any", Verb::Key("enter"), "combobox", "Theme"),
    step("Typed again, and Dark is back", Verb::Keys(&["cmd-a", "d", "a", "r", "k", "enter"]), "combobox", "Theme"),
    press_on("A mod's switch saves at once", "switch", "dusk", "switch", "dusk"),
    press_on("Promote puts the mod into Armada by a Job of its own, which goes under the row", "button", "Promote dusk", "button", "Job dusk"),
    step("Esc with nothing searched closes Settings, and the keys go back", Verb::Key("escape"), "button", "Menu"),
    press_on("The menu at the top right holds Settings", "button", "Menu", "menuitem", "Settings"),
    press_on("It opens Settings again, its search empty", "menuitem", "Settings", "dialog", "Settings"),
    step("Esc closes it", Verb::Key("escape"), "button", "Menu"),
    step("The project switcher is on the wide bar", Verb::Widen(1100), "button", "Project"),
    press_on("The project switcher's Settings opens it on that project", "button", "Project", "button", "Settings"),
    press_on("Its own values come first, under its name; a value it does not set says where it comes from", "button", "Settings", "group", "Armada"),
    step("How work is reviewed: Plans, Pull requests, then Walks", Verb::Look, "combobox", "Walks"),
    press_on("Walks is Off or Review", "combobox", "Walks", "menuitemcheckbox", "Review"),
    press_on("Picking Review writes the project's values whole, the rest kept", "menuitemcheckbox", "Review", "combobox", "Walks"),
    step("Esc closes it", Verb::Key("escape"), "button", "Project"),
];
