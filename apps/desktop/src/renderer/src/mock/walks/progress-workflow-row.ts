// Progress's band holds only its label and acts; the workflow and its steps stand in a quiet row at the top of the body. Over `compact-session-view`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const progress = region("Progress", { exact: true });

const progressWorkflowRow = walk("compact-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Job open, Progress in the right column" },
  { look: inside(progress, button("Ask for an update")), say: "The band keeps its label and a few acts, and nothing else" },
  { look: inside(progress, role("list", "Workflow")), say: "The workflow's steps are one row under the band" },
  { hover: inside(progress, role("img", "Build · current")), say: "A step still names itself and its state on hover" },
]);

export { progressWorkflowRow as "progress-workflow-row" };
