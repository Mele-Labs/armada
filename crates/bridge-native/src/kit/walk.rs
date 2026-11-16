//! The `kit` walk: Kit in the strip, a piece picked, the project's switch, a piece a plugin brought, and a removed
//! skill coming back.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const LOCK: &str = "Comes with the design-kit plugin. Remove the plugin under Plugins.";

pub static STEPS: [Step; 11] = [
    step("Kit in the strip: its kinds down the rail", Verb::PressOn("tab", "Kit"), "listbox", "Kinds"),
    step("Pieces grouped by where they come from: yours, a plugin's, this repository's", Verb::Look, "heading", "design-kit"),
    step("A piece: a lamp for whether it is on in the project, its name and what it says", Verb::PressOn("option", "review-pr"), "heading", "review-pr"),
    step("The file, with line numbers", Verb::Look, "region", "File"),
    step("Edit and Open wait on Fleet's file operations", Verb::Hover, "button", "Edit review-pr"),
    step("Off in the project: the lamp goes hollow", Verb::PressOn("switch", "In armada"), "img", "Off in armada"),
    step("A piece a plugin brought has no Remove: a lock says why", Verb::PressOn("option", "frontend-design"), "img", LOCK),
    step("Remove a skill of your own: it stays, amber, with Restore", Verb::PressOn("option", "pdf-tools"), "button", "Remove pdf-tools"),
    step("Removed", Verb::Press, "button", "Restore pdf-tools"),
    step("Restore: it is back, as it was", Verb::Press, "option", "pdf-tools"),
    step("Models, Prompts and Workflows open in Settings", Verb::Look, "button", "Models in Settings"),
];
