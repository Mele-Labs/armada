//! The `reviews` walk: back to a pull request he reviewed, what changed since he looked, a critique
//! of just that, and the page's panes arranged and kept as a layout.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const ROW: &str = "#2198 Retry failed webhooks with backoff";
const FINDING: &str = "Finding: Jitter can push the wait past the cap";

pub static STEPS: [Step; 22] = [
    step("Reviews in the strip. The inbox reads the forge again, and keeps the last one drawn meanwhile", Verb::PressOn("tab", "Reviews"), "status", "Refreshing"),
    step("The pull request open last time waits for its fresh page before it opens", Verb::Look, "status", "#2190 opens once the refresh confirms it"),
    step("Changed since you looked: three new commits on this one", Verb::Look, "label", "+3 commits"),
    step("Open it. Its page is read first", Verb::PressOn("row", ROW), "status", "Reading #2198"),
    step("Read: it opens on What changed, from the head you last saw to the new one", Verb::Later(1600), "heading", "Since you looked, yesterday 16:40 · a3f9c21 → e71b0d4"),
    step("New commits, the files they touched, replies to your comments, and checks", Verb::Look, "table", "What changed"),
    step("Diff since you looked: Files, on just that range", Verb::PressOn("button", "Diff since you looked"), "radio", "Since you looked"),
    step("Whole PR: every file, the viewed ones folded", Verb::Press, "radio", "Whole PR"),
    step("Tick a file viewed: it folds away", Verb::Press, "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("⌥1 brings What changed forward again", Verb::Key("alt-1"), "button", "Critique these changes"),
    step("Critique these changes asks first, as every run does", Verb::Open("button", "Critique these changes"), "dialog", "Critique these changes"),
    step("Start it. Findings comes forward while it reads the range", Verb::PressOn("button", "Start critique"), "status", "Critique running"),
    step("Its findings, each on one line with what to do", Verb::Later(3200), "article", FINDING),
    step("Add as draft: the comment becomes a draft on that line", Verb::OpenIn(FINDING, "Add as draft"), "status", "Drafted"),
    step("+ adds a tab to this pane", Verb::PressWithin("region", "Pane: Findings, Comments", "Add tab"), "menuitem", "Review"),
    step("Review holds the drafts, a body and the verdict", Verb::PressOn("menuitem", "Review"), "textbox", "Draft on src/webhooks/retry.rs:9"),
    step("Drag Comments to the pane's bottom edge", Verb::Carry("tab", "Comments", 50, 92), "region", "Pane: Findings, Comments, Review"),
    step("Let go: the pane splits, Comments under the rest", Verb::Release, "region", "Pane: Comments"),
    step("Layout in the title bar", Verb::Open("button", "Layout: Default"), "menuitem", "Save as"),
    step("Save as", Verb::PressOn("menuitem", "Save as"), "dialog", "Save this layout"),
    step("A name for it", Verb::Typed("Side by side"), "textbox", "Layout name"),
    step("Saved: every pull request opens on it", Verb::PressOn("button", "Save layout"), "button", "Layout: Side by side"),
];
