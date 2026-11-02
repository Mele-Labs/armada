// Copy debug report's reads: a scenario's `debugReport` readings feed the sheet directly (`debug-report-reader.ts`), so the
// mock Fleet answers neither read.

import type { DebugReportApi } from "../../../../shared/api/debug-report";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const debugReport: Slice<DebugReportApi, Record<string, never>> = {
  name: "debugReport",
  state: {},
  api: () => ({
    getDebugReport: async () => ({
      ok: false,
      outcome: unanswered("/debug_report"),
    }),
    runArmadaReport: async () => ({
      ok: false,
      why: "the mock runs no armada",
    }),
  }),
};
