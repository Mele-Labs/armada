// Main red, and another machine holds the claim on it. The band names the holder and its age and
// offers Take over, which asks once, then drafts a Job on the brief and opens it.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { redHeldElsewhere, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

function heldElsewhere(): Scenario {
  const base = holding("red-held-elsewhere", "Main red, claimed on another machine: Take over, a stale claim");
  const { first, later } = redHeldElsewhere(repository().root);
  // Sessions served, so Take over has a Job to draft.
  const draft = { ...base.draft, sessions: () => sessionsStore() };
  return { ...base, state: { ...base.state, ...first }, later, draft, behaves: writingMainsLogs };
}

export const s245RedHeldElsewhere: Scenario = heldElsewhere();
