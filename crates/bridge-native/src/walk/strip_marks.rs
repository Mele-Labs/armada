//! The strip-marks walk: the merge queue strip, a Job's context ring with Compact, and the write-clash
//! mark, each step going straight to a state. The strip's scenes are `merge_strip/scenes.rs`, the
//! clashes' `clash_mark/scenes.rs` and the ring's `stand_in.rs`; a step that names one changes the
//! data a screen would be handed and presses nothing.
//!
//! The ring stands in a bar named for the Job head, since the head is another Job's: see
//! [`super::stand_in`].

use gpui::App;

use super::{marks, press_on, step, Step, Verb};

pub(super) const CALLING: &str = "#3381 Checkout page loads twice · armada/checkout-twice · ci failed · Job: Checkout page loads twice";
pub(super) const RUNNING: &str = "#3392 Quiet the write-clash notices · armada/quiet-clashes · In the merge queue, running its checks, place 2 · Job: Quiet the write-clash notices";
pub(super) const READY: &str = "#3388 Fix the merge line freshness chip · armada/merge-line-chip · ci passed · Job: Fix the merge line freshness chip";
pub(super) const RED: &str = "Main is red in armada, checkout 3 commits behind main";
pub(super) const PAIR: &str = "Conflicts with Quiet the write-clash notices · crates/bridge-native/src/jobs.rs, crates/ipc/src/clashes.rs";
pub(super) const SEVERAL: &str = "Conflicts with Quiet the write-clash notices, main, Pocket sign-in on the phone";
pub(super) const FILES: &str = "Conflicts with Quiet the write-clash notices · crates/bridge-native/src/jobs.rs, crates/bridge-native/src/clash_mark.rs, crates/ipc/src/clashes.rs, crates/fleet/src/write_clashes/detect.rs, docs/concepts/write-clashes.md";
pub(super) const PAUSED: &str = "Conflicts with Native Bridge foundation · crates/bridge-native/src/main.rs";

pub static STEPS: [Step; 41] = [
    marks("The strip stands along the bottom edge: the merge button, main's light, and the forge's queue in one track, each led by its place, then the rest", "strip-rest", "region", "Merge queue"),
    step("A ring says a pull request is open and a filled lamp that a queue holds it; the hue is its checks. Rest the pointer on a chip for its card", Verb::Hover, "button", RUNNING),
    step("A press selects the pull request, and the chip says so in the same frame", Verb::Press, "button", READY),
    marks("A call nobody holds pulses the chip it is about", "strip-calling", "button", CALLING),
    marks("A forge that stopped answering dims main's lamp and an unplugged glyph says since when", "strip-quiet", "img", "Main is green"),
    marks("Main red: the light says where, and a held checkout marks it", "strip-red", "button", RED),
    press_on("A press keeps the card open: what failed, what broke it, the checkout, and how old the reading is", "button", RED, "region", "main"),
    press_on("Dispatch a new Job is pending in the same frame", "button", "Dispatch a new Job", "button", "Dispatch a new Job"),
    press_on("Send back, beside the pull request that broke main, asks which Job gets the work, the one that merged it chosen, with what goes with it and a Message", "button", "Send back", "dialog", "Send the work back"),
    step("Esc shuts it and the keys go back to the light", Verb::Key("escape"), "button", RED),
    marks("Another machine's claim on the red main: who holds it and for how long, and Take over", "strip-red-elsewhere", "region", "main"),
    press_on("Take over asks once, naming the machine that holds it", "button", "Take over", "dialog", "Take over from Nicks-MacBook-Pro"),
    step("Esc shuts it and the keys go back to the strip", Verb::Key("escape"), "region", "Merge queue"),
    marks("A Job that took the red main shows on the card in place of the button", "strip-red-taken", "region", "main"),
    marks("Until Fleet answers, the line is placeholder chips", "strip-loading", "region", "Merge queue"),
    marks("On All the strip stacks one row for each repository and shows one; a main turning red brings its row in", "strip-all", "button", "Main is red in sample-api"),
    step("Q puts the keys on the strip", Verb::Key("q"), "region", "Merge queue"),
    step("↑ moves to the row before it", Verb::Key("up"), "img", "Main is green in sample-web"),
    marks("A Job's context ring, the stand-in for the Job head: the window filled clockwise, a tick at the autocompact threshold", "ring-normal", "button", "Context 63%"),
    step("Hover opens the card: each category, the heaviest memory files and MCP servers, and Compact", Verb::Hover, "button", "Context 63%"),
    press_on("Compact is pending in the same frame", "button", "Compact", "button", "Compact"),
    marks("A Compact that was refused is put back, with the reason", "ring-refused", "alert", "Compact not sent: the agent is not taking messages"),
    marks("From 80 the ring and the card's top edge turn amber", "ring-warning", "button", "Context 84%"),
    marks("From 95 they turn red, and Compact is off with a reason", "ring-danger", "button", "Compact"),
    step("The reason is the tooltip", Verb::Hover, "button", "Compact"),
    marks("With no breakdown the ring opens a small popover instead", "ring-popover", "button", "Context 63%"),
    step("A press opens the fill in words and Compact", Verb::Press, "button", "Context 63%"),
    marks("With neither, the ring is a picture and its tooltip carries the tokens", "ring-tip", "img", "Context 63%"),
    marks("A write clash marks both rows of the pair, in the Conflicts column", "clash-pair", "button", PAIR),
    step("Hover names the other Job and the files that collide", Verb::Hover, "button", PAIR),
    press_on("A press opens the other Job, and the row it sits on stays as it was", "button", PAIR, "row", "Quiet the write-clash notices"),
    marks("Two Jobs that collide in five files wear the same mark", "clash-files", "button", FILES),
    step("Hover lists the files: the first three, the directory dim and the file name whole, and a chip for the rest", Verb::Hover, "button", FILES),
    press_on("The chip opens the other two and now reads Fewer", "button", "2 more", "button", "Fewer"),
    press_on("A path is a press: Open crates/ipc/src/clashes.rs asks for that file", "button", "Open crates/ipc/src/clashes.rs", "button", "Open crates/ipc/src/clashes.rs"),
    marks("Two Jobs that collide in five files wear the same mark", "clash-files", "button", FILES),
    step("Tab reaches the mark, and Tab again the first path, with its card still open", Verb::Keys(&["tab", "tab"]), "button", "Open crates/bridge-native/src/jobs.rs"),
    step("Enter on a path asks for it, and Esc puts the card away with the keys back on the mark", Verb::Keys(&["enter", "escape"]), "button", FILES),
    marks("A Job in several clashes wears one mark; against main alone the mark is a picture, with nothing of main to open", "clash-states", "img", SEVERAL),
    step("Hover lists each holder under one heading", Verb::Hover, "img", SEVERAL),
    step("The Paused scope: a paused Job keeps its mark beside the Paused mark, and the mark names the Job it clashes with", Verb::PausedScope, "button", PAUSED),
];

/// Move to a named scene: the strip, the clashes or the ring.
pub fn apply(name: &str, cx: &mut App) -> Result<(), String> {
    if !name.starts_with("ring-") {
        super::stand_in::hide(cx);
    }
    if name.starts_with("strip-") {
        crate::merge_strip::apply(name, cx)
    } else if name.starts_with("clash-") {
        if name == "clash-pair" {
            // The strip goes back to rest: a scene is a jump, not a continuation.
            crate::merge_strip::apply("strip-rest", cx)?;
        }
        crate::clash_mark::scenes::apply(name, cx)
    } else if name.starts_with("ring-") {
        super::stand_in::apply(name, cx)
    } else {
        Err(format!("no strip-marks scene named {name}"))
    }
}

/// What a walk on this screen starts from: the strip at rest, nothing clashing, a press on a mark
/// pressing the other Job's row.
pub fn enter(cx: &mut App) {
    crate::merge_strip::begin(cx);
    crate::clash_mark::scenes::begin(cx);
    crate::clash_mark::scenes::open_by_press(cx);
}

/// Another screen's walk: no strip, no marks, so one process playing two walks (takes) draws each as it is.
pub fn leave(cx: &mut App) {
    crate::merge_strip::end(cx);
    crate::clash_mark::scenes::clear(cx);
}
