// The Cockpit on All repositories with Sessions served and the plan's 5-hour limit nearly spent, so there is a reason
// to save an idea and not start it. The dispatch sheet's Session kind has its Model and Effort selects and Queue for
// later beside Start; a queued idea waits in its repository's group of the Cockpit's list. Built on `quick-dispatch`, so the four
// repositories and the Session store are its own. The walk `queue-for-later` plays it.

import type { PlanUsage } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s210QuickDispatch } from "./quick-dispatch";

const fromNow = (hours: number, minutes: number): string => new Date(Date.now() + (hours * 60 + minutes) * 60_000 + 30_000).toISOString();

const nearlyFull = (): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: 93, resets_at: fromNow(3, 40) },
    { kind: "seven_day", percent_used: 64, resets_at: fromNow(2 * 24 + 17, 0) },
  ],
  read_at: new Date().toISOString(),
});

export const s264QueueForLater: Scenario = {
  ...s210QuickDispatch,
  name: "queue-for-later",
  says: "Sessions served, the 5-hour limit at 93%: Queue for later beside Start, Model and Effort in the dispatcher, queued ideas in their repositories' groups of the Cockpit's list",
  behaves: (handle) => ({
    ...s210QuickDispatch.behaves?.(handle),
    getPlanUsage: async () => ({ ok: true, value: nearlyFull() }),
    onPlanUsageChanged: () => () => {},
  }),
};
