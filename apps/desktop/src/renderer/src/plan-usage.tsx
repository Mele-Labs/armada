// The plan's windows for the title row. Behind a source the way Sleep's state is, so a Bridge with none
// draws nothing. The real one is `fleet-plan-usage.ts`, over Fleet's `/plan_usage` and `plan_usage.changed`.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { PlanUsage as PlanUsageDrawn } from "@armada/components";
import type { PlanUsage } from "@armada/protocol";

export type PlanUsageSource = {
  get(): PlanUsage | undefined;
  subscribe(on: () => void): () => void;
};

const PlanUsageContext = createContext<PlanUsageSource | null>(null);
export const PlanUsageSourceProvider = PlanUsageContext.Provider;

const NONE: PlanUsageSource = { get: () => undefined, subscribe: () => () => undefined };

/**
 * The title row's figures, from the wire's windows: `percent_used` is the row's `percentUsed`. Nothing for a
 * reading of no windows, an account off a subscription, where an empty list draws the rings loading.
 */
export function planUsageDrawn(usage: PlanUsage): PlanUsageDrawn[] | undefined {
  if (usage.windows.length === 0 && usage.read_at !== undefined) return undefined;
  return usage.windows.map((one) => ({ kind: one.kind, percentUsed: one.percent_used, ...(one.resets_at === undefined ? {} : { resetsAt: one.resets_at }) }));
}

/** The windows the title row draws, where a source has a reading of a plan. */
export function usePlanUsage(): PlanUsageDrawn[] | undefined {
  const source = useContext(PlanUsageContext) ?? NONE;
  const usage = useSyncExternalStore(source.subscribe, source.get);
  return usage === undefined ? undefined : planUsageDrawn(usage);
}
