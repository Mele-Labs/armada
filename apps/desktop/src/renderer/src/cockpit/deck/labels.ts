// Which names the map draws. A drone's name and a plot's are each drawn only where they meet no other, so no two collide and none lies
// over a drone's body that it does not name; the drone that is picked or under the pointer is always named. Zoomed out to the whole
// field the type is too small to read, so no name is drawn and the callout says it. Pure. Ported from proto/cockpit-flight-deck. #2301.

import type { DroneIn } from "./layout";

/** A box on the plane, left, right, top and bottom. */
export type Box = { l: number; r: number; t: number; b: number };
export const meets = (a: Box, b: Box): boolean => a.l < b.r && a.r > b.l && a.t < b.b && a.b > b.t;

/** Where a drone stands on the plane and how far it is lifted, for the stack of the screen: the plane stands out of it by its tilt. */
export type Place = (key: string) => { x: number; y: number; up: number } | undefined;

const rank = (one: DroneIn, selected: string | undefined, hot: string | undefined) => (one.key === selected ? 0 : one.key === hot ? 1 : one.hue === "bad" ? 2 : 3);

/** The box each drone's body takes, as the map shows it. */
export function bodiesOf(drones: readonly Pick<DroneIn, "key" | "kind">[], place: Place, big: number): (Box & { key: string })[] {
  return drones.flatMap((one) => {
    const there = place(one.key);
    if (there === undefined) return [];
    const w = (one.kind === "job" ? 34 : 26) * big;
    return [{ key: one.key, l: there.x - w, r: there.x + w, t: there.y - there.up - 30 * big, b: there.y - there.up + 22 * big }];
  });
}

/**
 * The drones that are named: the picked one's, the one under the pointer's, then those that need him and those failing, each only
 * where it meets no body but its own and no name already drawn. `taken` is the boxes of the names drawn.
 */
export function labelsOf(input: { drones: readonly DroneIn[]; place: Place; bodies: readonly (Box & { key: string })[]; big: number; readable: boolean; selected: string | undefined; hot: string | undefined }): { labels: Set<string>; taken: Box[] } {
  const { drones, place, bodies, big, readable, selected, hot } = input;
  const labels = new Set<string>();
  const taken: Box[] = [];
  const ranked = drones.filter((one) => readable && one.shown && (one.key === selected || one.key === hot || one.hue !== "calm")).sort((a, b) => rank(a, selected, hot) - rank(b, selected, hot));
  for (const one of ranked) {
    const there = place(one.key);
    if (there === undefined) continue;
    const wide = Math.min(22, one.title.length) * 6.7 + 10;
    const y = there.y - there.up - 24 * big - (one.flare ? 26 : one.ping ? 14 : 4);
    const box = { l: there.x - wide / 2, r: there.x + wide / 2, t: y - 12, b: y + 5 };
    if (one.key !== selected && one.key !== hot && (bodies.some((other) => other.key !== one.key && meets(box, other)) || taken.some((other) => meets(box, other)))) continue;
    taken.push(box);
    labels.add(one.key);
  }
  return { labels, taken };
}

/** Whether a plot's name is drawn: wanted (a drone that is picked or calling is over it), readable, and clear of every body and name; the pointer on the plot always names it. */
export function plotNamed(input: { box: Box; wanted: boolean; readable: boolean; hot: boolean; bodies: readonly Box[]; taken: readonly Box[] }): boolean {
  const { box, wanted, readable, hot, bodies, taken } = input;
  return readable && (hot || (wanted && !bodies.some((one) => meets(box, one)) && !taken.some((one) => meets(box, one))));
}
