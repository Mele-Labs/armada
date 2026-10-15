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

import { useEffect, useState } from "react";
import { lessonsSourceNamed, lessonsTabNamed, type LessonsSource, planViewNamed, type LessonsTab, type PlanView } from "@armada/jobs";

/** What a press tells the window, so two readers of one key stay in step. */
const HEARD = "armada:remembered";
const PLAN_KEY = "armada.bridge.plan-view";
const LESSONS_KEY = "armada.bridge.lessons-tab";
const LESSONS_SOURCE_KEY = "armada.bridge.lessons-source";
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
  // Another reader of the same key, in this window, hears a press and reads it again.
  useEffect(() => {
    const heard = (event: Event) => {
      if ((event as CustomEvent<string>).detail === key) setView(read());
    };
    window.addEventListener(HEARD, heard);
    return () => window.removeEventListener(HEARD, heard);
  }, []);

  function press(next: T): void {
    setView(next);
    try {
      window.localStorage.setItem(key, next);
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
    window.dispatchEvent(new CustomEvent(HEARD, { detail: key }));
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

/** Whose retros Lessons lists, Sessions' or Jobs'. All where nothing is stored. */
export function useLessonsSource(): [LessonsSource, (source: LessonsSource) => void] {
  return remembered(LESSONS_SOURCE_KEY, lessonsSourceNamed);
}

export type DispatchKind = "job" | "session";

/** What the dispatch panel starts: the last choice, a Job where nothing is stored. */
export function useDispatchKind(): [DispatchKind, (kind: DispatchKind) => void] {
  return remembered(DISPATCH_KEY, (value): DispatchKind => (value === "session" ? "session" : "job"));
}

// ---- The flight deck's views, which live in a store and not in a component ---------------------------------------
//
// The deck keeps its state in a module so the top bar and the body, two trees, agree and a body that unmounts while a
// page is open comes back to the exact view (`cockpit/deck/store.ts`). It remembers the few ways of reading that
// should survive a restart through the same keys and the same failed-read-is-the-default rule as the hooks above.

export const DECK_VIEW_KEY = "armada.bridge.deck-view";
export const DECK_JOB_VIEW_KEY = "armada.bridge.deck-job-view";
export const DECK_ONLY_KEY = "armada.bridge.deck-only";
export const DECK_FILTERS_KEY = "armada.bridge.deck-filters";
/** The System page the System tab opens on: the last one used. */
export const DECK_SYSTEM_KEY = "armada.bridge.deck-system";

/** What was stored under `key`, on whatever the reader makes of it. A failed read is the default. */
export function readRemembered<T extends string>(key: string, named: (value: string | null) => T): T {
  try {
    return named(window.localStorage.getItem(key));
  } catch {
    return named(null);
  }
}

/** Stores a choice. A failed write leaves it unremembered, the honest answer for a preference. */
export function writeRemembered(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Unremembered.
  }
}
