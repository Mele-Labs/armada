// A Job is a story, and the right column tells it in order: the workflow, the blueprint (the plan), the field it is built on, and what it lands in.
// The stage is read from data the Job already holds. The Field and the Landing are open at their stage and folded to their bands otherwise. The
// Blueprint is open until the Job lands: drafted it is the plan to approve, and approved it stays up as a compact read of the build. A press on a band or its key
// opens or folds any of them by hand; that holds until the stage moves on. Mock: the landing kind comes from the scenario (`work.landing`) until Fleet says it.

import { useCallback, useSyncExternalStore } from "react";
import { attachmentsOf, type Session, type WorkLanding } from "@armada/screens/src/draft/sessions";

export type Stage = "blueprint" | "field" | "landing";
export type Fold = { open: boolean; toggle: () => void };

const done = (state: string) => state === "done" || state === "handed_in";

/**
 * Where the Job is. A plan not approved is the blueprint. An approved plan with every task done, or a pull request out of draft, is the landing.
 * An approved plan with work left is the field, and so is a draft pull request: it is still being built.
 */
export function stageOf(session: Session): Stage {
  const plan = session.work?.plan;
  if (plan !== undefined && !plan.approved) return "blueprint";
  const landed = attachmentsOf(session, "pull_request").some((pr) => pr.state !== "draft");
  const built = plan !== undefined && plan.groups.length > 0 && plan.groups.every((group) => group.tasks.every((task) => done(task.state)));
  if (landed || built) return "landing";
  return plan === undefined ? "blueprint" : "field";
}

/** What the workflow ends in. */
export function landingOf(session: Session): WorkLanding {
  return session.work?.landing ?? (session.mod === true ? "mod" : "pull_request");
}

// ---- Panels opened or folded by hand -----------------------------------------------------------------------

const forced = new Map<string, boolean>();
const hear = new Set<() => void>();
const keyOf = (session: string, area: Stage) => `${session}:${area}`;
const changed = () => hear.forEach((one) => one());

/** Open at its own stage. The blueprint stays up through the build too, and folds once the Job lands. */
const openAt = (session: Session, area: Stage): boolean => {
  const stage = stageOf(session);
  return area === "blueprint" ? stage !== "landing" : stage === area;
};

/** Whether the panel is open: by hand where it was pressed, else as the stage has it. */
export function useFold(session: Session, area: Stage): Fold {
  const byHand = useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    () => forced.get(keyOf(session.id, area)),
  );
  const open = byHand ?? openAt(session, area);
  const id = session.id;
  const toggle = useCallback(() => {
    forced.set(keyOf(id, area), !open);
    changed();
  }, [id, area, open]);
  return { open, toggle };
}

/** A press of a panel's key: opens it if folded, folds it if open. */
export function pressBand(session: Session, area: Stage): void {
  const was = forced.get(keyOf(session.id, area)) ?? openAt(session, area);
  forced.set(keyOf(session.id, area), !was);
  changed();
}

/** The story moved on: what was opened or folded by hand gives way to the stage again. */
export function resetFolds(session: string): void {
  let any = false;
  for (const key of [...forced.keys()]) {
    if (key.startsWith(`${session}:`)) {
      forced.delete(key);
      any = true;
    }
  }
  if (any) changed();
}

// ---- Keys ---------------------------------------------------------------------------------------------------

/** One key opens each band. Not acts of the registry, which is Fleet's to generate: Bridge reads these where the Job is open. `code` is the physical key, since ⌥ changes the letter. */
export const BAND_KEYS: readonly { area: Stage | "workflow"; code: string; cap: string }[] = [
  { area: "workflow", code: "KeyW", cap: "⌥W" },
  { area: "blueprint", code: "KeyB", cap: "⌥B" },
  { area: "field", code: "KeyF", cap: "⌥F" },
  { area: "landing", code: "KeyL", cap: "⌥L" },
];

export const bandKeyFor = (area: string): string | undefined => BAND_KEYS.find((one) => one.area === area)?.cap;

/** Artifacts over the right column. */
export const ARTIFACTS_KEY = { code: "KeyA", cap: "⇧⌘A" } as const;

export const bandOf = (event: KeyboardEvent): (typeof BAND_KEYS)[number] | undefined =>
  event.altKey && !event.metaKey && !event.ctrlKey && !event.shiftKey ? BAND_KEYS.find((one) => one.code === event.code) : undefined;

export const isArtifactsKey = (event: KeyboardEvent): boolean => event.metaKey && event.shiftKey && !event.altKey && !event.ctrlKey && event.code === ARTIFACTS_KEY.code;
