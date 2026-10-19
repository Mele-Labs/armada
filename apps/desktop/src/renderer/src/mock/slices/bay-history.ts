// A bay's history: the wire is answered by Fleet alone, so the mock has none for it. A scenario that
// wants a bay's history carries its own answer (`draft.bayHistories`), which the side panel's bay reads.

import type { BayHistoryApi } from "../../../../shared/api/bay-history";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const bayHistory: Slice<BayHistoryApi, Record<string, never>> = {
  name: "bayHistory",
  state: {},
  api: (scenario) => ({
    getSlotHistory: async (manifestId, slot) => {
      const own = scenario.draft?.bayHistories?.find((one) => one.manifest_id === manifestId && one.slot === slot);
      return own === undefined ? { ok: false, outcome: unanswered(`/slot_history?manifest_id=${manifestId}&slot=${slot}`) } : { ok: true, value: own };
    },
  }),
};
