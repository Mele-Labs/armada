// Where a Job's acts stand when its panes do: as icons in the Thread tab's head, drawn by the Cockpit's `SessionTiles` into the slot the head holds. The acts live with the Job, not with the Thread
// tab, so a Thread tab that is closed or hidden leaves them pressable by their keys; and where the head is narrow they are the ellipsis menu's instead.

import type { DropdownMenuEntry } from "@armada/components";
import type { LucideIcon } from "lucide-react";
import { useLayoutEffect, useRef, useSyncExternalStore } from "react";

/**
 * How a Thread head makes room for its name. `rows` is the head: the state and name on a row with the way out, and the repository, the acts and the figures on a
 * row under it, whose acts leave for the ellipsis menu as the row runs out. `overflow` is the mock's other take, picked by `?take=overflow` (`mock/main.tsx`):
 * one row, the name kept at 18 characters while the acts leave.
 */
export type HeadTake = "overflow" | "rows";
export function headTake(): HeadTake {
  return document.documentElement.dataset.headTake === "overflow" ? "overflow" : "rows";
}

export type JobAct = {
  id: string;
  label: string;
  icon?: LucideIcon /** `on` is a switch that is on, `running` a thing that is running: the glyph is blue, and pulses while it runs. */;
  state?: "on" | "running" /** Why it cannot be pressed, where it cannot. */;
  off: string | undefined;
  run: () => void;
};

/** What a head with a take has given up: the acts that moved into the ellipsis menu, by id, and whether the repository is its icon alone. */
type Place = {
  slot: HTMLElement | null;
  away: readonly string[];
  chip: boolean;
};
const NOWHERE: Place = {
  slot: null,
  away: [],
  chip: false,
};
let place: Place = NOWHERE;
const placeListeners = new Set<() => void>();
const setPlace = (next: Place) => {
  if (
    next.slot === place.slot &&
    next.chip === place.chip &&
    next.away.join("\n") === place.away.join("\n")
  )
    return;
  place = next;
  placeListeners.forEach((one) => one());
};

let acts: readonly JobAct[] = [];
let latest: readonly JobAct[] = [];
let actsKey = "";
const actsListeners = new Set<() => void>();

/**
 * Every render offers the acts again with fresh closures; only a change of what they say is told. What a reader holds stays the same list until then, so a reader that
 * draws the acts' own parent cannot be drawn again by them, and each `run` goes to the closure offered last.
 */
export function publishActs(next: readonly JobAct[]): void {
  const key = next
    .map((one) => `${one.id}|${one.label}|${one.state ?? ""}|${one.off ?? ""}`)
    .join("\n");
  latest = next;
  if (key === actsKey) return;
  actsKey = key;
  acts = next.map((one) => ({
    ...one,
    run: () => latest.find((other) => other.id === one.id)?.run(),
  }));
  actsListeners.forEach((one) => one());
}

export const clearActs = () => publishActs([]);

/**
 * The order acts leave a head in, the first named going first: what the owner reads at a glance (a Job's pause, its auto land,
 * its Checks) stays longest, and what he reaches for once in a while goes first. An act not named here, which is one of his own, goes before all of them.
 */
const LEAVES = [
  "requests",
  "release",
  "report",
  "subagents",
  "judge",
  "checks",
  "land",
  "pause",
];
const leaveRank = (id: string) => LEAVES.indexOf(id);
const leaveOrder = (ids: readonly string[]) =>
  [
    ...ids,
  ].sort((a, b) => leaveRank(a) - leaveRank(b));

/** The ellipsis menu's id for an act. */
const ENTRY = "act:";

/** The acts a head has given up, as rows of the ellipsis menu: in the order the head draws them, each with its glyph, its state and why it cannot be pressed where it cannot. */
export const actEntries = (all: readonly JobAct[], away: readonly string[]): DropdownMenuEntry[] =>
  all
    .filter((one) => away.includes(one.id))
    .map(
      (one): DropdownMenuEntry => ({
        kind: "item",
        id: `${ENTRY}${one.id}`,
        label: one.label,
        ...(one.icon === undefined
          ? {}
          : {
              icon: one.icon,
            }),
        ...(one.state === undefined
          ? {}
          : {
              state: one.state,
            }),
        disabled: one.off !== undefined,
        ...(one.off === undefined
          ? {}
          : {
              hint: one.off,
            }),
      }),
    );

/** Runs the act a row of `actEntries` names. False for any other row. */
export function runActEntry(all: readonly JobAct[], id: string): boolean {
  if (!id.startsWith(ENTRY)) return false;
  all.find((one) => one.id === id.slice(ENTRY.length))?.run();
  return true;
}

export function useActsPlace(): Place {
  return useSyncExternalStore(
    (hear) => (placeListeners.add(hear), () => void placeListeners.delete(hear)),
    () => place,
  );
}

export function useActs(): readonly JobAct[] {
  return useSyncExternalStore(
    (hear) => (actsListeners.add(hear), () => void actsListeners.delete(hear)),
    () => acts,
  );
}

/** What one head has taught the measuring, which no re-render forgets. */
type Widths = {
  icon: number;
  chipFull: number;
  chipIcon: number;
  ch18: number;
};

const px = (value: string) => Number.parseFloat(value) || 0;
/** An element's width as it asks for it, not as its flex row has squeezed it. */
const asked = (one: Element) => Math.max(one.getBoundingClientRect().width, one.scrollWidth);

/** The name's floor: 18 characters of its own type, and what stands beside the text in its span (the wand). */
function floorOf(title: HTMLElement, widths: Widths): number {
  if (widths.ch18 === 0) {
    const probe = document.createElement("span");
    probe.style.cssText = "position:absolute;visibility:hidden;width:18ch";
    title.append(probe);
    widths.ch18 = probe.getBoundingClientRect().width;
    probe.remove();
  }
  const gap = px(getComputedStyle(title).columnGap);
  const beside = [
    ...title.children,
  ].filter(
    (one) =>
      one.querySelector(".armada-session-frame__title-text, input") === null &&
      !one.matches(".armada-session-frame__title-text"),
  );
  return widths.ch18 + beside.reduce((sum, one) => sum + gap + asked(one), 0);
}

/**
 * How much of a row's content has to give way for the name to keep its floor. Going narrower the repository goes to its icon first and then the
 * acts go one at a time in `LEAVES` order; going wider the same steps come back in reverse. Widths are read off what is drawn, so the row's own padding and
 * gap, a longer context figure or a mark that appears are all counted, and no step is guessed from the window.
 */
function settle(
  row: HTMLElement,
  slot: HTMLElement,
  widths: Widths,
  count: number,
  now: {
    away: number;
    chip: boolean;
  },
): {
  away: number;
  chip: boolean;
} {
  const style = getComputedStyle(row);
  const gap = px(style.columnGap);
  const inner = px(getComputedStyle(slot).columnGap);
  const avail = row.clientWidth - px(style.paddingLeft) - px(style.paddingRight);
  const drawn = [
    ...row.children,
  ].filter((one) => getComputedStyle(one).display !== "none") as HTMLElement[];
  const chipEl = row.querySelector<HTMLElement>(".armada-session-repository");
  const first = slot.firstElementChild;
  if (first !== null) widths.icon = asked(first);
  if (chipEl !== null) {
    if (now.chip) widths.chipIcon = chipEl.getBoundingClientRect().width;
    else widths.chipFull = chipEl.getBoundingClientRect().width;
  }
  const title = drawn.find((one) => one.classList.contains("armada-session-frame__title"));
  const slotAt = (shown: number) =>
    shown <= 0 ? 0 : shown * widths.icon + (shown - 1) * inner + gap;
  const chipAt = (icon: boolean) =>
    chipEl === null ? 0 : icon ? widths.chipIcon || widths.icon : widths.chipFull;
  let need = gap * Math.max(drawn.length - 1, 0);
  for (const one of drawn) need += one === title ? floorOf(title, widths) : asked(one);
  const total = (away: number, chip: boolean) =>
    need - slotAt(count - now.away) + slotAt(count - away) - chipAt(now.chip) + chipAt(chip);
  const fits = (away: number, chip: boolean) => total(away, chip) <= avail;
  let { away, chip } = now;
  while (!fits(away, chip)) {
    if (!chip && chipEl !== null) chip = true;
    else if (away < count) away += 1;
    else break;
  }
  while (away > 0 && fits(away - 1, chip)) away -= 1;
  if (away === 0 && chip && fits(0, false)) chip = false;
  return {
    away,
    chip,
  };
}

/** The place in a Thread head the acts are drawn into. */
export function JobActsSlot() {
  const element = useRef<HTMLSpanElement>(null);
  const widths = useRef<Widths>({
    icon: 24,
    chipFull: 0,
    chipIcon: 0,
    ch18: 0,
  });
  const measuring = useRef<(() => void) | undefined>(undefined);
  // Each change of what is drawn is measured again, a step at a time until the row fits.
  const here = useActsPlace();
  const offered = useActs();
  useLayoutEffect(
    () => measuring.current?.(),
    [
      here,
      offered,
    ],
  );
  useLayoutEffect(() => {
    const slot = element.current;
    if (slot === null) return;
    const head = slot.closest("[data-room]");
    const measure = () => {
      if (!(head instanceof HTMLElement))
        return setPlace({
          ...NOWHERE,
          slot,
        });
      const leaving = leaveOrder(acts.map((one) => one.id));
      const next = settle(head, slot, widths.current, leaving.length, {
        away: place.away.length,
        chip: place.chip,
      });
      setPlace({
        slot,
        away: leaving.slice(0, next.away),
        chip: next.chip,
      });
    };
    measuring.current = measure;
    measure();
    const watch =
      typeof ResizeObserver === "undefined" || head === null
        ? undefined
        : new ResizeObserver(measure);
    if (head !== null) watch?.observe(head);
    return () => {
      measuring.current = undefined;
      watch?.disconnect();
      setPlace(NOWHERE);
    };
  }, []);
  return <span ref={element} className="job-acts" role="toolbar" aria-label="Job acts" />;
}
