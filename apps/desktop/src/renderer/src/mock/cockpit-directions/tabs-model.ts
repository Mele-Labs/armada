// The workspace tab bar's model: which tabs there are, the one the window is on (read off the app's own
// state, never kept beside it), which was last in front, and what each tab remembers. Also the colour budget,
// as functions: gray is the baseline, one hue says "needs you", red is a thing failing right now and
// nothing handled, old or somebody else's. Mock only. proto/cockpit-three-directions.

import { useSyncExternalStore } from "react";
import { Check, Clock, Eye, Hammer, Pause, X, type LucideIcon } from "lucide-react";
import { SURFACE } from "@armada/shell";

import type { Phase, Pr, Ship, StationId, Subject, Tone } from "./ship";

// ---- The tabs -----------------------------------------------------------------------------------

export type Pinned = { id: string; name: string; surface: string; digit: string };

/** The places that are always there, in order, each with the digit that jumps to it. */
export const PINNED: readonly Pinned[] = [
  { id: "cockpit", name: "Cockpit", surface: SURFACE.overview, digit: "1" },
  { id: "jobs", name: "Jobs", surface: SURFACE.jobs, digit: "2" },
  { id: "sessions", name: "Sessions", surface: SURFACE.sessions, digit: "3" },
  { id: "checks", name: "Checks", surface: SURFACE.checks, digit: "4" },
  { id: "merge", name: "Merge line", surface: SURFACE.mergeLine, digit: "5" },
  { id: "bays", name: "Bays", surface: SURFACE.worktrees, digit: "6" },
];
const PINNED_OF_SURFACE = new Map(PINNED.map((one) => [one.surface, one]));

/** Pages that open as tabs of their own, closable. */
export const PLACE_NAME: Record<string, string> = {
  [SURFACE.studios]: "Studios",
  [SURFACE.lessons]: "Retros",
  [SURFACE.settings]: "Settings",
  [SURFACE.kit]: "Kit",
  [SURFACE.mods]: "Mods",
  [SURFACE.guides]: "Guides",
  [SURFACE.workflows]: "Workflows",
  [SURFACE.doctor]: "Doctor",
  [SURFACE.manifest]: "Manifest",
};

/** `g` and a letter. Studios and Retros have no digit: they open from here or from ⌘K. */
export const CHORD: Record<string, string> = { j: SURFACE.jobs, s: SURFACE.sessions, c: SURFACE.checks, m: SURFACE.mergeLine, b: SURFACE.worktrees, t: SURFACE.studios, r: SURFACE.lessons };

export type Where = { showing: string; openJobId: string | null; openSessionId: string | null };

/** The tab the app's own state says is in front. */
export function idOfPlace({ showing, openJobId, openSessionId }: Where): string {
  if (openJobId !== null) return `job:${openJobId}`;
  if (showing === SURFACE.sessions && openSessionId !== null) return `session:${openSessionId}`;
  return PINNED_OF_SURFACE.get(showing)?.id ?? `place:${showing}`;
}

export const isPinned = (id: string) => PINNED.some((one) => one.id === id);
export const pinnedOf = (id: string) => PINNED.find((one) => one.id === id);

// ---- What the window remembers about its tabs ------------------------------------------------------

type State = {
  /** The tabs that are not pinned, in the order they were opened. */
  opened: readonly string[];
  /** Most recently in front first: where closing a tab returns to. */
  mru: readonly string[];
  /** What a tab's page was scrolled to, by the scroller's own signature. */
  scrolled: Readonly<Record<string, Readonly<Record<string, number>>>>;
  /** A thing the Cockpit should put under focus the next time it is drawn. */
  focusing: string | undefined;
};
const FRESH: State = { opened: [], mru: ["cockpit"], scrolled: {}, focusing: undefined };
let state: State = FRESH;
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const set = (change: Partial<State>) => {
  state = { ...state, ...change };
  listeners.forEach((one) => one());
};

export const forgetTabs = () => {
  state = FRESH;
  listeners.forEach((one) => one());
};
export const useTabs = () => useSyncExternalStore(subscribe, () => state);
export const tabsNow = () => state;

export const exists = (id: string) => isPinned(id) || state.opened.includes(id);

/** The window is now on `id`: it is open, and it is the one to come back to. */
export function arrive(id: string): void {
  const opened = isPinned(id) || state.opened.includes(id) ? state.opened : [...state.opened, id];
  const mru = [id, ...state.mru.filter((one) => one !== id)].slice(0, 30);
  if (opened === state.opened && mru[0] === state.mru[0] && mru.length === state.mru.length) return;
  set({ opened, mru });
}

/** Where closing `id` goes: the tab it was reached from, by most recent use, else the Cockpit. */
export const returnFrom = (id: string): string => state.mru.find((one) => one !== id && exists(one)) ?? "cockpit";

export function drop(id: string): void {
  if (isPinned(id)) return;
  const { [id]: _gone, ...scrolled } = state.scrolled;
  set({ opened: state.opened.filter((one) => one !== id), mru: state.mru.filter((one) => one !== id), scrolled });
}

export function rememberScroll(id: string, key: string, top: number): void {
  // Held without a render: nothing draws a scroll position.
  state = { ...state, scrolled: { ...state.scrolled, [id]: { ...state.scrolled[id], [key]: top } } };
}
export const scrollOf = (id: string): Readonly<Record<string, number>> => state.scrolled[id] ?? {};

export const askFocus = (key: string | undefined) => set({ focusing: key });

// ---- The colour budget ------------------------------------------------------------------------------

export type Group = "needs" | "working" | "held" | "done";
export const GROUPS: readonly { id: Group; name: string }[] = [
  { id: "needs", name: "Needs you" },
  { id: "working", name: "Working" },
  { id: "held", name: "Stuck or held" },
  { id: "done", name: "Done" },
];

/**
 * How loud a Job or Session is. **Amber says he is owed something; red says a Check is failing on it right
 * now.** A Session that is stuck is held in its own group and gray, and a red pull request on an idle Session
 * is drawn quiet on its chip: the Job that is on the same failure carries the red.
 */
const QUIET_AFTER = 15;
const minutes = (subject: Subject) => Number.parseInt(subject.since ?? "0", 10) || 0;

export function toneOf(subject: Subject): Tone {
  if (subject.kind === "job") return subject.mark.said === "Check failed" ? "bad" : subject.mark.tone === "ask" ? "ask" : "calm";
  if (subject.kind === "session") {
    if (subject.mark.tone === "ask") return "ask";
    // A failure nobody has touched for a quarter of an hour is old news and drawn quiet; so is a stuck Session.
    if (subject.mark.tone === "bad") return subject.phase === "held" || minutes(subject) >= QUIET_AFTER ? "calm" : "bad";
  }
  return "calm";
}

export function groupOf(subject: Subject): Group {
  if (toneOf(subject) !== "calm") return "needs";
  const phase: Phase = subject.phase;
  return phase === "needs" ? "done" : phase;
}

const LOUD: Record<Tone, number> = { bad: 0, ask: 1, calm: 2 };

/** The Jobs and Sessions of the ship, grouped by standing, the loudest first inside each. */
export function groupedOf(ship: Ship): { group: (typeof GROUPS)[number]; rows: Subject[] }[] {
  const rows = ship.subjects.filter((one) => one.kind !== "pr");
  return GROUPS.map((group) => ({
    group,
    rows: rows.filter((one) => groupOf(one) === group.id).sort((a, b) => LOUD[toneOf(a)] - LOUD[toneOf(b)]),
  })).filter((one) => one.rows.length > 0);
}

export const orderOf = (ship: Ship): Subject[] => groupedOf(ship).flatMap((one) => one.rows);

/** A mark: the glyph a place wears, the shape of its state beside it, and how loud it is. */
export type Roll = { id: StationId; name: string; tone: Tone | "off"; badge: LucideIcon | undefined; lines: readonly string[] };

const VOTE_OF: Record<Tone | "off", LucideIcon | undefined> = { calm: Check, ask: Eye, bad: X, off: undefined };

/** The roll call: main, the merge line, the bays and Autopilot, each as one mark that names why. */
export function rollOf(ship: Ship): Roll[] {
  const main = ship.main;
  const red = main?.state === "red";
  const ready = readyPrs(ship);
  const free = ship.bays.some((one) => one.state === "free");
  const mainRoll: Roll =
    main === undefined
      ? { id: "main", name: "Main", tone: "off", badge: undefined, lines: ["Main: not read"] }
      : red
        ? { id: "main", name: "Main", tone: main.fixing ? "calm" : "bad", badge: main.fixing ? Hammer : X, lines: [`Main: ${main.check ?? "a Check"} failing`, ...(main.fixing ? ["A fix is running"] : [])] }
        : main.state === "running"
          ? { id: "main", name: "Main", tone: "calm", badge: Clock, lines: ["Main: Checks running"] }
          : { id: "main", name: "Main", tone: "calm", badge: Check, lines: ["Main: green"] };
  const line: Roll = red
    ? { id: "merge", name: "Merge line", tone: "calm", badge: Pause, lines: ["Merge line: stopped for main"] }
    : ready.length > 0
      ? { id: "merge", name: "Merge line", tone: "ask", badge: Eye, lines: ready.slice(0, 3).map((pr) => `Merge line: #${pr.number} ready to merge`) }
      : { id: "merge", name: "Merge line", tone: "calm", badge: Check, lines: ["Merge line: clear"] };
  const bays: Roll = free ? { id: "bays", name: "Bays", tone: "calm", badge: Check, lines: ["Bays: one is free"] } : { id: "bays", name: "Bays", tone: "calm", badge: Pause, lines: ["Bays: none free"] };
  const autopilot: Roll = { id: "autopilot", name: "Autopilot", tone: "off", badge: undefined, lines: [ship.autopilot === "off" ? "Autopilot: off" : ship.autopilot === "paused" ? "Autopilot: paused" : "Autopilot: running"] };
  return [mainRoll, line, bays, autopilot];
}

export const badgeOf = (tone: Tone | "off") => VOTE_OF[tone];

/** The tone a tab wears for its pinned place: only red main marks a place, and only until a fix is running. */
export function mainIsRed(ship: Ship): boolean {
  return ship.main?.state === "red" && ship.main.fixing !== true;
}

// ---- What needs him, in the order `.` walks it ------------------------------------------------------

export type Need = { key: string; kind: "main" | "subject" | "unit" };

/** Main when it is red and nobody is fixing it, then what is failing, then what he is owed, then what he can merge. */
export function needsOf(ship: Ship): Need[] {
  const out: Need[] = [];
  if (mainIsRed(ship)) out.push({ key: "main", kind: "main" });
  const rows = orderOf(ship);
  for (const one of rows.filter((s) => toneOf(s) === "bad")) out.push({ key: one.key, kind: "subject" });
  for (const one of rows.filter((s) => toneOf(s) === "ask")) out.push({ key: one.key, kind: "subject" });
  if (ship.main?.state !== "red") for (const pr of readyPrs(ship)) out.push({ key: `unit:${pr.key}`, kind: "unit" });
  return out;
}

export const readyPrs = (ship: Ship): Pr[] => ship.prs.filter((pr) => ship.subjects.some((one) => one.pr?.key === pr.key && one.mergeable));
