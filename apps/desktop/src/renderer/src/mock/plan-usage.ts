// Mock-only: the plan's limits with no mod behind them. A scenario sets them, and a walk's `later`
// moves them on, so the title row can be seen at each level.

import type { PlanUsage } from "@armada/components";
import type { PlanUsageSource } from "../plan-usage";

export function createMockPlanUsage(): PlanUsageSource & { set(to: readonly PlanUsage[] | undefined): void } {
  let held: readonly PlanUsage[] | undefined;
  const listeners = new Set<() => void>();
  return {
    get: () => held,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    set: (to) => {
      held = to;
      listeners.forEach((on) => on());
    },
  };
}

export const mockPlanUsage = createMockPlanUsage();
