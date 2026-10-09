// Which arrangement a Job's two graph destinations open in, remembered across
// a restart — Plan's graph or list (owner, 25 Sep 2026).
//
// **One module, because it is one mechanism.** Plan's toggle was asked for
// after Workflow already had one, and a second way of remembering a way of
// reading is two places to keep in step.
//
// **`localStorage`, not a Fleet preference**, for `panel-open.ts`' own reason:
// Fleet's preference set is closed (`fleet.unknown_preference`) and a SQL
// `CHECK` constraint names every legal key, so a new one is a wire change in
// five crates. These are ways of reading, not facts about the Job.

import { useState } from "react";
import { dashboardTabNamed, type DashboardTab } from "@armada/overview";
import { lessonsTabNamed, planViewNamed, type LessonsTab, type PlanView } from "@armada/jobs";

const PLAN_KEY = "armada.bridge.plan-view";
const LESSONS_KEY = "armada.bridge.lessons-tab";
export const DASHBOARD_KEY = "armada.bridge.dashboard-tab";
const DISPATCH_KEY = "armada.bridge.dispatch-kind";

/**
 * A remembered arrangement and the press that moves it, on whatever the
 * package's own reader makes of what was stored.
 *
 * **A failed read is the default, not a throw.** `localStorage` is denied
 * outright in some window configurations, and a preference is the last thing
 * that should stop a window drawing.
 */
function remembered<T extends string>(key: string, named: (value: string | null) => T): [T, (view: T) => void] {
  const read = (): T => {
    try {
      return named(window.localStorage.getItem(key));
    } catch {
      return named(null);
    }
  };
  const [view, setView] = useState(read);

  function press(next: T): void {
    setView(next);
    try {
      window.localStorage.setItem(key, next);
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
  }

  return [view, press];
}

/** Graph or list on Plan. Graph where nothing is stored. */
export function usePlanView(): [PlanView, (view: PlanView) => void] {
  return remembered(PLAN_KEY, planViewNamed);
}

/** Which place Lessons is narrowed to (owner, 3 Oct 2026). All where nothing is stored. */
export function useLessonsTab(): [LessonsTab, (tab: LessonsTab) => void] {
  return remembered(LESSONS_KEY, lessonsTabNamed);
}

/** Which tab the Dashboard reads. Command Central where nothing is stored. */
export function useDashboardTab(): [DashboardTab, (tab: DashboardTab) => void] {
  return remembered(DASHBOARD_KEY, dashboardTabNamed);
}

export type DispatchKind = "job" | "session";

/** What the Dashboard's dispatch bar starts: the last choice, a Job where nothing is stored. */
export function useDispatchKind(): [DispatchKind, (kind: DispatchKind) => void] {
  return remembered(DISPATCH_KEY, (value): DispatchKind => (value === "session" ? "session" : "job"));
}
