// The plan's windows: the wire is answered by Fleet alone, so the mock has none for it unless a scenario
// brings its own (`behaves`), as the session-usage one does.

import type { PlanUsageApi } from "../../../../shared/api/plan-usage";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const planUsage: Slice<PlanUsageApi, Record<string, never>> = {
  name: "planUsage",
  state: {},
  api: () => ({
    getPlanUsage: async () => ({ ok: false, outcome: unanswered("/plan_usage") }),
    onPlanUsageChanged: () => () => {},
  }),
};
