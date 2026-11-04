// Where Jobs' time went: the wire is answered by Fleet alone, so the mock has none for it unless a scenario brings its own
// (`behaves`), as the one under the Time walk does.

import type { UsageTimeApi } from "../../../../shared/api/usage-time";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const usageTime: Slice<UsageTimeApi, Record<string, never>> = {
  name: "usageTime",
  state: {},
  api: () => ({
    getUsageTime: async () => ({
      ok: false,
      outcome: unanswered("/usage/time"),
    }),
    getUsageHours: async () => ({
      ok: false,
      outcome: unanswered("/usage/hours"),
    }),
    getJobTime: async (sessionId) => ({
      ok: false,
      outcome: unanswered(`/sessions/time?session_id=${sessionId}`),
    }),
  }),
};
