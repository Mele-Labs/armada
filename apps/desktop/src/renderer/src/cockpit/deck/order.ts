// How loud a Session is, and the order the deck's list reads them in. Gray is the baseline, one hue says he is
// owed something, and red is a Check failing right now and nothing handled, old or somebody else's.

import type { Phase, Ship, Subject, Tone } from "./ship";

export type Group = "needs" | "working" | "held" | "done";
export const GROUPS: readonly { id: Group; name: string }[] = [
  { id: "needs", name: "Needs you" },
  { id: "working", name: "Working" },
  { id: "held", name: "Stuck or held" },
  { id: "done", name: "Done" },
];

/**
 * **Amber says he is owed something; red says a Check is failing on it right now.** A Session that is stuck is held in
 * its own group and gray, and a red pull request on an idle Session is drawn quiet on its chip: the Session that is on the
 * same failure carries the red.
 */
const QUIET_AFTER = 15;

export function toneOf(subject: Subject): Tone {
  if (subject.kind === "session") {
    if (subject.mark.tone === "ask") return "ask";
    // A failure nobody has touched for a quarter of an hour is old news and drawn quiet; so is a stuck Session.
    if (subject.mark.tone === "bad") return subject.phase === "held" || (subject.idleMinutes ?? 0) >= QUIET_AFTER ? "calm" : "bad";
  }
  return "calm";
}

/** The state a row's lamp shows: needs him, failed, running or done; anything else is idle. */
export type Lamp = "needs" | "failed" | "running" | "done" | "idle";
export function lampOf(subject: Subject): Lamp {
  const tone = toneOf(subject);
  if (tone === "ask") return "needs";
  if (tone === "bad") return "failed";
  const group = groupOf(subject);
  return group === "working" ? "running" : group === "done" ? "done" : "idle";
}

export function groupOf(subject: Subject): Group {
  if (toneOf(subject) !== "calm") return "needs";
  const phase: Phase = subject.phase;
  return phase === "needs" ? "done" : phase;
}

const LOUD: Record<Tone, number> = { bad: 0, ask: 1, calm: 2 };

/** The Sessions of the ship, grouped by standing, the loudest first inside each. */
export function groupedOf(ship: Ship): { group: (typeof GROUPS)[number]; rows: Subject[] }[] {
  const rows = ship.subjects.filter((one) => one.kind !== "pr" && one.past === undefined);
  return GROUPS.map((group) => ({
    group,
    rows: rows.filter((one) => groupOf(one) === group.id).sort((a, b) => LOUD[toneOf(a)] - LOUD[toneOf(b)]),
  })).filter((one) => one.rows.length > 0);
}

export const orderOf = (ship: Ship): Subject[] => groupedOf(ship).flatMap((one) => one.rows);

/**
 * The live Sessions in the order he placed them, and the ones he has not placed after, in standing order. **A state change never
 * moves a row**: opening a Session quiets its call, and a row that sorted by that would jump away under the pointer.
 */
export function fixedOrderOf(ship: Ship, placed: readonly string[]): Subject[] {
  const at = new Map(placed.map((key, place) => [key, place]));
  const rows = orderOf(ship);
  const known = rows.filter((one) => at.has(one.key)).sort((a, b) => at.get(a.key)! - at.get(b.key)!);
  return [...known, ...rows.filter((one) => !at.has(one.key))];
}

/** `key` taken out of the order and put back before `target`, or after it. */
export function movedIn(order: readonly string[], key: string, target: string, after: boolean): string[] {
  if (key === target || !order.includes(key) || !order.includes(target)) return [...order];
  const rest = order.filter((one) => one !== key);
  rest.splice(rest.indexOf(target) + (after ? 1 : 0), 0, key);
  return rest;
}
