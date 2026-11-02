// What the title row draws beyond the shell's own: the plan's windows. It is behind a source, so a Bridge
// with none leaves its prop off and the row draws nothing for it.

import type { PlanUsage } from "@armada/components";

import { useDownshiftOffer } from "./downshift";
import { usePlanUsage } from "./plan-usage";

export function useTitleRow(openModelSetting: () => void): { planUsage?: PlanUsage[] } {
  const planUsage = useDownshiftOffer(usePlanUsage(), openModelSetting);
  return planUsage === undefined ? {} : { planUsage };
}
