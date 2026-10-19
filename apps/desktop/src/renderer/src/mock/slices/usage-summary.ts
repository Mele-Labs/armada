// The usage summary: the wire is answered by Fleet alone, so the mock has none for it unless a scenario brings its own
// (`behaves`), as the one under the Usage walk does.

import type { UsageSummaryApi } from "../../../../shared/api/usage-summary";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const usageSummary: Slice<UsageSummaryApi, Record<string, never>> = {
  name: "usageSummary",
  state: {},
  api: () => ({
    getUsageSummary: async (tzOffsetMinutes) => ({ ok: false, outcome: unanswered(`/usage/summary?tz_offset_minutes=${tzOffsetMinutes}`) }),
    getUsageRules: async () => ({ ok: false, outcome: unanswered("/usage/rules") }),
  }),
};
