//! The `usage` walk: the page found by name, the week read, a kind opened and moved between, the windows, then the page loading, a read Fleet refused and
//! a window with no burn. It is a Job's own check of the screen and its source of pictures; it is not shown to him.

use crate::walk::{Screen, Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

/// The page built again on a scenario of its own.
const fn fresh(say: &'static str, scenario: &'static str) -> Step {
    Step { say, verb: Verb::FreshIn(Screen::Reviews, scenario), target: Target { role: "tab", name: "Jobs" } }
}

const SUBAGENTS: &str = "Subagents ran past 300k context";

pub static STEPS: [Step; 31] = [
    step("⌘K finds the page by name", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Usage", Verb::Type("Usage"), "searchbox", "Search actions, jobs and settings"),
    step("Usage is a Page, found by its name", Verb::Look, "option", "Usage"),
    step("Enter opens the page on the week", Verb::Key("enter"), "region", "Usage"),
    step("The header says how much of the burn went to waste", Verb::Look, "heading", "131M of 412M went to fixable waste"),
    step("A row for each kind of waste, largest first: its glyph, its title and its units", Verb::Look, "row", SUBAGENTS),
    step("A press on a kind opens its panel under the list", Verb::PressOn("row", SUBAGENTS), "table", "Jobs hit by Subagents ran past 300k context"),
    step("The panel's head is the kind's fix", Verb::Look, "group", "Fix"),
    step("Down and Enter move to the next kind and open it", Verb::Keys(&["down", "enter"]), "table", "Jobs hit by Main thread ran past 300k context"),
    step("d reads the day, and the open kind stays open", Verb::Key("d"), "radio", "Day"),
    step("Esc shuts the panel", Verb::Key("escape"), "region", "Usage"),
    step("w reads the week again", Verb::Key("w"), "radio", "Week"),
    step("f reads the fortnight", Verb::Key("f"), "radio", "Fortnight"),
    step("Enter opens the kind the keys are on", Verb::Key("enter"), "table", "Jobs hit by Main thread ran past 300k context"),
    step("A Job with no title shows its short id", Verb::Look, "row", "5d1e9c42"),
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
