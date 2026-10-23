// The workflow and its steps stand in a Workflow panel of their own, one line tall between the Field and Progress; Progress's band keeps its label and acts. The workflow is a picker the proposer chose, and a long workflow wraps under it. Over `one-session-view`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const progress = region("Progress", { exact: true });
const workflow = region("Workflow", { exact: true });

const progressWorkflowRow = walk("one-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Job open, its Field, Workflow and Progress in the right column" },
  { look: inside(workflow, role("list", "Workflow")), say: "The workflow's steps are a panel of their own between the Field and Progress" },
  { look: inside(workflow, role("combobox", "Workflow")), say: "Where the workflow can be changed it is a picker, with the proposer's mark beside it" },
  { hover: inside(workflow, role("img", "Plan · current")), say: "A step names itself and its state on hover" },
  { look: inside(progress, button("Ask for an update")), say: "Progress's band keeps its label and a few acts, and nothing else" },
]);

export { progressWorkflowRow as "progress-workflow-row" };
