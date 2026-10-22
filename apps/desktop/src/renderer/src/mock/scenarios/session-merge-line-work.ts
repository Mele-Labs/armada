// `session-merge-line` with the Job on a workflow, as every Job is now, so the Cockpit draws its Flight control and the pull
// requests are rows of the Artifacts layer. The walk `session-pr-merge-line` plays it; `session-pr-merge-line.test.tsx` stays on the Job without one.

import { onAWorkflow } from "../sessions/on-a-workflow";
import { s205SessionMergeLine } from "./session-merge-line";

export const s205SessionMergeLineWork = onAWorkflow(s205SessionMergeLine, "session-merge-line-work", "A Job on a workflow whose pull request is in the merge queue, with others ahead and behind it", ["s3"]);
