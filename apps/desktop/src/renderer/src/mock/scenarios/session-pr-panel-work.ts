// `session-pr-panel` with the Job on a workflow, as every Job is now, so the Cockpit draws its Flight control and the pull
// requests are rows of the Artifacts layer. The walks `pullRequestPanel`, `pullRequestPanelNarrow` and `files-as-a-tree` play it;
// `session-pr-place.test.tsx` stays on the Job without one.

import { onAWorkflow } from "../sessions/on-a-workflow";
import { s236SessionPrPanel } from "./session-pr-panel";

export const s236SessionPrPanelWork = onAWorkflow(s236SessionPrPanel, "session-pr-panel-work", "A Job on a workflow whose pull requests are in the merge queue, ready to merge, and failed their checks", ["s3"]);
