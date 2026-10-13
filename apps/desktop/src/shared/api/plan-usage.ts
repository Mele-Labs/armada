// The plan's windows: the newest reading Fleet holds, and the event that carries it whole when it
// changes. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, PlanUsage } from "@armada/protocol";

/** What reading the plan came to: Fleet's reading, or the refusal to word. */
export type PlanUsageRead = { ok: true; value: PlanUsage } | { ok: false; outcome: Outcome };

export type PlanUsageApi = {
  /** The newest reading of the account's windows. */
  getPlanUsage: () => Promise<PlanUsageRead>;
  /** `plan_usage.changed`: the reading, whole, whenever a session reports a newer one. */
  onPlanUsageChanged: (onChanged: (usage: PlanUsage) => void) => () => void;
};

export const PLAN_USAGE_CHANNELS = {
  getPlanUsage: "bridge:get-plan-usage",
  /** Main to every window, not a request. */
  planUsageChanged: "bridge:plan-usage-changed",
} as const;
