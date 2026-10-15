// A bay's history: the wire is answered by Fleet alone, so the mock has none for it. A scenario that
// wants a bay's history carries its own answer.

import type { BayHistoryApi } from "../../../../shared/api/bay-history";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const bayHistory: Slice<BayHistoryApi, Record<string, never>> = {
  name: "bayHistory",
  state: {},
  api: () => ({
    getSlotHistory: async (manifestId, slot) => ({ ok: false, outcome: unanswered(`/slot_history?manifest_id=${manifestId}&slot=${slot}`) }),
  }),
};
