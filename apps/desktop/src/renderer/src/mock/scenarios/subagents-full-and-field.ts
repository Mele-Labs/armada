// An open Session with no plan and two subagents at work, neither of which has touched a file yet. Over the `one-session-view` scenario, with
// one of its Sessions given the subagents. The walk `subagentsFullAndField` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const QUIET = "one-view-2";

const working = (one: Session): Session => ({
  ...one,
  turn: { state: "working" },
  standing: "working",
  attachments: [
    ...one.attachments,
    { kind: "subagent", id: "sa-restart", task: "Harden the restart scripts", state: "running", agentType: "bridge-engineer" },
    { kind: "subagent", id: "sa-toast", task: "Failed refresh toast", state: "running", agentType: "bridge-engineer" },
  ],
  work: { proposal: { workflow: null } },
});

export const s280SubagentsFullAndField: Scenario = {
  ...base,
  name: "subagents-full-and-field",
  says: "A Session with no plan and two subagents at work: the Field and the Blueprint show them, and Subagents opens one full size",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(QUIET, working);
      return store;
    },
  },
};
