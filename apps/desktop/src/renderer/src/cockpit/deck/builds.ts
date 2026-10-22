// What each Session has built, read for the map: where its ring lies, the frame that aims a laser at it, and what a piece, an arc of
// its road or its band says when it is hovered. The canvas draws the ring (`pixi/tiles.ts`) and the hit layer carries the words
// (`pixi/Hits.tsx`). #2301.

import type { Frame } from "./frame";
import type { Layout, Point } from "./layout";
import { arcSaid, arcWord, arcsOf, askedOf, bandOf, type BuildIn } from "./build";
import { SAID } from "./BuildCard";
import type { Tip } from "./tip";

/** A landed ring stands for this long, then is gone. */
export const FRESH_LAND_MS = 15 * 60_000;

type Placed = { build: BuildIn; anchor: Point; /** The plot it stands over, which lifts it. */ tile: string | undefined };

/**
 * Where each ring lies, in layout units: round its drone's shadow. A Session whose drone is not on the field draws none.
 */
export function placeBuilds(builds: readonly BuildIn[], layout: Layout): Placed[] {
  return builds.flatMap<Placed>((build) => {
    const shadow = layout.at.get(build.key);
    return shadow === undefined ? [] : [{ build, anchor: shadow, tile: layout.over.get(build.key)?.[0]?.tile.path }];
  });
}

/** A frame whose laser lands on the ring under the drone that is building it, and not on the plot's nearest corner. */
export const aimedAt = (frame: Frame, placed: readonly Placed[]): Frame => {
  if (placed.length === 0) return frame;
  const at = new Map(placed.map((one) => [one.build.key, one.anchor] as const));
  return { ...frame, aimPoint: (drone, tile) => at.get(drone.key) ?? frame.aimPoint(drone, tile) };
};

/** What a piece says when it is hovered: the task it stands for, and where that task is. The card is the callout's, since a tooltip cannot open here. */
export function pieceTip(build: BuildIn, k: number): { tip: Tip; said: string } | undefined {
  const task = build.tasks[k];
  if (task === undefined) return undefined;
  const said = SAID[task.piece];
  const tip: Tip = { label: "Task", title: task.text, hue: task.piece === "solid" ? "good" : "quiet", lines: task.by === undefined ? [said] : [said, askedOf(task.by)] };
  return { tip, said: task.by === undefined ? `${task.text}, ${said}` : `${task.text}, ${said}, ${askedOf(task.by)}` };
}

/** What an arc of the road says when it is hovered: its name, and when or that it is only what comes next here. */
export function arcTip(build: BuildIn, k: number): { tip: Tip; said: string } | undefined {
  const one = arcsOf(build)[k];
  if (one === undefined) return undefined;
  const said = arcSaid(one, build.now);
  return { tip: { label: "Road", title: arcWord(one.id), hue: one.at === undefined ? "quiet" : "good", lines: [said] }, said: `${arcWord(one.id)}, ${said}` };
}

/** What the band says when it is hovered: the phase, and nothing of how much is done, since there is no list to say it by. */
export function bandTip(build: BuildIn): { tip: Tip; said: string } | undefined {
  const word = bandOf(build)?.word;
  return word === undefined ? undefined : { tip: { label: "Phase", title: word, hue: "quiet" }, said: word };
}
