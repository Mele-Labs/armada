// Where the battlefield puts a drone: over the plots its observed edits name, and otherwise at its pad. A drone with no
// known path is never given a directory.

import { expect, test } from "vitest";

import { bigOf, layoutOf, type DroneIn, type RepoIn } from "./layout";

const SIZE = { width: 1000, height: 700 };
const BIG = bigOf(SIZE);

const drone = (key: string, extra: Partial<DroneIn> = {}): DroneIn => ({
  key,
  kind: "session",
  title: key,
  root: "/r",
  said: "",
  hue: "calm",
  spin: false,
  hover: false,
  ping: false,
  stuck: false,
  flare: false,
  working: false,
  touches: [],
  shown: true,
  ...extra,
});

const repo = (extra: Partial<RepoIn> = {}): RepoIn => ({ root: "/r", name: "r", pads: ["slot-1", "slot-2", "slot-3"], goal: true, areas: [], ...extra });
const AREAS: RepoIn["areas"] = [
  { dir: "crates", kids: [["fleet", 100], ["store", 60]] },
  { dir: "docs", kids: [] },
];

const inside = (tile: { rect: { x: number; y: number; w: number; h: number } }, at: { x: number; y: number }) =>
  at.x >= tile.rect.x && at.x <= tile.rect.x + tile.rect.w && at.y >= tile.rect.y && at.y <= tile.rect.y + tile.rect.h;

test("a drone with no known path rests by the pad of the bay it holds, and is over nothing", () => {
  const layout = layoutOf([repo({ areas: AREAS })], [drone("session:a", { slot: "slot-3" })], SIZE, BIG);
  const territory = layout.territories[0]!;
  const pad = territory.pads.find((one) => one.key === "slot-3")!;
  const at = layout.at.get("session:a")!;

  expect(layout.over.get("session:a")).toEqual([]);
  expect(territory.tiles.some((tile) => inside(tile, at))).toBe(false);
  // Nearer its own pad than either other.
  const far = territory.pads.filter((one) => one.key !== "slot-3");
  for (const other of far) expect(Math.hypot(at.x - pad.x, at.y - pad.y)).toBeLessThan(Math.hypot(at.x - other.x, at.y - other.y));
});

test("a drone that holds no bay rests at its repository's base", () => {
  const layout = layoutOf([repo({ areas: AREAS })], [drone("session:a")], SIZE, BIG);
  const territory = layout.territories[0]!;
  const at = layout.at.get("session:a")!;

  expect(layout.over.get("session:a")).toEqual([]);
  expect(territory.tiles.some((tile) => inside(tile, at))).toBe(false);
  // In the strip above the base, between the plots and the base, inside the territory.
  expect(at.x).toBeGreaterThan(territory.rect.x);
  expect(at.x).toBeLessThan(territory.rect.x + territory.rect.w);
  expect(at.y).toBeGreaterThan(Math.max(...territory.tiles.map((tile) => tile.rect.y + tile.rect.h)));
  expect(at.y).toBeLessThan(territory.base.y);
});

test("a path the repository does not draw is no place: the drone stays at its pad", () => {
  const touched = layoutOf([repo({ areas: AREAS })], [drone("session:a", { slot: "slot-1", touches: [{ path: "apps/desktop", w: 4 }] })], SIZE, BIG);
  const rested = layoutOf([repo({ areas: AREAS })], [drone("session:a", { slot: "slot-1" })], SIZE, BIG);

  expect(touched.over.get("session:a")).toEqual([]);
  expect(touched.at.get("session:a")).toEqual(rested.at.get("session:a"));
});

test("with no areas at all a territory draws its pads and flag with no plots, and a drone still rests at its pad", () => {
  const layout = layoutOf([repo()], [drone("session:a", { slot: "slot-2", touches: [{ path: "crates/fleet", w: 3 }] })], SIZE, BIG);

  expect(layout.territories[0]!.tiles).toEqual([]);
  expect(layout.territories[0]!.pads).toHaveLength(3);
  expect(layout.over.get("session:a")).toEqual([]);
  expect(layout.clashes).toEqual([]);
});

test("a drone with an observed path hovers over the plot of that area", () => {
  const layout = layoutOf([repo({ areas: AREAS })], [drone("session:a", { slot: "slot-1", touches: [{ path: "crates/fleet", w: 5 }] })], SIZE, BIG);
  const tile = layout.territories[0]!.tiles.find((one) => one.path === "crates/fleet")!;

  expect(layout.over.get("session:a")?.map((one) => one.tile.path)).toEqual(["crates/fleet"]);
  expect(Math.hypot(layout.at.get("session:a")!.x - tile.cx, layout.at.get("session:a")!.y - tile.cy)).toBeLessThan(40);
});

test("two drones over one plot are not a clash: only Fleet's write-clash read marks one, and only on a plot that is drawn", () => {
  const drones = [drone("session:a", { touches: [{ path: "crates/fleet", w: 2 }] }), drone("job:b", { kind: "job", touches: [{ path: "crates/fleet", w: 2 }] })];
  expect(layoutOf([repo({ areas: AREAS })], drones, SIZE, BIG).clashes).toEqual([]);

  const clash = { path: "crates/fleet", root: "/r", keys: ["session:a", "job:b"] };
  const elsewhere = { path: "apps/desktop", root: "/r", keys: ["session:a"] };
  expect(layoutOf([repo({ areas: AREAS })], drones, SIZE, BIG, [], [clash, elsewhere]).clashes).toEqual([clash]);
});
