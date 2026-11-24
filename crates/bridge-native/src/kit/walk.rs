//! The `kit` walk: Kit in the strip, a piece picked, the project's switch, a piece a plugin brought, its file edited, and a removed
//! skill coming back.

use crate::walk::{Screen, Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb, target: Target { role, name } }
}

const LOCK: &str = "Comes with the design-kit plugin. Remove the plugin under Plugins.";

pub static STEPS: [Step; 21] = [
    step("Kit in the strip: its kinds down the rail", Verb::PressOn("tab", "Kit"), "listbox", "Kinds"),
    step("Pieces grouped by where they come from: yours, a plugin's, this repository's", Verb::Look, "heading", "design-kit plugin"),
    step("A piece: a lamp for whether it is on in the project, its name and what it says", Verb::PressOn("option", "review-pr"), "heading", "review-pr"),
    step("The file, with line numbers", Verb::Look, "region", "File"),
    step("Edit turns the file into a text box", Verb::PressOn("button", "Edit review-pr"), "textbox", "File text"),
    step("A changed line: Save shows, beside Cancel", Verb::Typed("x"), "button", "Save"),
    step("Esc drops the edit", Verb::Key("escape"), "button", "Edit review-pr"),
    step("Off in the project: the lamp goes hollow", Verb::PressOn("switch", "In armada"), "img", "Off in armada"),
    step("A piece a plugin brought has no Remove: a lock says why", Verb::PressOn("option", "frontend-design"), "img", LOCK),
    step("A piece a plugin brought: Edit is off and says why", Verb::Hover, "button", "Edit frontend-design"),
    step("A skill of your own can be removed", Verb::PressOn("option", "pdf-tools"), "button", "Remove pdf-tools"),
    step("Remove: the folder goes to the Trash", Verb::PressOn("button", "Remove pdf-tools"), "button", "Restore pdf-tools"),
    step("Restore: it is back, as it was", Verb::PressOn("button", "Restore pdf-tools"), "option", "pdf-tools"),
    step("Further down the list, this repository's own", Verb::WheelTo("listbox", "Skills", 120), "option", "long-notes"),
    step("A long file scrolls inside the pane, its long line wrapping, the numbers moving with it", Verb::PressOn("option", "long-notes"), "region", "File"),
    step("Page down: the pane reads on from a line further in", Verb::Keys(&["tab", "pagedown"]), "region", "File"),
    step("An output style a plugin brought reads like any file", Verb::PressOn("option", "Output styles"), "heading", "armada plugin"),
    step("Armada's style: its file, with the plugin named in its group", Verb::PressOn("option", "Armada"), "region", "File"),
    step("MCP servers grouped by where they are configured", Verb::PressOn("option", "MCP servers"), "heading", "design-kit plugin"),
    step("A server has no file: its facts stand in the pane", Verb::PressOn("option", "docs"), "region", "Facts"),
    step("Models, Prompts and Workflows open in Settings", Verb::Look, "button", "Models in Settings"),
];

const fn part(say: &'static str, scenario: &'static str) -> Step {
    Step { id: None, say, verb: Verb::FreshIn(Screen::Reviews, scenario), target: Target { role: "tab", name: "Kit" } }
}

/// The `kit-sync` walk: Kit over the Mac link with no Mac linked, with every row the same, and with a row in every
/// mark and a clash, on `kit/mock.rs`.
pub static SYNC_STEPS: [Step; 20] = [
    part("No Mac linked", "kit-unlinked"),
    step("The bar says Kit crosses once a Mac is linked, and opens Account in Settings", Verb::PressOn("tab", "Kit"), "button", "Link a Mac in Settings"),
    step("Each kind has its switch in the rail: the choice is this Mac's", Verb::Look, "switch", "Cross Skills"),
    part("A linked Mac with every row the same", "kit-in-step"),
    step("In step: the Mac it crosses to and when its Kit last crossed", Verb::PressOn("tab", "Kit"), "heading", "Crosses to Work Mac"),
    step("Nothing differs, so the bar has no chip", Verb::Absent("img", "Clash over the Mac link"), "heading", "Crosses to Work Mac"),
    part("A linked Mac with a row in every mark and a clash", "kit-sync"),
    step("A clash opens in the detail: both versions side by side, each Mac's own lines tinted", Verb::PressOn("tab", "Kit"), "region", "Clash with Work Mac"),
    step("The bar's chip says something clashes, and names it", Verb::Hover, "img", "Clash over the Mac link"),
    step("Each row's mark in a fixed column: same, differs, new here, refused, only on Work Mac", Verb::Look, "img", "Only on Work Mac"),
    step("Each row has its own switch under its mark. Skills are on, so every one crosses", Verb::Look, "switch", "Cross deploy-notes"),
    step("Refused: its switch stays off, with Fleet's reason in the tooltip", Verb::Hover, "switch", "Cross tracker-sync"),
    step("Keep Work Mac's: its version is written here and the row reads the same", Verb::PressOn("button", "Keep Work Mac's"), "region", "File"),
    step("deploy-notes off: the rest keep crossing by themselves", Verb::PressOn("switch", "Cross deploy-notes"), "switch", "Cross deploy-notes"),
    step("Skills' switch in the rail now says some cross", Verb::Hover, "switch", "Cross Skills"),
    step("Pressed from some, Skills cross whole again", Verb::PressOn("switch", "Cross Skills"), "switch", "Cross deploy-notes"),
    step("Sub agents are off, with code-reviewer crossing by itself", Verb::PressOn("option", "Sub agents"), "switch", "Cross code-reviewer"),
    step("test-writer on: it crosses by itself too", Verb::PressOn("switch", "Cross test-writer"), "switch", "Cross test-writer"),
    step("Back in Skills", Verb::PressOn("option", "Skills"), "option", "pricing-review"),
    step("A row only on Work Mac has a switch too, and no file here: its facts say where it is", Verb::PressOn("option", "pricing-review"), "region", "Facts"),
];
