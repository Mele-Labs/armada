// The Now panel beside a Job's Overview canvas: a Drone at work with its output open, Checks
// landing with their quick acts, a Plan decision asked one at a time, a Judge reading, an issue
// with its fixes, each reason a Job runs nothing, and the sentence a Job with no reason reads.
// The canvas keeps the steps the rows belong to lit and stands the rest back. Over the
// `job-now-panel` scenario.

import { button, card, inside, region, role, tab, text, walk } from "../walk";

const now = region("Now");
const board = button("Overview", { exact: true });

const jobNowPanel = walk("job-now-panel", [
  { press: role("option", "Tidy the store fixtures"), say: "A Job the panel has nothing for" },
  { look: inside(now, text("Nothing is actively running on this job")), say: "It says so plainly. A Job should always show something, so this reads as a bug" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Pin the store clock"), say: "A Job with one Drone at work" },
  { look: inside(now, role("heading", "Running")), say: "Running, with a header for the kind of row" },
  { look: inside(now, text("Drones")), say: "Drones" },
  { look: inside(now, role("img", "Running")), say: "The mark breathes, and its tooltip names it" },
  { look: inside(now, text(/Read crates\/store\/src\/writer.rs/)), say: "One Drone, so its live output is already open" },
  { look: card("Write tests"), say: "The canvas dims the steps nothing in the panel belongs to, and keeps Implement lit" },
  { press: inside(now, role("button", /Open Implement Drone/)), say: "A press opens the Drone's panel" },
  { press: tab("Overview"), say: "Back to the Overview" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Cache the manifest read"), say: "A Job whose Checks are landing" },
  { look: inside(now, text("Checks")), say: "Checks, under their own header" },
  { look: inside(now, role("img", "Passed")), say: "A Check that passed" },
  { look: inside(now, role("img", "Failed")), say: "and one that failed, each marked as it finished" },
  { look: inside(now, button("Retry now")), say: "A Check can be retried now" },
  { look: inside(now, button("Skip check")), say: "or skipped" },
  { look: inside(now, button("Skip all checks")), say: "and every Check can be skipped from the band" },
  { press: inside(now, role("button", /Open lint, failed/)), say: "A press opens the Check" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Split the writer from the clock"), say: "A Job with a Plan decision open" },
  { look: inside(now, role("heading", "Asks you")), say: "Asks you" },
  { look: inside(now, text("Plan question")), say: "The decision has its header" },
  { look: inside(now, role("radiogroup", "Split the clock out of the writer, or wrap it in place?")), say: "The first decision, nothing preselected" },
  { look: inside(now, role("list", "Decisions")), say: "Marks for where it stands among the decisions" },
  { press: inside(now, role("radio", "Wrap it in place")), say: "Pick an answer" },
  { press: inside(now, button("Next")), say: "Next goes to the following decision" },
  { press: inside(now, role("radio", "Add a fake clock")), say: "The second" },
  { press: inside(now, button("Next")), say: "and the third" },
  { press: inside(now, role("radio", "Take it here")), say: "The last carries the answer" },
  { press: inside(now, button("Answer")), say: "and sends all three" },
  { look: inside(now, text("Judge question")), say: "Under their own headers, the questions of a Judge" },
  { look: inside(now, text("Drone question")), say: "and of a Drone" },
  { look: inside(now, role("button", /Judge on Implement, asks you/)), say: "A Judge's question is a row that opens the Judge" },
  { look: inside(now, role("button", /Implement Drone, asks you/)), say: "and a Drone's, the Drone" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Cap the retry backoff"), say: "A Job with a Judge reading" },
  { look: inside(now, role("button", /Judge on Implement, .*running/)), say: "The Judge, running" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Shorten the reconnect wait"), say: "A Job with an issue" },
  { look: inside(now, role("heading", "Issues")), say: "Issues" },
  { look: inside(now, role("img", "Drone stuck")), say: "A stuck Drone" },
  { look: inside(now, button("Redirect Drone")), say: "which can be redirected" },
  { look: inside(now, button("Retry step")), say: "or its step retried" },
  { look: inside(now, role("img", "Check failed")), say: "and a failed Check, with retry and skip" },
  { press: inside(now, button("Hide now")), say: "The head's button hides the panel" },
  { press: button("Show now"), say: "and a button beside the cards shows it again" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Index the record files"), say: "A Job that runs nothing because it waits for a resource" },
  { look: inside(now, role("heading", "Waiting")), say: "Waiting" },
  { look: inside(now, text("Worktree slot")), say: "The resource, named" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Move the store reads"), say: "A Job waiting on other Jobs" },
  { look: inside(now, role("button", /Job Pin the store clock/)), say: "Each Job it waits on, a row that opens it" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Rename the gate verbs"), say: "A Job between two steps" },
  { look: inside(now, text("Plan to Implement")), say: "The steps changing over, named" },
  { press: board, say: "Back to the Board" },

  { press: role("option", "Rebuild the fixtures"), say: "A Job running a one-off step" },
  { look: inside(now, text("Handoff, a one-off step")), say: "The step, shown" },
]);

export { jobNowPanel as "job-now-panel" };
