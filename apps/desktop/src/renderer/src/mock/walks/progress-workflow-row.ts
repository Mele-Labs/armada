// The workflow and its steps head the story as a readout, one line tall above Completed; Now's band keeps its label and acts. The workflow is a picker the proposer chose, and a long workflow wraps under it. Over `one-session-view`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const now = region("Now", { exact: true });

const progressWorkflowRow = walk("one-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Job open, its Completed, Now and Next in the right column" },
  { look: role("list", "Workflow"), say: "The workflow's steps head the story, above Completed" },
  { look: button(/^Workflow /), say: "Where the workflow can be changed its name is a picker, with the proposer's mark beside it" },
  { hover: inside(role("list", "Workflow"), role("img", "Plan · current")), say: "A step names itself and its state on hover" },
  { look: inside(now, button("Ask for an update")), say: "Now's band keeps its label and a few acts, and nothing else" },
]);

export { progressWorkflowRow as "progress-workflow-row" };
