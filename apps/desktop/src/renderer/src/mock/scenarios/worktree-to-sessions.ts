// The `sessions` scenario with Release notes script working to a workflow, as every Job Fleet hosts does, so it opens as a
// Job does on the Cockpit: its slot in the Bays layer and not on a ledger. The walk `worktree-to-sessions` plays it.

import { onAWorkflow } from "../sessions/on-a-workflow";
import { s200Sessions } from "./sessions";

export const s318WorktreeToSessions = onAWorkflow(s200Sessions, "worktree-to-sessions", "From a worktree to the Job that holds it, open on the Cockpit", ["s2"]);
