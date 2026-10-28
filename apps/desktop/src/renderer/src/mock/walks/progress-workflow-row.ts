// The workflow's name heads the story as a readout, one line tall, and its steps are the zones below it; the current step's band keeps its label and acts. The workflow is a picker the proposer chose, and a long workflow wraps under it. Over `one-session-view`.

import { button, deckRow, inside, region, walk } from "../walk";

const now = region("Plan", { exact: true });

const progressWorkflowRow = walk("one-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Job open, its Brief and a zone for each workflow step in the right column" },
  { look: now, say: "The workflow's steps are the zones of the story, the current one open" },
  { look: button(/^Workflow /), say: "Where the workflow can be changed its name is a picker, with the proposer's mark beside it" },
  { look: inside(now, button("Ask for an update")), say: "The current step's band keeps its label and a few acts, and nothing else" },
]);

export { progressWorkflowRow as "progress-workflow-row" };
