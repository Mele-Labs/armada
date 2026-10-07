// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`. Steps added to
// one Job are here too and answer as a Fleet that is not connected, until a surface draws them.

import type { AddedStepsApi } from "../../../../shared/api/added-steps";
import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Slice } from "../fake-context";
import { triggersServed } from "../triggers-fleet";

const none = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export const triggers: Slice<TriggersApi & AddedStepsApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: () => ({ ...triggersServed(), addJobStep: async () => none, removeJobStep: async () => none }),
};
