// A Job mid-build whose agent named a plan group in each subagent's brief, for the walk `subagents-on-plan-groups`: two subagents at work and one finished, each on
// the group it was named for, and one that named none, on no group. Over the `field-expand-plan` scenario, with its busy Session given the four subagents.

import type { Scenario } from "../moment";
import { BUSY } from "../audit-job";
import { s274FieldExpandPlan as base } from "./field-expand-plan";

export const s348SubagentsOnPlanGroups: Scenario = {
  ...base,
  name: "subagents-on-plan-groups",
  says: "A Job mid-build with two subagents at work and one finished, each on the plan group its brief named, and one that named none",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => ({
        ...one,
        attachments: [
          ...one.attachments,
          {
            kind: "subagent",
            id: "agent-protocol",
            task: "Mirror it in the TypeScript types",
            state: "running",
            agentType: "protocol-engineer",
            call: "toolu_protocol",
            group: "f2",
          },
          {
            kind: "subagent",
            id: "agent-reply",
            task: "Add rotated_at to the audit reply",
            state: "done",
            report: "The audit reply carries rotated_at.",
            agentType: "rust-engineer",
            call: "toolu_reply",
            group: "f2",
          },
          {
            kind: "subagent",
            id: "agent-bridge",
            task: "List rotated files in System",
            state: "running",
            agentType: "bridge-engineer",
            call: "toolu_bridge",
            group: "f3",
          },
          {
            kind: "subagent",
            id: "agent-explorer",
            task: "Find where the audit log is opened",
            state: "running",
            agentType: "armada:explorer",
            call: "toolu_explorer",
          },
        ],
      }));
      return store;
    },
  },
};
