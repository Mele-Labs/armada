// The Usage page over Fleet: the summary read when Fleet is connected and again each minute, the titles Bridge already holds
// for Jobs, and the plan's weekly window from the plan's own reading. The page draws what it is handed and leaves empty
// what it is not.

import { useEffect, useMemo, useState } from "react";
import { UsagePage } from "@armada/components";
import type { UsageSummary } from "@armada/protocol";

import type { BridgeState } from "../../shared/bridge";
import { usePlanUsage } from "./plan-usage";

const EVERY_MS = 60_000;

/** Job titles by id, from the Sessions Bridge holds; a Job with no title is left out. */
export function titlesOf(sessions: BridgeState["sessions"]): ReadonlyMap<string, string> {
  const titles = new Map<string, string>();
  if (sessions.state !== "read") return titles;
  for (const one of sessions.sessions) if (one.title !== undefined && one.title !== "") titles.set(one.id, one.title);
  return titles;
}

export function UsageSurface({ state }: { state: Pick<BridgeState, "connection" | "sessions"> }) {
  const [summary, setSummary] = useState<UsageSummary | undefined>(undefined);
  const connected = state.connection.state === "connected";
  useEffect(() => {
    if (!connected) return;
    let live = true;
    const read = (): void => {
      void window.armada.getUsageSummary(-new Date().getTimezoneOffset()).then((answer) => {
        if (live && answer.ok) setSummary(answer.value);
      });
    };
    read();
    const again = setInterval(read, EVERY_MS);
    return () => {
      live = false;
      clearInterval(again);
    };
  }, [connected]);
  const titles = useMemo(() => titlesOf(state.sessions), [state.sessions]);
  const planPercent = usePlanUsage()?.find((one) => one.kind === "seven_day")?.percentUsed;
  return <UsagePage summary={summary} titles={titles} planPercent={planPercent} />;
}
