// Where the Usage page gets what the tokens were spent on: Fleet's `get_usage_spent`, read with the rest of the page and
// drawn under the titles Bridge holds for each Job. A mock scenario may hand a finished stand-in through `SpentFrom`, which
// wins over Fleet's answer.

import { createContext, useContext, type ReactNode } from "react";
import type { UsageSpent } from "@armada/components";
import type { UsageSpentUnits } from "@armada/protocol";

const Source = createContext<UsageSpent | undefined>(undefined);

export function SpentFrom({
  spent,
  children,
}: {
  spent: UsageSpent | undefined;
  children: ReactNode;
}) {
  return <Source.Provider value={spent}>{children}</Source.Provider>;
}

/** A stand-in a scenario handed, if any. */
export const useSpent = (): UsageSpent | undefined => useContext(Source);

const WINDOW_LABELS: Readonly<Record<string, string>> = {
  day: "24 hours",
  week: "7 days",
  fortnight: "14 days",
};

/** Fleet's windows as the page draws them: each labelled, and each Job under its title, else the end of its id. */
export function spentOf(wire: UsageSpentUnits, titles: ReadonlyMap<string, string>): UsageSpent {
  return {
    windows: wire.windows.map((one) => ({
      id: one.id,
      label: WINDOW_LABELS[one.id] ?? one.id,
      total: one.total,
      jobs: one.jobs.map((job) => ({
        id: job.session_id,
        title: titles.get(job.session_id) ?? job.session_id.slice(-6),
        split: job.split,
      })),
    })),
  };
}
