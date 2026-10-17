// What the canvas map is handed to draw: the battlefield's own readings (the layout, the plots' heights, the route, the pull requests),
// cut to what the canvas needs and with the few placements the CSS map made in its render worked out here, once. Pure. Nothing here
// draws, and nothing is read off the page.

import type { RouteIn } from "../mission";
import type { Frame } from "../frame";
import { ROUTE } from "../geometry";
import { ALONG, type Play } from "../plays";
import { FLAT, alongOf, type DroneIn, type Layout, type P3, type PrIn, type Size } from "../layout";
import type { GoalIn, PadIn } from "../tip";
import type { placeBuilds } from "../Builds";
import { onPlane } from "../bf-parts";

/** A construction placed on the field: its ring and where it lies. */
export type Placed = ReturnType<typeof placeBuilds>[number];

/** A pull request placed on its route to the objective, in the plane's own units. */
export type Crate = {
  pr: PrIn;
  /** Where its route begins and ends, in the air. */
  start: P3;
  end: P3;
  /** Where the crate itself is: along the route by how far it has got, or at the objective once it has landed. */
  at: P3;
  /** Its owner is outside the filters: the route and the crate dim. */
  dim: boolean;
};

export type PixiSceneIn = {
  layout: Layout;
  world: Size;
  /** The plane, whose depth is stretched by `FLAT`. */
  plane: Size;
  big: number;
  heights: ReadonlyMap<string, number>;
  /** The plots a working drone is over, and those Fleet says two are writing. */
  lit: ReadonlySet<string>;
  clashing: ReadonlySet<string>;
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  crates: readonly Crate[];
  /** A Job's mission: its route, and the pin whose card is open. */
  route: RouteIn | undefined;
  pinned: string | undefined;
  /** Zoomed in far enough that a name can be read. */
  readable: boolean;
  builds: readonly Placed[];
};

export type SceneInputs = {
  layout: Layout;
  world: Size;
  big: number;
  frame: Frame;
  drones: readonly DroneIn[];
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  prs: readonly PrIn[];
  plays: readonly Play[];
  route: RouteIn | undefined;
  pinned: string | undefined;
  readable: boolean;
  builds: readonly Placed[];
};

/** The base's hangar end, where a pull request with no drone of its own starts from. */
export function baseOf(layout: Layout, world: Size, root: string): P3 {
  const t = layout.territories.find((one) => one.root === root);
  const at = onPlane(t === undefined ? { x: world.width / 2, y: world.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 });
  return { ...at, z: 20 };
}

/** How the CSS map placed a pull request, which stands on a route from its drone (or the base) to the flag, further along the nearer it is to merging. */
export function cratesOf(i: Pick<SceneInputs, "layout" | "world" | "frame" | "drones" | "prs" | "plays" | "route">): Crate[] {
  const droneBy = new Map(i.drones.map((one) => [one.key, one] as const));
  const landing = new Set(i.plays.filter((one) => one.kind === "capture" && one.number !== undefined).map((one) => one.number));
  const out: Crate[] = [];
  for (const pr of i.prs) {
    const territory = i.layout.territories.find((one) => one.root === pr.root);
    if (territory?.goal === undefined) continue;
    const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
    const start: P3 = i.route !== undefined ? { ...onPlane(i.route.crate), z: ROUTE } : ((owner === undefined ? undefined : i.frame.flying(owner)) ?? { ...baseOf(i.layout, i.world, pr.root), z: ROUTE });
    const end: P3 = { ...onPlane(territory.goal), z: ROUTE };
    const landed = pr.stage === "landed";
    if (landed && !landing.has(pr.number)) continue;
    const rest = alongOf(start, end, 34, landed ? ALONG.queued : ALONG[pr.stage]);
    out.push({ pr, start, end, at: landed ? end : rest, dim: owner !== undefined && !owner.shown });
  }
  return out;
}

export function sceneOf(i: SceneInputs): PixiSceneIn {
  const lit = new Set<string>();
  for (const drone of i.drones) {
    if (!drone.shown || !drone.spin) continue;
    for (const { tile } of i.layout.over.get(drone.key) ?? []) lit.add(tile.path);
  }
  return {
    layout: i.layout,
    world: i.world,
    plane: { width: i.world.width, height: i.world.height * FLAT },
    big: i.big,
    heights: i.frame.heights,
    lit,
    clashing: new Set(i.layout.clashes.map((one) => one.path)),
    pads: i.pads,
    goals: i.goals,
    crates: cratesOf(i),
    route: i.route,
    pinned: i.pinned,
    readable: i.readable,
    builds: i.builds,
  };
}
