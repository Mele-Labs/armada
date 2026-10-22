// Another Job's drone on a Job's own Field: drawn at a quarter strength, with no beam, label or action, raising no plot, and giving way
// to the Job's own drones. Browser: the drone is drawn by the real Pixi picture.

import { expect, test } from "vitest";

import { frameOf } from "../frame";
import { SLAB, bigOf, layoutOf, type DroneIn } from "../layout";
import { GHOST, alphaOf, drawDrone } from "./drones";
import { DRONES, PANE, REPO, sceneFor } from "./scene-fixture";
import { readPalette } from "./palette";

const OTHER: DroneIn = { key: "session:Other", kind: "session", title: "Other", root: "/r", said: "Working", hue: "calm", spin: true, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [{ path: "crates/store", w: 4 }], shown: true, ghost: true };

test("another Job's drone is drawn faded, with no label, no beam line and no action, and its plot does not rise or light", () => {
  const drones = [...DRONES, OTHER];
  const scene = sceneFor({ drones, labels: new Set(["session:Other"]), loops: new Set(["session:Other"]) });
  const spec = scene.drones.find((one) => one.drone.key === "session:Other")!;
  const own = scene.drones.find((one) => one.drone.key === "session:Writer")!;
  expect(spec.ghost).toBe(true);
  expect(own.ghost).toBe(false);
  expect(alphaOf(spec)).toBe(GHOST);
  expect(alphaOf(own)).toBe(1);
  expect(spec.touches).toEqual([]);
  expect(spec.aim).toBeUndefined();
  expect(scene.lit.has("crates/store")).toBe(false);
  expect(scene.heights.get("crates/store")).toBe(SLAB.low);

  const host = document.createElement("div");
  document.body.append(host);
  const parts = drawDrone(spec, scene.big, readPalette(host));
  host.remove();
  expect(parts.body.alpha).toBe(GHOST);
  // The shell and nothing beside it: no name.
  expect(parts.body.children.length).toBe(1);
});

test("a faded drone gives way to the Job's own, which stays where its edits put it", () => {
  const world = { width: PANE.width * 1.6, height: PANE.height * 1.6 };
  const big = bigOf(PANE);
  const writer = DRONES[0]!;
  const alone = layoutOf([REPO], [writer], world, big).at.get(writer.key)!;
  const crowded = layoutOf([REPO], [writer, { ...OTHER, touches: writer.touches }], world, big);
  expect(crowded.at.get(writer.key)).toEqual(alone);
  expect(crowded.at.get(OTHER.key)).not.toEqual(alone);
  expect(frameOf({ layout: crowded, drones: [writer, OTHER], plays: [], big }).doers.map((one) => one.key)).toEqual([writer.key]);
});
