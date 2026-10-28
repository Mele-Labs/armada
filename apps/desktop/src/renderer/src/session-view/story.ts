// A Job is a story, and the right column tells it in three zones: Completed (what it has made, in the order it made it), Now (what it is doing this minute and
// what waits on him) and Next (the plan's groups and the workflow's steps still to come, and the outputs still to be made). The plan is the spine: a finished
// group moves to Completed carrying what it produced, the group being worked is in Now, the rest are in Next. A zone is open at every stage and folds only by a press
// of its band or its key, which is kept per Job. The stage (`stageOf`) is read from data the Job already holds and sizes nothing now; it names where the Job is.
// The landing kind is the workflow's `lands`, which Fleet resolves; a mock scenario says it in `work.landing`. Whole plan: ⌥N or its button shows every group with its detail in Now.

import { useCallback, useSyncExternalStore } from "react";
import { attachmentsOf, type Session, type WorkLanding } from "@armada/screens/src/draft/sessions";

export type Stage = "blueprint" | "field" | "landing";
/** The three zones the right column tells a Job in: what it made (drawn as Completed), what it is doing, what comes (`StoryColumn.tsx`). */
export type Zone = "done" | "now" | "next";
export type Area = Stage | Zone;
export type Fold = { open: boolean; toggle: () => void; /** The stage is at this panel: it takes the larger share of the column. */ current: boolean };

const done = (state: string) => state === "done" || state === "handed_in";

/**
 * Where the Job is. A plan not approved is the blueprint. An approved plan with every task done, a pull request out of draft, or a workflow step in
 * the delivery phase, is the landing. An approved plan with work left is the field, and so is a draft pull request: it is still being built.
 * A Job with no plan is the field while its turn runs, since its drones are out, and the blueprint while it waits.
 */
export function stageOf(session: Session): Stage {
  const plan = session.work?.plan;
  if (plan !== undefined && !plan.approved) return "blueprint";
  const landed = attachmentsOf(session, "pull_request").some((pr) => pr.state !== "draft");
  const built = plan !== undefined && plan.groups.length > 0 && plan.groups.every((group) => group.tasks.every((task) => done(task.state)));
  const delivering = session.work?.workflow?.phase === "delivery";
  if (landed || built || delivering) return "landing";
  if (plan === undefined) return session.turn.state === "working" ? "field" : "blueprint";
  return "field";
}

/** What the workflow ends in. A Mod Job lands its mod, whatever its workflow says. */
export function landingOf(session: Session): WorkLanding {
  return session.mod === true ? "mod" : (session.work?.landing ?? "pull_request");
}

// ---- Panels opened or folded by hand -----------------------------------------------------------------------

const forced = new Map<string, boolean>();
const hear = new Set<() => void>();
const keyOf = (session: string, area: Area) => `${session}:${area}`;
const changed = () => hear.forEach((one) => one());

// What was pressed is kept per Job in the window's storage, so a Job comes back as it was left. A viewer's
// convenience, so a blocked store only forgets it. The first look at a Job reads it; the maps are the truth after that.
const KEPT = "armada.session-folds";
type Kept = { folds: Partial<Record<Area, boolean>>; whole: boolean };
const STAGES: readonly Area[] = ["blueprint", "field", "landing", "done", "now", "next"];
const loaded = new Set<string>();

function load(session: string): void {
  if (loaded.has(session)) return;
  loaded.add(session);
  try {
    const kept = JSON.parse(localStorage.getItem(`${KEPT}.${session}`) ?? "null") as Kept | null;
    if (kept === null) return;
    for (const area of STAGES) {
      const fold = kept.folds?.[area];
      if (typeof fold === "boolean") forced.set(keyOf(session, area), fold);
    }
    if (kept.whole === true) whole.set(session, true);
  } catch {
    // Unreadable or blocked: the Job opens with every panel open.
  }
}

function save(session: string): void {
  const folds: Kept["folds"] = {};
  for (const area of STAGES) {
    const fold = forced.get(keyOf(session, area));
    if (fold !== undefined) folds[area] = fold;
  }
  const kept: Kept = { folds, whole: whole.get(session) === true };
  try {
    localStorage.setItem(`${KEPT}.${session}`, JSON.stringify(kept));
  } catch {
    // Not kept: the panels are as they were left until the window closes.
  }
}

const held = (session: string, area: Area) => (load(session), forced.get(keyOf(session, area)));
const heldWhole = (session: string) => (load(session), whole.get(session) === true);
const touched = (session: string) => {
  save(session);
  changed();
};

// Panels that open shut until pressed: the finished steps of a workflow drawn as the spine's zones (`StoryColumn.tsx`).
const shutFirst = new Set<string>();

/** Whether the panel is open: folded only where it was pressed, so open at every stage. `shut` is a panel that starts folded. */
export function useFold(session: Session, area: Area, shut = false): Fold {
  if (shut) shutFirst.add(keyOf(session.id, area));
  else shutFirst.delete(keyOf(session.id, area));
  const byHand = useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    () => held(session.id, area),
  );
  const open = byHand ?? !shut;
  const id = session.id;
  const toggle = useCallback(() => {
    forced.set(keyOf(id, area), !open);
    touched(id);
  }, [id, area, open]);
  return { open, toggle, current: area === "now" || stageOf(session) === area };
}

/** A press of a panel's key: opens it if folded, folds it if open. */
export function pressBand(session: Session, area: Area): void {
  const was = held(session.id, area) ?? !shutFirst.has(keyOf(session.id, area));
  forced.set(keyOf(session.id, area), !was);
  touched(session.id);
}

/** Forgets what was opened or folded by hand, and the Blueprint's whole-plan view: every panel open again, on its Now view. */
export function resetFolds(session: string): void {
  let any = false;
  for (const key of [...forced.keys()]) {
    if (key.startsWith(`${session}:`)) {
      forced.delete(key);
      any = true;
    }
  }
  if (whole.delete(session)) any = true;
  try {
    localStorage.removeItem(`${KEPT}.${session}`);
  } catch {
    // Nothing was kept to remove.
  }
  if (any) changed();
}

// ---- The whole blueprint ------------------------------------------------------------------------------------

const whole = new Map<string, boolean>();

/** Whether an approved Blueprint shows every group and task rather than the Now view. Drafting always shows the whole. */
export function useWhole(session: string): { whole: boolean; toggle: () => void } {
  const now = useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    () => heldWhole(session),
  );
  const toggle = useCallback(() => {
    whole.set(session, !heldWhole(session));
    touched(session);
  }, [session]);
  return { whole: now, toggle };
}

/** A press of the whole-blueprint key: opens the Blueprint if it is folded, and swaps its Now view for the whole plan or back. */
export function pressWhole(session: Session): void {
  const key = keyOf(session.id, "blueprint");
  if (held(session.id, "blueprint") ?? true) {
    whole.set(session.id, !heldWhole(session.id));
  } else {
    forced.set(key, true);
    whole.set(session.id, true);
  }
  touched(session.id);
}

// ---- Keys ---------------------------------------------------------------------------------------------------

/** One key opens each band. Not acts of the registry, which is Fleet's to generate: Bridge reads these where the Job is open. `code` is the physical key, since ⌥ changes the letter. */
export const BAND_KEYS: readonly { area: Area | "workflow"; code: string; cap: string }[] = [
  { area: "workflow", code: "KeyW", cap: "⌥W" },
  { area: "done", code: "Digit1", cap: "⌥1" },
  { area: "now", code: "Digit2", cap: "⌥2" },
  { area: "next", code: "Digit3", cap: "⌥3" },
  { area: "field", code: "KeyF", cap: "⌥F" },
];

export const bandKeyFor = (area: string): string | undefined => BAND_KEYS.find((one) => one.area === area)?.cap;

/** Swaps the approved Blueprint between its Now view and the whole plan. */
export const WHOLE_KEY = { code: "KeyN", cap: "⌥N" } as const;

/** Swaps Completed between By step and By kind. */
export const LEDGER_KEY = { code: "KeyL", cap: "⌥L" } as const;

/** Artifacts over the right column. */
export const ARTIFACTS_KEY = { code: "KeyA", cap: "⇧⌘A" } as const;

export const bandOf = (event: KeyboardEvent): (typeof BAND_KEYS)[number] | undefined =>
  event.altKey && !event.metaKey && !event.ctrlKey && !event.shiftKey ? BAND_KEYS.find((one) => one.code === event.code) : undefined;

export const isWholeKey = (event: KeyboardEvent): boolean => event.altKey && !event.metaKey && !event.ctrlKey && !event.shiftKey && event.code === WHOLE_KEY.code;

export const isLedgerKey = (event: KeyboardEvent): boolean => event.altKey && !event.metaKey && !event.ctrlKey && !event.shiftKey && event.code === LEDGER_KEY.code;

export const isArtifactsKey = (event: KeyboardEvent): boolean => event.metaKey && event.shiftKey && !event.altKey && !event.ctrlKey && event.code === ARTIFACTS_KEY.code;
