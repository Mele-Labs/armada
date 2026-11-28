//! The `vitals` walk: the page found by name, tonight's stall read tile by tile, a limit stepped from the keys and its row in the
//! change log, put back with Reset, the spans, then the page while the reading has not come. A Job's own check of the screen and
//! its source of pictures; it is not shown to him.

use crate::walk::{Screen, Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb, target: Target { role, name } }
}

/// The page built again on a scenario of its own.
const fn fresh(say: &'static str, scenario: &'static str) -> Step {
    Step { id: None, say, verb: Verb::FreshIn(Screen::Reviews, scenario), target: Target { role: "tab", name: "Jobs" } }
}

pub static STEPS: [Step; 26] = [
    step("⌘K finds the page by name", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Vitals", Verb::Type("Vitals"), "searchbox", "Search actions, jobs and settings"),
    step("Vitals is an act, found by its name", Verb::Look, "option", "Vitals"),
    step("Enter opens the page on the last hour", Verb::Key("enter"), "region", "Vitals"),
    step("The chip says what the gate does with the next Check", Verb::Look, "status", "Checks held: memory pressure 2"),
    step("The span switch, the hour picked", Verb::Look, "radio", "1 hour"),
    step("Load first, its graph against the load limit", Verb::Look, "img", "Chart: Load"),
    step("Memory beside it, over its limit", Verb::Look, "img", "Memory: Over its limit"),
    step("Memory's pressure level against the level the gate holds at", Verb::Look, "img", "Chart: Memory pressure"),
    step("The settings that limit memory stand in its foot", Verb::Look, "row", "Memory to keep free"),
    step("Right moves the keys to Memory", Verb::Key("right"), "group", "Memory"),
    step("Down moves them to Compiler", Verb::Key("down"), "group", "Compiler"),
    step("Minus takes a core less from the compiler, and the pool line moves", Verb::Key("-"), "status", "Cores kept from the compiler: 3"),
    step("The change has its row in the log", Verb::Look, "row", "You, Cores kept from the compiler, 4 cores → 3 cores"),
    step("r puts back what ships", Verb::Key("r"), "status", "Cores kept from the compiler: 4"),
    step("The Load gate Job's own change, with its Undo", Verb::Look, "row", "Load gate, Load to hold Checks at, 0 → 24"),
    step("Tab goes into the tile's setting", Verb::Key("tab"), "button", "Fewer Cores kept from the compiler"),
    step("Esc comes back to the tile", Verb::Key("escape"), "group", "Compiler"),
    step("1 reads the last 15 minutes", Verb::Key("1"), "radio", "15 minutes"),
    step("4 reads the whole day", Verb::Key("4"), "radio", "24 hours"),
    step("Compile CPU, stacked by who started it", Verb::Look, "img", "Chart: Compile CPU by who started it"),
    fresh("The page while the reading has not come", "vitals-loading"),
    step("⌘K finds the page", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typed words find Vitals", Verb::Type("Vitals"), "searchbox", "Search actions, jobs and settings"),
    step("Vitals is an act, found by its name", Verb::Look, "option", "Vitals"),
    step("Every tile draws its shape first", Verb::Key("enter"), "img", "Loading"),
];
