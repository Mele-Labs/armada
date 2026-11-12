//! Running a command an agent showed: Run on a shell block of the thread, the run under the block with its
//! mark, Stop, Run again and Close; then a Needs you run item, its run under its row. The runs are
//! `terminal::shells::Mock`'s.

use super::{step, Step, Verb};

pub static STEPS: [Step; 18] = [
    step("A press on a Job opens it", Verb::Press, "row", "Fix the merge line freshness chip"),
    step("A press in the thread gives it the keys", Verb::Press, "region", "Thread"),
    step("The pointer on a shell block shows Run beside Copy", Verb::Hover, "button", "Run cargo test -p fleet"),
    step("Run runs it where the Job works, on a terminal under the block: it failed", Verb::Open("button", "Run cargo test -p fleet"), "img", "Failed"),
    step("A run that ended offers Run again", Verb::Look, "button", "Run again cargo test -p fleet"),
    step("Run again runs it once more, in place", Verb::Open("button", "Run again cargo test -p fleet"), "img", "Failed"),
    step("Close puts the run away; the block keeps its mark", Verb::Open("button", "Close cargo test -p fleet"), "button", "Show terminal: Failed"),
    step("The mark brings the run back", Verb::Open("button", "Show terminal: Failed"), "img", "Failed"),
    step("Run on another block stops the thread's run and draws this one under its block", Verb::Open("button", "Run pnpm dev"), "img", "Running"),
    step("While it runs the head offers Stop", Verb::Look, "button", "Stop pnpm dev"),
    step("Stop ends it", Verb::Open("button", "Stop pnpm dev"), "img", "Stopped"),
    step("A command that exits 0 passed", Verb::Open("button", "Run git status"), "img", "Passed"),
    step("Close puts it away", Verb::Open("button", "Close git status"), "button", "Show terminal: Passed"),
    step("What waits on you holds a command to run", Verb::Open("button", "Needs you"), "button", "Open Run the chip checks"),
    step("A press runs it where the Job works, its run under the row", Verb::Open("button", "Open Run the chip checks"), "img", "Passed"),
    step("Its output stays under the row", Verb::Look, "button", "Run again armada check test chip"),
    step("A press on the row once it passed does nothing", Verb::Open("button", "Open Run the chip checks"), "img", "Passed"),
    step("Close puts the item's run away", Verb::Open("button", "Close armada check test chip"), "button", "Open Run the chip checks"),
];
