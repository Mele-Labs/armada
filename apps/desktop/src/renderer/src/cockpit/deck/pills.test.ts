// The pills that fill a plot: one for each file its directory holds, in a grid that depends on the count and the plot's size and on nothing
// else, so a file is always in the same place. The canvas draws the grid and the pointer finds a pill by arithmetic on the plot.

import { expect, test } from "vitest";

import { FLAT, layoutOf, type DroneIn, type RepoIn } from "./layout";
import { gridOf, pillAt, pillRect } from "./pills";

const ROOM = { x: 100, y: 40, w: 220, h: 120 };
const NAMES = ["a.rs", "b.rs", "c.rs"];

const centre = (r: { x: number; y: number; w: number; h: number }) => ({ x: r.x + r.w / 2, y: r.y + r.h / 2 });

test("a pill stands in the cell its index names, and the point in that cell finds the index", () => {
  for (const count of [1, 5, 40, 300]) {
    const grid = gridOf(ROOM, count, NAMES, FLAT)!;
    expect(grid, `${count} files`).toBeDefined();
    for (let i = 0; i < count; i += 1) {
      const at = centre(pillRect(grid, i));
      expect(pillAt(grid, at.x, at.y), `pill ${i} of ${count}`).toBe(i);
    }
  }
});

test("the grid holds exactly the files the directory has: the empty cells at its end find no pill, and nor does a point outside it", () => {
  const grid = gridOf(ROOM, 7, [], FLAT)!;
  expect(grid.count).toBe(7);
  // The last row is partly empty unless the columns divide the count.
  const cells = grid.cols * Math.ceil(7 / grid.cols);
  for (let i = 7; i < cells; i += 1) {
    const at = centre(pillRect(grid, i));
    expect(pillAt(grid, at.x, at.y), `empty cell ${i}`).toBeUndefined();
  }
  expect(pillAt(grid, grid.x - 1, grid.y + 1)).toBeUndefined();
  expect(pillAt(grid, grid.x + 1, grid.y - 1)).toBeUndefined();
  expect(pillAt(grid, grid.x + grid.cols * grid.cw + 1, grid.y + 1)).toBeUndefined();
});

test("where a file stands depends on the count and the plot, not on which files are named", () => {
  const named = gridOf(ROOM, 30, NAMES, FLAT)!;
  const bare = gridOf(ROOM, 30, [], FLAT)!;
  expect({ ...named, names: [] }).toEqual({ ...bare, names: [] });
  expect(pillRect(named, 12)).toEqual(pillRect(bare, 12));
  // The named files are the first of them, relative to the plot.
  expect(named.names).toEqual(NAMES);
});

test("no pill is wider than its cell and none lies outside the plot", () => {
  for (const count of [3, 60, 900]) {
    const grid = gridOf(ROOM, count, [], FLAT)!;
    for (const i of [0, count - 1]) {
      const r = pillRect(grid, i);
      expect(r.w).toBeLessThanOrEqual(grid.cw + 1e-9);
      expect(r.x).toBeGreaterThanOrEqual(ROOM.x);
      expect(r.x + r.w).toBeLessThanOrEqual(ROOM.x + ROOM.w + 1e-9);
      expect(r.y + r.h).toBeLessThanOrEqual(ROOM.y + ROOM.h + 1e-9);
    }
  }
});

test("a plot with no files, or too small to hold one, has no grid", () => {
  expect(gridOf(ROOM, 0, [], FLAT)).toBeUndefined();
  expect(gridOf({ x: 0, y: 0, w: 4, h: 4 }, 10, [], FLAT)).toBeUndefined();
});

// ---- Through the layout ------------------------------------------------------------------------------

const drone: DroneIn = { key: "session:a", kind: "session", title: "a", root: "/r", said: "", hue: "calm", spin: false, hover: false, ping: false, stuck: false, flare: false, working: false, touches: [], shown: true };
const repo = (files?: RepoIn["files"]): RepoIn => ({
  root: "/r",
  name: "r",
  pads: [],
  goal: true,
  areas: [{ dir: "crates", kids: [["fleet", 40, ["lib.rs", "main.rs"]], ["store", 3, []]] }],
  ...(files === undefined ? {} : { files }),
});
const SIZE = { width: 1000, height: 700 };

test("a plot's pills are the files Fleet counted, and the named files are the first of them", () => {
  const layout = layoutOf([repo()], [drone], SIZE, 1.3);
  const fleet = layout.territories[0]!.tiles.find((t) => t.path === "crates/fleet")!;
  expect(fleet.pills?.count).toBe(40);
  expect(fleet.pills?.names).toEqual(["lib.rs", "main.rs"]);
  const store = layout.territories[0]!.tiles.find((t) => t.path === "crates/store")!;
  expect(store.pills?.count).toBe(3);
});

test("a file with a name is lit on its own pill, by its index, and stays on that pill when another is named", () => {
  const one = layoutOf([repo([{ path: "crates/fleet/main.rs", act: "write" }])], [drone], SIZE, 1.3);
  const two = layoutOf([repo([{ path: "crates/fleet/main.rs", act: "write" }, { path: "crates/fleet/lib.rs", act: "edit" }])], [drone], SIZE, 1.3);
  expect(one.bars.get("crates/fleet/main.rs")?.pill).toBe(1);
  expect(two.bars.get("crates/fleet/main.rs")?.pill).toBe(1);
  expect(two.bars.get("crates/fleet/lib.rs")?.pill).toBe(0);
  expect(two.bars.get("crates/fleet/main.rs")?.rect).toEqual(one.bars.get("crates/fleet/main.rs")?.rect);
});
