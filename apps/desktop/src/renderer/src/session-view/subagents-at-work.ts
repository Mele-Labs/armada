// The subagents a Session has at work, for Progress to list where there is no plan to show them: only the running ones, since a finished one is in the layer. And
// every subagent by the plan group the agent named in its brief, for the plan's rows. Read off the same `subagent` attachments the Subagents layer lists and the Field draws.

import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

export type SubagentAtWork = {
  id: string;
  task: string;
  agentType: string | undefined;
  group?: string;
};

export const subagentsAtWork = (session: Session): SubagentAtWork[] =>
  attachmentsOf(session, "subagent").flatMap((one) =>
    one.state === "running"
      ? [
          {
            id: one.id,
            task: one.task,
            agentType: one.agentType,
            ...(one.group === undefined ? {} : { group: one.group }),
          },
        ]
      : [],
  );

/** A subagent the agent named a plan group for: its task, and whether it is still running. */
export type SubagentOnGroup = {
  id: string;
  task: string;
  running: boolean;
};

/** Every subagent by the plan group the agent named for it, a fact Fleet kept, running or finished. One that named none is on no group. */
export const subagentsByGroup = (session: Session): ReadonlyMap<string, readonly SubagentOnGroup[]> => {
  const by = new Map<string, SubagentOnGroup[]>();
  for (const one of attachmentsOf(session, "subagent")) {
    if (one.group === undefined) continue;
    const placed = { id: one.id, task: one.task, running: one.state === "running" };
    by.set(one.group, [...(by.get(one.group) ?? []), placed]);
  }
  return by;
};
