// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`.

import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Slice } from "../fake-context";
import { triggersServed } from "../triggers-fleet";

export const triggers: Slice<TriggersApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: () => triggersServed(),
};
