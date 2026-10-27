// A walk is an output of the Job, and its whole life is on the Job's story: pending in Next, shown in Now as a call on him, and approved in Completed. Over
// `job-walk-on-the-spine`, with the story's walk notes on the way: Completed's items and toggle, the workflow readout, the Bays clash, the calls line, ⌥M, Field
// and the Jobs list; then Now on a real Job, whose plan, marks and calls are its own.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const next = region("Next", { exact: true });
const now = region("Now", { exact: true });
const thread = region("Thread", { exact: true });
const workflow = button("Workflow Feature", { exact: true });
const theWalk = inside(now, button("Audit files in System", { exact: true }));

export const jobWalkOnTheSpine = walk("job-walk-on-the-spine", [
  { press: deckRow("Rotate the audit log"), say: "Open a Job. The spine sits on dim glass, so the right column carries weight like the left" },
  { look: region("Completed", { exact: true }), say: "Completed lists what each step made: issues, pull requests, documents and sketches" },
  { press: role("radio", "By kind", { exact: true }), say: "The toggle is two icons. By kind groups the same items, and each group's header starts on Completed's edge" },
  { press: workflow, say: "The workflow's name is a readout at the head of the story. A press opens the old dropdown" },
  { key: "Escape", on: workflow, say: "Close it" },
  { look: button("Bays", { exact: true }), say: "A write clash is a dot in the Bays button's corner, clear of its icon" },
  { look: inside(now, text("Editing audit.rs")), say: "Now leads with what the Job is doing, worked out from its calls and not from its task marks" },
  { look: thread, say: "In the thread, each run of calls reads as one line: Read multiple files, Ran 3 checks, Watched deploy. Open one for its calls" },
  { key: "Alt+m", on: thread, say: "⌥M from anywhere in the Job" },
  { look: role("textbox", "Message", { exact: true }), say: "puts you in its message box" },
  { look: inside(next, role("img", "Walk, not shown yet")), say: "A walk coming up sits in Next, under the step that will show it" },
  { later: now, say: "The Job finishes Build and shows its walk" },
  { look: theWalk, say: "Now it is a call on you, with a tick for each take. A press opens it" },
  { look: region("Field", { exact: true }), say: "Field has no picker. It follows the Job you select" },
  { look: region("Jobs", { exact: true }), say: "The dots are gone from the rows. A row that needs you keeps its tint and pulses" },
  { press: deckRow("Issues tab beside Reviews"), say: "A real Job: the Issues tab, at call 396 of 676. Its plan, task marks and calls are its own" },
  { look: inside(now, text("Running multiple commands")), say: "Now leads with what its calls show it doing" },
  { hover: inside(now, text(/^\d+ h/)), say: "Ten marks say in progress. One nobody has changed for an hour dims, and the time says since when" },
  { look: inside(now, text("Full key map for the tab")), say: "Each mark in progress shows, with its own time" },
]);
