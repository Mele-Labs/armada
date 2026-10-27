// A walk is an output of the Job, and its whole life is on the Job's story: pending in Next, shown in Now as a call on him, and approved in Completed. Over
// `job-walk-on-the-spine`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const next = region("Next", { exact: true });
const now = region("Now", { exact: true });

export const jobWalkOnTheSpine = walk("job-walk-on-the-spine", [
  { press: deckRow("Rotate the audit log"), say: "A Job at Build. The workflow at the head of the story has a Present step still ahead" },
  { look: inside(next, role("img", "Walk, not shown yet")), say: "Next: the walk under the Present step it will be shown at, before it is shown" },
  { later: now, say: "The Job finishes Build and shows its walk" },
  { look: inside(now, button("Audit files in System")), say: "Now: the walk is a call on him. The lamp pulses, its name is the walk's, and a tick stands for each take. A press, or Enter, opens it as Bridge opens a walk window" },
  { press: inside(now, button("Audit files in System")), say: "The walk window opens over the app, with its takes. Approve there, and the walk moves to Completed as Walk approved, under Present" },
]);
