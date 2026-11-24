//! The `reviews` walk: the figure beside Reviews of what was asked of him, back to a pull request he reviewed, what changed since he looked, a review
//! of just that, a new pull request read by change set, the page's panes arranged and kept as a layout, and a row's menu: seen, ignored and back.

use crate::walk::{Step, Target, Verb};

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { id: None, say, verb, target: Target { role, name } }
}

const ROW: &str = "#2198 Retry failed webhooks with backoff";
const SUGGESTION: &str = "Suggestion: Jitter can push the wait past the cap";
const INVOICE: &str = "#2190 Split the invoice job into stages";
const SUMMARY: &str = "Tobias now adds up to 250 ms of jitter to each retry wait and caps retries at five. Each retry is logged with its attempt.";
const SUMMARY_INVOICE: &str = "Mira now names the failing stage in the invoice job's log. Nothing else changed.";
const STAGE: &str = "#2231 Say which stage of an invoice failed";
const RATE: &str = "#2214 Add rate limits to the export endpoint";
const BLOCKER: &str = "Blocker: The backfill locks invoices for minutes";
const RISK: &str = "Risk: Strict clients may reject the new field";
const KEPT: &str = "Kept by Armada for every PR that touches migrations/ in ledger-api. Read from docs/migrations.md on Tuesday, and read again when that doc changes.";
const HOW_IT_WORKS: &str = "How it works: The migration adds a stage column that may be empty, then copies each invoice's last failed stage from the job log in one update. The down migration drops the column.";

pub static STEPS: [Step; 80] = [
    step("Beside Reviews in the strip: the reviews asked of you by name that you have not opened", Verb::Hover, "status", "2 reviews asked of you, not opened"),
    step("Reviews in the strip. The pull request open last time opens at once, as you left it", Verb::PressOn("tab", "Reviews"), "heading", INVOICE),
    step("The inbox reads the forge again, and keeps the last one drawn meanwhile", Verb::Look, "status", "Refreshing"),
    step("It opens on its Overview. What the review writes on the commits since you looked waits in grey for the refresh", Verb::Look, "status", "Finding problems in the commits since you looked"),
    step("The fresh page lands in place: one commit since you looked", Verb::Lands, "label", "Since then: 1 commit"),
    step("What the review wrote on that commit lands on the Overview", Verb::Look, "article", "Possible bug: A stage with no name logs an empty field"),
    step("Each row: its title, then its number, repository, the commits new since you looked, and its age", Verb::Look, "label", "#2198 · ledger-api · 3 new commits · 40m"),
    step("Checks draw a glyph at the row's end only when they fail or run, and its tooltip says which", Verb::Hover, "img", "Checks failed"),
    step("Open it: its Overview first, with why Tobias made it", Verb::PressOn("row", ROW), "heading", "Why, from Tobias"),
    step("What changed: from the head you last saw to the new one", Verb::PressOn("tab", "What changed"), "heading", "Since you looked yesterday at 16:40"),
    step("A summary of the new commits is written as it opens", Verb::Look, "status", "Writing a summary of the new commits"),
    step("Written: what changed, in a paragraph", Verb::Lands, "label", SUMMARY),
    step("Your review, pinned under the title: you commented yesterday, and what came since", Verb::Look, "status", "You commented yesterday"),
    step("New commits, the files they touched, replies to your comments, and checks", Verb::Look, "table", "What changed"),
    step("Diff since you looked: Files, on just that range", Verb::PressOn("button", "Diff since you looked"), "radio", "Since you looked"),
    step("Whole PR: every file, the viewed ones folded", Verb::Press, "radio", "Whole PR"),
    step("Tick a file viewed: it folds away", Verb::Press, "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("Press anywhere on its row: it opens again, still ticked", Verb::PressOn("button", "src/webhooks/queue.rs"), "checkbox", "Viewed: src/webhooks/queue.rs"),
    step("⌥2 brings What changed forward again", Verb::Key("alt-2"), "button", "Review these changes"),
    step("Review these changes asks what to look for and how hard, with your last choice for this repository", Verb::Open("button", "Review these changes"), "dialog", "Review these changes"),
    step("Look for API breaking changes too", Verb::PressOn("checkbox", "API breaking changes"), "checkbox", "API breaking changes"),
    step("Thorough: the line under it says what that costs", Verb::PressOn("radio", "Thorough"), "label", "Several minutes, reads around each change"),
    step("Start: Review comments comes forward while it reads the range", Verb::PressOn("button", "Start"), "status", "Review running"),
    step("It says what you asked it to look for, and how hard", Verb::Look, "label", "Possible bugs, risks and API breaking changes, thorough"),
    step("Its suggestions, each in a box you can edit", Verb::Lands, "article", SUGGESTION),
    step("Change its words before they go", Verb::Type(" Clamp after adding it."), "textbox", SUGGESTION),
    step("Add as draft comment: your words become a draft on that line", Verb::OpenIn(SUGGESTION, "Add as draft comment"), "textbox", "Draft comment on src/webhooks/retry.rs:9"),
    step("Your review counts it among the drafts that go with it", Verb::Look, "label", "1 draft comment"),
    step("A pull request you have not opened yet opens on its Overview: why Mira made it", Verb::PressOn("row", STAGE), "heading", "Why, from Mira"),
    step("It was asked of you, so the figure beside Reviews drops to one", Verb::Hover, "status", "1 review asked of you, not opened"),
    step("Its change sets run down the rail, each with the glyph of its worst finding", Verb::Look, "listbox", "Change sets"),
    step("A blocker: its change set, what is wrong, and the line", Verb::Look, "article", BLOCKER),
    step("Add as draft comment: it joins your drafts, and the card says so", Verb::OpenIn(BLOCKER, "Add as draft comment"), "status", "Added as draft comment"),
    step("Your review counts it among the drafts that go with it", Verb::Look, "label", "1 draft comment"),
    step("Show in change set: the database migration in focus, at the blocker's hunk", Verb::OpenIn(BLOCKER, "Show in change set"), "article", "Suggestion: Batch the backfill"),
    step("The practices for migrations, kept by Armada from the repository's own doc", Verb::Look, "label", KEPT),
    step("One it misses: the backfill runs as one statement", Verb::Look, "row", "Missed: Backfill runs in batches. It runs as one statement."),
    step("Wrong beside a practice: say its result is wrong", Verb::Open("button", "Wrong: Has a down migration"), "menuitem", "Check is wrong"),
    step("Check is wrong: it asks why, in one line", Verb::PressOn("menuitem", "Check is wrong"), "textbox", "Why the check is wrong"),
    step("Say why", Verb::Typed("The down migration drops the column but loses the stages it held."), "textbox", "Why the check is wrong"),
    step("Save: the practice turns to missed, with what you said under it", Verb::PressOn("button", "Save reason"), "label", "You said: The down migration drops the column but loses the stages it held."),
    step("Its flag again: change the reason, or undo what you said", Verb::Open("button", "Wrong: Has a down migration"), "menuitem", "Undo"),
    step("Undo: the check's own result comes back, and what you said goes", Verb::PressOn("menuitem", "Undo"), "row", "Met: Has a down migration"),
    step("How it works", Verb::PressOn("button", "How it works"), "button", "How it works"),
    step("Its answer reads under it", Verb::Lands, "article", HOW_IT_WORKS),
    step("Next: Data layer", Verb::PressOn("button", "Next: Data layer"), "heading", "Data layer"),
    step("The rail follows it", Verb::Look, "option", "Data layer"),
    step("Overview in the rail", Verb::PressOn("option", "Overview"), "article", RISK),
    step("Not a problem: the card goes", Verb::OpenIn(RISK, "Not a problem"), "article", "Possible bug: A crash can retry forever"),
    step("The risk is gone from the Overview", Verb::Absent("article", RISK), "heading", "Blockers, risks and possible bugs"),
    step("Each change set's row says what is still found in it, in the glyph and hue of its worst finding", Verb::Wheel("heading", "Blockers, risks and possible bugs", 700), "label", "1 possible bug"),
    step("Back to Retry failed webhooks", Verb::PressOn("row", ROW), "heading", "Why, from Tobias"),
    step("Drag Comments to the pane's bottom edge", Verb::Carry("tab", "Comments", 50, 92), "region", "Pane: Review comments, Comments"),
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
    step("Split the invoice job opens on its Overview", Verb::PressOn("row", INVOICE), "heading", INVOICE),
    step("What changed: since you looked, in words", Verb::PressOn("tab", "What changed"), "heading", "Since you looked on Tuesday at 14:10"),
    step("Its summary, written when the fresh page landed", Verb::Look, "label", SUMMARY_INVOICE),
    step("A new commit: its title, who made it and when", Verb::Look, "row", "Name each stage in the log"),
    step("Checks: one still running, the rest passed", Verb::Look, "row", "test: Running, 3 min"),
    step("Its glyph says the state in a tooltip", Verb::Hover, "img", "Running"),
    step("Your draft from an earlier visit waits in its own box", Verb::Hover, "article", "Draft comment on src/jobs/invoice.rs:41"),
    step("What went with Tuesday's review, folded", Verb::Hover, "button", "Sent with your review on Tuesday"),
    step("The last review says what it read in words; the commits are in its tooltip", Verb::Hover, "heading", "Suggestions on Mira's changes up to your review"),
    step("A right press on a row offers Mark as seen and Ignore", Verb::RightOpen("row", RATE), "menuitem", "Mark as seen"),
    step("Mark as seen: the row dims and drops to the bottom of its group", Verb::PressOn("menuitem", "Mark as seen"), "row", RATE),
    step("Its mark says so in a tooltip. A new commit brings the row back", Verb::Hover, "img", "Seen at the latest commit"),
    step("The menu key or ⇧F10 opens the same menu under the row you are on. Mark as seen is gone", Verb::Key("shift-f10"), "menuitem", "Ignore"),
    step("Ignore: the row goes to Ignored, after Done and folded", Verb::PressOn("menuitem", "Ignore"), "button", "Ignored"),
    step("Open Ignored. Fleet brings a row back when you are asked to review it again", Verb::PressOn("button", "Ignored"), "row", RATE),
    step("On an ignored row the menu offers Stop ignoring", Verb::RightOpen("row", RATE), "menuitem", "Stop ignoring"),
    step("Stop ignoring: it goes back to Review requested, still seen", Verb::PressOn("menuitem", "Stop ignoring"), "row", RATE),
];
