// A Job on the feature workflow at Refine whose agent is idle, held on a question for him, for the walk `agentRowRefine`: with no calls to read, the
// agent row in Now names what the Job waits on rather than the bare word. Over the same Job as `skip-a-step`.

import type { WaitingItem } from "@armada/protocol";

import { AUDIT_FILES, BUSY, ROTATION, approved, auditLedger } from "../audit-job";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"];

const ASK: WaitingItem = {
  id: "refine-ask-1",
  text: "Keep the nightly rotation at midnight?",
  since: new Date().toISOString(),
  source: "task_done",
  options: [{ label: "Keep" }, { label: "Change" }],
  act: { kind: "answer", target: "refine-ask-1" },
};

export const s337AgentRowRefine: Scenario = {
  ...base,
  name: "agent-row-refine",
  says: "A Job at Refine whose idle agent row names what it waits on",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => ({
        ...auditLedger(one, { steps: FLOW, at: 4 }, [approved(ROTATION, "2026-10-26T13:58:00.000Z"), approved(AUDIT_FILES, "2026-10-26T14:06:00.000Z")], [ASK]),
        turn: { state: "idle" },
        standing: "idle",
      }));
      return store;
    },
  },
};
