// The plots a Session's plan names reach the canvas scene, and the card of such a plot says what the plan holds there. Pure: the scene and
// the pointer's lookup, with nothing drawn.

import { expect, test } from "vitest";

import { FLAT } from "../layout";
import { plateAt } from "./plates";
import { project, viewOf } from "./project";
import { PANE, sceneFor } from "./scene-fixture";

const planned = new Map([["crates/store", { piece: "scaffold" as const, said: "Show the rotation · Working" }]]);
const view = viewOf({ x: 0, y: 0, eye: 0.6, tilt: false }, PANE);

function overStore(scene: ReturnType<typeof sceneFor>) {
  const tile = scene.layout.territories[0]!.tiles.find((one) => one.path === "crates/store")!;
  const at = project(view, { x: tile.cx, y: tile.cy * FLAT, z: scene.heights.get(tile.path) ?? 0 });
  return plateAt(scene, view, at.x, at.y);
}

test("a scene carries the plots the plan names, and none where it names none", () => {
  expect(sceneFor({ planned }).planned).toBe(planned);
  expect("planned" in sceneFor()).toBe(false);
});

test("the card of a plot the plan names says the group and how it stands", () => {
  expect(overStore(sceneFor({ planned }))?.tip).toMatchObject({ label: "Directory", title: "crates/store", lines: ["Show the rotation · Working"] });
});

test("the card of a plot the plan does not name is the plain directory's", () => {
  const tip = overStore(sceneFor())?.tip;
  expect(tip).toMatchObject({ label: "Directory", title: "crates/store" });
  expect(tip?.lines).toBeUndefined();
});
