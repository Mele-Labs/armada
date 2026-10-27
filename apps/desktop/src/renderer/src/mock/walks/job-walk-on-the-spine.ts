// A walk is an output of the Job, and its whole life is on the Job's story: pending in Next, shown in Now as a call on him, and approved in Completed. Over
// `job-walk-on-the-spine`, with the rest of the story's first round of notes on the way: Completed's items and toggle, the workflow readout, Field and the Jobs list.

import { button, deckRow, inside, region, role, walk } from "../walk";

const next = region("Next", { exact: true });
const now = region("Now", { exact: true });
const workflow = button("Workflow Feature", { exact: true });
const theWalk = inside(now, button("Audit files in System", { exact: true }));

export const jobWalkOnTheSpine = walk("job-walk-on-the-spine", [
  { press: deckRow("Rotate the audit log"), say: "Open a Job. The spine sits on dim glass, so the right column carries weight like the left" },
  { look: region("Completed", { exact: true }), say: "Done is now Completed. Each step lists what it made: issues, pull requests, documents and sketches" },
  { press: role("radio", "By kind", { exact: true }), say: "The toggle is two icons. By kind groups the same items by what they are" },
  { press: workflow, say: "The workflow's name is a readout at the head of the story. A press opens the old dropdown" },
  { key: "Escape", on: workflow, say: "Close it" },
  { look: inside(next, role("img", "Walk, not shown yet")), say: "A walk coming up sits in Next, under the step that will show it" },
  { later: now, say: "The Job finishes Build and shows its walk" },
  { look: theWalk, say: "Now it is a call on you, with a tick for each take" },
  { look: region("Field", { exact: true }), say: "Field has no picker. It follows the Job you select" },
  { look: region("Jobs", { exact: true }), say: "The dots are gone from the rows. A row that needs you keeps its tint and pulses" },
  { press: theWalk, say: "A press opens the walk window. Approving there moves it to Completed" },
]);
