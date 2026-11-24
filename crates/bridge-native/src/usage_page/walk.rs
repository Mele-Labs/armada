//! The `usage` walk: the page found by name, the week read, a setting changed from the keys, a kind opened and moved between, the windows, then the page loading, a change
//! Fleet refused, a read Fleet refused and a window with no burn. It is a Job's own check of the screen and its source of pictures; it is not shown to him.

use crate::walk::{Screen, Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb, target: Target { role, name } }
}

/// The page built again on a scenario of its own.
const fn fresh(say: &'static str, scenario: &'static str) -> Step {
    Step { id: None, say, verb: Verb::FreshIn(Screen::Reviews, scenario), target: Target { role: "tab", name: "Jobs" } }
}

const SUBAGENTS: &str = "Subagents ran past 300k context";

pub static STEPS: [Step; 51] = [
    step("⌘K finds the page by name", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Enter opens the page on the week", Verb::Key("enter"), "region", "Usage"),
    step("The header says what share of the burn was waste a person can fix", Verb::Look, "heading", "32% of this week's burn was waste you can fix"),
    step("A panel of the settings that change it", Verb::Look, "heading", "Settings that change it"),
    step("A row each: the value in force, a dropdown preset to what would save most, what it cuts and what it saves", Verb::Look, "row", "Subagent cache length"),
    step("The dropdown names the setting it changes", Verb::Look, "combobox", "Change Subagent cache length to"),
    step("What it would save is a share of the week's burn", Verb::Look, "label", "5.5% saved"),
    step("Enter on the row opens its values", Verb::Key("enter"), "menu", "Change Subagent cache length to"),
    step("Esc shuts them and nothing is sent", Verb::Key("escape"), "label", "5 min now"),
    step("a applies the value the dropdown stands on, and the row shows it", Verb::Key("a"), "label", "1 hour now"),
    step("What each change did, before and since", Verb::Look, "heading", "After a change"),
    step("Down twice reaches the setting that applies after a restart", Verb::Keys(&["j", "j"]), "row", "Stop an idle Job after"),
    step("Enter opens its values", Verb::Key("enter"), "menu", "Change Stop an idle Job after to"),
    step("Down and Enter pick one", Verb::Keys(&["down", "enter"]), "combobox", "Change Stop an idle Job after to"),
    step("a applies it, and the row says when it takes hold", Verb::Key("a"), "status", "Applies when Fleet restarts"),
    step("A row for each kind of waste, largest first: its glyph, its title and its share of the burn", Verb::Look, "row", SUBAGENTS),
    step("A press on a kind opens its panel under the list", Verb::PressOn("row", SUBAGENTS), "table", "Jobs hit by Subagents ran past 300k context"),
    step("The panel's head is the kind's fix", Verb::Look, "group", "Fix"),
    step("Down and Enter move to the next kind and open it", Verb::Keys(&["down", "enter"]), "table", "Jobs hit by Main thread ran past 300k context"),
    step("d reads the day, and the open kind stays open", Verb::Key("d"), "heading", "32% of today's burn was waste you can fix"),
    step("Esc shuts the panel", Verb::Key("escape"), "region", "Usage"),
    step("w reads the week again", Verb::Key("w"), "radio", "Week"),
    step("f reads the fortnight", Verb::Key("f"), "radio", "Fortnight"),
    step("Enter opens the kind the keys are on", Verb::Key("enter"), "table", "Jobs hit by Main thread ran past 300k context"),
    step("A Job with no title shows its short id", Verb::Look, "row", "5d1e9c42"),
    step("A kind that names its cause adds a column: the file read again", Verb::Keys(&["down", "down", "enter"]), "table", "Jobs hit by Same file read again"),
    fresh("A change Fleet refused", "usage-apply-failed"),
    step("⌘K finds the page", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Enter opens it on the week", Verb::Key("enter"), "label", "5 min now"),
    step("a applies the preset value, and Fleet refuses it", Verb::Key("a"), "alert", "settings.json was not written: it is read only"),
    step("The old value is back and the dropdown stays where it was, so a press tries again", Verb::Look, "combobox", "Change Subagent cache length to"),
    fresh("The page while the reading is not answered", "usage-loading"),
    step("⌘K finds the page", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Shapes stand where the rows will be", Verb::Key("enter"), "img", "Loading"),
    fresh("A read Fleet refused", "usage-failed"),
    step("⌘K finds the page", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Fleet's reason stands under the switch, which still asks another window", Verb::Key("enter"), "alert", "Not read"),
    fresh("A window with no burn", "usage-empty"),
    step("⌘K finds the page", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Enter opens it: the switch stands, and nothing else is said", Verb::Key("enter"), "radiogroup", "Window"),
    step("No row is drawn", Verb::Absent("row", "Cold cache after a pause"), "region", "Usage"),
];
