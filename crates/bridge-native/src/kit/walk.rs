//! The `kit` walk: Kit in the strip, a piece picked, the project's switch, a piece a plugin brought, its file edited, and a removed
//! skill coming back.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const LOCK: &str = "Comes with the design-kit plugin. Remove the plugin under Plugins.";

pub static STEPS: [Step; 17] = [
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
    step("A long file scrolls inside the pane, its long line wrapping, the numbers moving with it", Verb::PressOn("option", "long-notes"), "region", "File"),
    step("Page down: the pane reads on from a line further in", Verb::Keys(&["tab", "pagedown"]), "region", "File"),
    step("A server has no file: its facts stand in the pane", Verb::PressOn("option", "MCP servers"), "region", "Facts"),
    step("Models, Prompts and Workflows open in Settings", Verb::Look, "button", "Models in Settings"),
];
