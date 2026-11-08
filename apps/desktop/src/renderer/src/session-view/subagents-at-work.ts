// The subagents a Session has at work, for Progress to list where there is no plan to show them, and for the plan to show on the group the agent named in each one's
// brief. Read off the same `subagent` attachments the Subagents layer lists and the Field draws, and only the running ones: a finished one is in the layer.

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

/** The subagents at work by the plan group the agent named for each, a fact Fleet kept. One that named none is on no group. */
export const subagentsByGroup = (session: Session): ReadonlyMap<string, readonly SubagentAtWork[]> => {
  const by = new Map<string, SubagentAtWork[]>();
  for (const one of subagentsAtWork(session)) {
    if (one.group !== undefined) by.set(one.group, [...(by.get(one.group) ?? []), one]);
  }
  return by;
};
