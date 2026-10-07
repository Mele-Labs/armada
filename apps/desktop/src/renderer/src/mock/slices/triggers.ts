// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`, and a failed
// Trigger's repair, answered from `repair-fleet.ts`.

import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Slice } from "../fake-context";
import { repairServed } from "../repair-fleet";
import { triggersServed } from "../triggers-fleet";

export const triggers: Slice<TriggersApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: (_scenario, fleet) => ({ ...triggersServed(), ...repairServed(fleet) }),
};
