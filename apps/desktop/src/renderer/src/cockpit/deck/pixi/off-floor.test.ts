// A Job's Field off the code floor, as the scene the canvas draws: the pad's blame line only where a check's record names a file the Field
// draws, a thinking drone risen over its last file with no beam, and a subagent lifting off the drone that sent it and flying back into it.

import { expect, test } from "vitest";

import { homeOf, marksOf, projectOf, reachOf } from "../camera";
import { frameOf } from "../frame";
import { FOOT, MARGIN } from "../geometry";
import { offFloorMarks } from "../off-floor";
import { FLAT, bigOf, layoutOf, type DroneIn, type RepoIn } from "../layout";
import type { Play } from "../plays";
import { padOf } from "./scene";
import { PANE, sceneFor } from "./scene-fixture";

const FILE = "crates/fleet/lib.rs";
const REPO: RepoIn = { root: "/r", name: "r", pads: [], goal: false, areas: [{ dir: "crates", kids: [["fleet", 40, ["lib.rs"]], ["store", 12, []]] }], files: [{ path: FILE }], pad: true };
const world = { width: PANE.width * 1.6, height: PANE.height * 1.6 };
const big = bigOf(PANE);

const drone = (key: string, extra: Partial<DroneIn> = {}): DroneIn => ({ key, kind: "session", title: key, root: "/r", said: "Working", hue: "calm", spin: true, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [{ path: FILE, w: 4 }], file: FILE, shown: true, ...extra });

test("a failing light draws its line to the file only where its record names one the Field draws", () => {
  const layout = layoutOf([REPO], [], world, big);
  const named = padOf(layout, [{ name: "fmt", state: "ok" }, { name: "test", state: "bad", blame: FILE }])!;
  expect(named.lights.map((one) => one.state)).toEqual(["ok", "bad"]);
  expect(named.blame).toEqual([{ from: named.lights[1]!.at, bar: FILE }]);
  expect(padOf(layout, [{ name: "test", state: "bad" }])!.blame).toEqual([]);
  expect(padOf(layout, [{ name: "test", state: "bad", blame: "crates/store/gone.rs" }])!.blame).toEqual([]);
  // A passing check blames nothing, whatever it names.
  expect(padOf(layout, [{ name: "test", state: "ok", blame: FILE }])!.blame).toEqual([]);
  // The pad lies off the floor: below every plot, toward the near edge.
  const pad = layout.territories[0]!.pad!;
  expect(Math.min(...layout.territories[0]!.tiles.map((one) => pad.y - (one.rect.y + one.rect.h)))).toBeGreaterThan(0);
});

test("a drone checking lands on the pad, and a thinking one stays over its last file, higher, with no beam", () => {
  const layout = (one: DroneIn) => layoutOf([REPO], [one], world, big);
  const writing = drone("session:a", { doing: "write" });
  const thinking = drone("session:a", { off: "think" });
  const checking = drone("session:a", { off: "pad", touches: [] });
  const bar = layout(writing).bars.get(FILE)!;
  expect(layout(thinking).at.get(thinking.key)).toMatchObject({ x: bar.cx, y: bar.cy });
  const pad = layout(checking).territories[0]!.pad!;
  expect(layout(checking).at.get(checking.key)).toMatchObject({ x: pad.x + pad.w / 2, y: pad.y + pad.h / 2 });

  const specOf = (one: DroneIn) => sceneFor({ drones: [one], layout: layout(one), prs: [], calls: new Map(), loops: new Set([one.key]) }).drones[0]!;
  const [wrote, thought] = [specOf(writing), specOf(thinking)];
  expect(wrote.aim).toBeDefined();
  expect(thought.aim).toBeUndefined();
  const { off: _off, ...still } = thinking;
  expect(thought.alt).toBeGreaterThan(frameOf({ layout: layout(still), drones: [still], plays: [], big }).altOf(still));
});

test("a subagent is drawn smaller, lifts off the drone that sent it, and flies back into it when it ends", () => {
  const main = drone("session:a", { doing: "write" });
  const sub = drone("session:a/s1", { parent: "session:a", touches: [{ path: "crates/store", w: 4 }], file: undefined });
  const drones = [main, sub];
  const layout = layoutOf([REPO], drones, world, big);
  const lift: Play = { id: "lift:session:a/s1:1", kind: "launch", key: sub.key, at: 1, delay: 0 };
  const dock: Play = { id: "dock:session:a/s1:2", kind: "dock", key: sub.key, at: 2, delay: 0 };
  const scene = (plays: Play[]) => sceneFor({ drones, layout, plays, prs: [], calls: new Map() });
  const lifting = scene([lift]);
  const own = lifting.drones.find((one) => one.drone.key === main.key)!;
  const child = lifting.drones.find((one) => one.drone.key === sub.key)!;
  expect(own.small).toBe(false);
  expect(child.small).toBe(true);
  expect(child.link).toEqual(own.flying);
  expect(child.launch?.from).toEqual(own.flying);
  const back = scene([dock]).drones.find((one) => one.drone.key === sub.key)!;
  expect(back.dock).toEqual({ id: dock.id, delay: 0, to: own.flying });
  expect(back.launch).toBeUndefined();
  expect(own.flying.y).toBeCloseTo((layout.at.get(main.key)!.y) * FLAT);
});

test("while the Job's drone is on the pad its own camera frames the pad, its lights and the file a failing check blames, and on the floor it does not", () => {
  const docked = drone("session:a", { off: "pad", touches: [] });
  const layout = layoutOf([REPO], [docked], world, big);
  const lights = [{ name: "fmt", state: "ok" as const }, { name: "test", state: "bad" as const, blame: FILE }];
  const marks = offFloorMarks({ layout, drones: [docked], lights, outposts: [], world });
  const pad = layout.territories[0]!.pad!;
  const bar = layout.bars.get(FILE)!;
  expect(marks).toContainEqual({ x: pad.x, y: pad.y * FLAT, z: 0 });
  expect(marks).toContainEqual({ x: pad.x + pad.w, y: (pad.y + pad.h + 16) * FLAT, z: 0 });
  expect(marks).toContainEqual({ x: bar.cx, y: bar.cy * FLAT, z: 20 });
  // The camera rested on those marks with the drone's own shows every corner of the pad.
  const at = layout.at.get(docked.key)!;
  const plane = { width: world.width, height: world.height * FLAT };
  const cam = homeOf(PANE, plane, [...marksOf([], [{ at, call: false }], big, reachOf(FOOT, MARGIN, big)), ...marks]);
  for (const x of [pad.x, pad.x + pad.w]) for (const y of [pad.y, pad.y + pad.h]) {
    const seen = projectOf(PANE, { x, y: y * FLAT, z: 0 }, cam);
    expect(Math.abs(seen.x)).toBeLessThanOrEqual(PANE.width / 2);
    expect(Math.abs(seen.y)).toBeLessThanOrEqual(PANE.height / 2);
  }
  // Calling, the outpost; back on the floor, nothing beside the drones.
  expect(offFloorMarks({ layout, drones: [drone("session:a", { calling: "forge" })], lights, outposts: [{ service: "forge" }], world })).toHaveLength(2);
  expect(offFloorMarks({ layout, drones: [drone("session:a")], lights, outposts: [{ service: "forge" }], world })).toEqual([]);
});
