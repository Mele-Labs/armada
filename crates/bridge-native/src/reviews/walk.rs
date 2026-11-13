//! The `reviews` walk: back to a pull request he reviewed, what changed since he looked, a critique
//! of just that, and the page's panes arranged and kept as a layout.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

const ROW: &str = "#2198 Retry failed webhooks with backoff";
const SUGGESTION: &str = "Suggestion: Jitter can push the wait past the cap";
const INVOICE: &str = "#2190 Split the invoice job into stages";
const SUMMARY: &str = "Tobias now adds up to 250 ms of jitter to each retry wait and caps retries at five. Each retry is logged with its attempt.";
const SUMMARY_INVOICE: &str = "Mira now names the failing stage in the invoice job's log. Nothing else changed.";
const HOW_IT_WORKS: &str = "How it works: A failed webhook waits half a second before its first retry, and the wait doubles each time up to thirty seconds. Up to 250 ms of jitter is added to each wait, so clients that failed together do not retry together. The queue keeps a fixed number of retries and drops its oldest once it is full.";

pub static STEPS: [Step; 45] = [
    step("Reviews in the strip. The inbox reads the forge again, and keeps the last one drawn meanwhile", Verb::PressOn("tab", "Reviews"), "status", "Refreshing"),
    step("The pull request open last time waits for its fresh page before it opens", Verb::Look, "status", "#2190 opens once the refresh confirms it"),
    step("Each row: its title, then its number, repository, the commits new since you looked, and its age", Verb::Look, "label", "#2198 · ledger-api · 3 new commits · 40m"),
    step("Checks draw a glyph at the row's end only when they fail or run, and its tooltip says which", Verb::Hover, "img", "Checks failed"),
    step("Open it. Its page is read first", Verb::PressOn("row", ROW), "status", "Reading #2198"),
    step("Read: it opens on What changed, from the head you last saw to the new one", Verb::Later(1600), "heading", "Since you looked yesterday at 16:40"),
    step("A summary of the new commits is written as it opens", Verb::Look, "status", "Writing a summary of the new commits"),
    step("Written: what changed, in a paragraph", Verb::Later(2600), "label", SUMMARY),
    step("Your review, pinned under the title: you commented yesterday, and what came since", Verb::Look, "status", "You commented yesterday"),
    step("New commits, the files they touched, replies to your comments, and checks", Verb::Look, "table", "What changed"),
    step("Diff since you looked: Files, on just that range", Verb::PressOn("button", "Diff since you looked"), "radio", "Since you looked"),
    step("Whole PR: every file, the viewed ones folded", Verb::Press, "radio", "Whole PR"),
    step("Tick a file viewed: it folds away", Verb::Press, "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("Press anywhere on its row: it opens again, still ticked", Verb::PressOn("button", "src/webhooks/queue.rs"), "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("⌥1 brings What changed forward again", Verb::Key("alt-1"), "button", "Critique these changes"),
    step("Critique these changes asks first, as every run does", Verb::Open("button", "Critique these changes"), "dialog", "Critique these changes"),
    step("Start it. Review comments comes forward while it reads the range", Verb::PressOn("button", "Start critique"), "status", "Critique running"),
    step("Its suggestions, each in a box you can edit", Verb::Later(3200), "article", SUGGESTION),
    step("Change its words before they go", Verb::Type(" Clamp after adding it."), "textbox", SUGGESTION),
    step("Add as draft comment: your words become a draft on that line", Verb::OpenIn(SUGGESTION, "Add as draft comment"), "textbox", "Draft comment on src/webhooks/retry.rs:9"),
    step("Your review counts it among the drafts that go with it", Verb::Look, "label", "1 draft"),
    step("Tools: the review tools, each with when it last ran", Verb::PressOn("tab", "Tools"), "row", "How it works"),
    step("Run How it works", Verb::PressOn("button", "Run How it works"), "button", "Run How it works"),
    step("Its answer reads under its row", Verb::Later(2400), "article", HOW_IT_WORKS),
    step("Run Possible bugs", Verb::PressOn("button", "Run Possible bugs"), "button", "Run Possible bugs"),
    step("What it finds comes to Review comments as a suggestion", Verb::Later(2400), "article", "Suggestion: A high attempt number panics"),
    step("Drag Comments to the pane's bottom edge", Verb::Carry("tab", "Comments", 50, 92), "region", "Pane: Review comments, Comments, Tools"),
    step("Let go: the pane splits, Comments under the rest", Verb::Release, "region", "Pane: Comments"),
    step("Layout in the title bar", Verb::Open("button", "Layout: Default"), "menuitem", "Save as"),
    step("Save as", Verb::PressOn("menuitem", "Save as"), "dialog", "Save this layout"),
    step("A name for it", Verb::Typed("Side by side"), "textbox", "Layout name"),
    step("Saved: every pull request opens on it", Verb::PressOn("button", "Save layout"), "button", "Layout: Side by side"),
    step("Review changes: the sheet takes the panes' place", Verb::Open("button", "Review changes"), "region", "Review changes"),
    step("The draft that will be sent, and a summary", Verb::Look, "textbox", "Draft on src/webhooks/retry.rs:9"),
    step("Approve: the bar says so at once, at the head", Verb::PressOn("button", "Approve"), "status", "You approved today"),
    step("The inbox row has no new commits now", Verb::Look, "label", "#2198 · ledger-api · 40m"),
    step("The comment that went with it folds into one line, so it cannot go twice. Press it to read what went", Verb::PressOn("button", "Sent with your review today"), "button", "Sent with your review today"),
    step("Split the invoice job opens on What changed: since you looked, in words", Verb::PressOn("row", INVOICE), "heading", "Since you looked on Tuesday at 14:10"),
    step("Its summary is written as it opens", Verb::Later(2600), "label", SUMMARY_INVOICE),
    step("A new commit: its title, who made it and when", Verb::Look, "row", "Name each stage in the log"),
    step("Checks: one still running, the rest passed", Verb::Look, "row", "test: Running, 3 min"),
    step("Its glyph says the state in a tooltip", Verb::Hover, "img", "Running"),
    step("Your draft from an earlier visit waits in its own box", Verb::Look, "textbox", "Draft comment on src/jobs/invoice.rs:41"),
    step("What went with Tuesday's review, folded", Verb::Hover, "button", "Sent with your review on Tuesday"),
    step("The last critique says what it read in words; the commits are in its tooltip", Verb::Hover, "heading", "Suggestions on Mira's changes up to your review"),
];
