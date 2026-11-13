//! The `reviews` walk: back to a pull request he reviewed, what changed since he looked, a critique
//! of just that, and the page's panes arranged and kept as a layout.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const ROW: &str = "#2198 Retry failed webhooks with backoff";
const FINDING: &str = "Finding: Jitter can push the wait past the cap";
const INVOICE: &str = "#2190 Split the invoice job into stages";

pub static STEPS: [Step; 34] = [
    step("Reviews in the strip. The inbox reads the forge again, and keeps the last one drawn meanwhile", Verb::PressOn("tab", "Reviews"), "status", "Refreshing"),
    step("The pull request open last time waits for its fresh page before it opens", Verb::Look, "status", "#2190 opens once the refresh confirms it"),
    step("Changed since you looked: three new commits on this one", Verb::Look, "label", "+3 commits"),
    step("Beside each, where your review stands, in a tooltip", Verb::Hover, "img", "Your review: Changes requested"),
    step("Open it. Its page is read first", Verb::PressOn("row", ROW), "status", "Reading #2198"),
    step("Read: it opens on What changed, from the head you last saw to the new one", Verb::Later(1600), "heading", "Since you looked yesterday at 16:40"),
    step("Your review, pinned under the title: you commented yesterday, and what came since", Verb::Look, "status", "You commented yesterday"),
    step("New commits, the files they touched, replies to your comments, and checks", Verb::Look, "table", "What changed"),
    step("Diff since you looked: Files, on just that range", Verb::PressOn("button", "Diff since you looked"), "radio", "Since you looked"),
    step("Whole PR: every file, the viewed ones folded", Verb::Press, "radio", "Whole PR"),
    step("Tick a file viewed: it folds away", Verb::Press, "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("Press anywhere on its row: it opens again, still ticked", Verb::PressOn("button", "src/webhooks/queue.rs"), "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("⌥1 brings What changed forward again", Verb::Key("alt-1"), "button", "Critique these changes"),
    step("Critique these changes asks first, as every run does", Verb::Open("button", "Critique these changes"), "dialog", "Critique these changes"),
    step("Start it. Findings comes forward while it reads the range", Verb::PressOn("button", "Start critique"), "status", "Critique running"),
    step("Its findings, each on one line with what to do", Verb::Later(3200), "article", FINDING),
    step("Add as draft: the comment becomes a draft on that line", Verb::OpenIn(FINDING, "Add as draft"), "status", "Drafted"),
    step("Your review counts it among the drafts that go with it", Verb::Look, "label", "1 draft"),
    step("Drag Comments to the pane's bottom edge", Verb::Carry("tab", "Comments", 50, 92), "region", "Pane: Findings, Comments"),
    step("Let go: the pane splits, Comments under the rest", Verb::Release, "region", "Pane: Comments"),
    step("Layout in the title bar", Verb::Open("button", "Layout: Default"), "menuitem", "Save as"),
    step("Save as", Verb::PressOn("menuitem", "Save as"), "dialog", "Save this layout"),
    step("A name for it", Verb::Typed("Side by side"), "textbox", "Layout name"),
    step("Saved: every pull request opens on it", Verb::PressOn("button", "Save layout"), "button", "Layout: Side by side"),
    step("Review changes: the sheet takes the panes' place, at the head", Verb::Open("button", "Review changes"), "region", "Review changes at e71b0d4"),
    step("The draft that will be sent, and a summary", Verb::Look, "textbox", "Draft on src/webhooks/retry.rs:9"),
    step("Approve: the bar says so at once, at the head", Verb::PressOn("button", "Approve"), "status", "You approved today"),
    step("Nothing new since, so Review changes steps back; the inbox row says Approved", Verb::Look, "img", "Your review: Approved"),
    step("The finding that went with it says Sent, so it cannot go twice", Verb::Look, "status", "Sent"),
    step("Split the invoice job, read during the refresh, opens on What changed: since you looked, in words", Verb::PressOn("row", INVOICE), "heading", "Since you looked on Tuesday at 14:10"),
    step("A new commit: its title, who made it and when", Verb::Look, "row", "Name each stage in the log"),
    step("Checks: one still running, the rest passed", Verb::Look, "row", "test: Running, 3 min"),
    step("Its glyph says the state in a tooltip", Verb::Hover, "img", "Running"),
    step("The last critique says what it read in words; the commits are in its tooltip", Verb::Hover, "heading", "Critique of Mira's changes up to your review"),
];
