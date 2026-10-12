// The Claude plan's limits for the title row. Behind a source the way Sleep's state is, so a Bridge with
// none draws nothing. There is no real one yet: the limits will come from the mod's `session.measure`.
// The mock's is `mock/plan-usage.ts`.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { PlanUsage } from "@armada/components";

export type PlanUsageSource = {
  get(): readonly PlanUsage[] | undefined;
  subscribe(on: () => void): () => void;
};

const PlanUsageContext = createContext<PlanUsageSource | null>(null);
export const PlanUsageSourceProvider = PlanUsageContext.Provider;

const NONE: PlanUsageSource = { get: () => undefined, subscribe: () => () => undefined };

/** The limits, where a source has them. */
export function usePlanUsage(): readonly PlanUsage[] | undefined {
  const source = useContext(PlanUsageContext) ?? NONE;
  return useSyncExternalStore(source.subscribe, source.get);
}
