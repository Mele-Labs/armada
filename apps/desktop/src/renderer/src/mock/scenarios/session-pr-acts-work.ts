// `session-pr-acts` with the Job on a workflow, as every Job is now, so the Cockpit draws its Flight control and the pull request
// is a row of the Artifacts layer. The walk `session-pr-acts` plays it; `session-pr-acts.test.tsx` stays on the Job without one.

import { onAWorkflow } from "../sessions/on-a-workflow";
import { s224SessionPrActs } from "./session-pr-acts";

export const s224SessionPrActsWork = onAWorkflow(s224SessionPrActs, "session-pr-acts-work", "A Job on a workflow waiting on the person to merge its pull request", ["s3"]);
