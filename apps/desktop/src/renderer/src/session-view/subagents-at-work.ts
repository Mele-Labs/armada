// The subagents a Session has at work, for Progress to list where there is no plan to show them. Read off the same `subagent` attachments the
// Subagents layer lists and the Field draws, and only the running ones: a finished one is in the layer.

import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

export type SubagentAtWork = { id: string; task: string; agentType: string | undefined };

export const subagentsAtWork = (session: Session): SubagentAtWork[] =>
  attachmentsOf(session, "subagent").flatMap((one) => (one.state === "running" ? [{ id: one.id, task: one.task, agentType: one.agentType }] : []));
