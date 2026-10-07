// Triggers' members: a Fleet that holds none, until a surface draws them and a fixture is written.

import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Slice } from "../fake-context";

const none = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export const triggers: Slice<TriggersApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: () => ({
    readTriggers: async () => ({ ok: true, triggers: [], left_out: [] }),
    readTrigger: async () => none,
    saveTrigger: async () => none,
    removeTrigger: async () => none,
  }),
};
