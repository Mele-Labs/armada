// What the title row draws beyond the shell's own: the moon, and the plan's windows. Each is behind a
// source, so a Bridge with none leaves its prop off and the row draws nothing for it.

import type { PlanUsage, TheShellProps } from "@armada/components";

import { usePlanUsage } from "./plan-usage";
import { useSleepMode } from "./sleep";

export function useTitleRow(): { sleep?: NonNullable<TheShellProps["sleep"]>; planUsage?: PlanUsage[] } {
  const { sleep } = useSleepMode();
  const planUsage = usePlanUsage();
  return { ...(sleep === undefined ? {} : { sleep }), ...(planUsage === undefined ? {} : { planUsage }) };
}
