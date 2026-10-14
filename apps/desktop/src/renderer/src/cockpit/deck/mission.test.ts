// A Job's mission scene: its route is its workflow, its plots are only the areas its drones were observed editing, and a
// clash is Fleet's, never two drones that happen to share a plot.

import type { WorkflowStepCardProps } from "@armada/components";
import { expect, test } from "vitest";

import type { DroneIn } from "./layout";
import { sceneOf, type Mission, type Waypoint } from "./mission";

const waypoint = (id: string, state: Waypoint["state"], here = false): Waypoint => ({ id, label: id, state, said: state, here, card: { kind: "step", name: id, activity: "not_started", said: state } as WorkflowStepCardProps });
const mission: Mission = { spine: [waypoint("step:plan", "done"), waypoint("step:work", "live", true), waypoint("step:review", "next")], fans: new Map(), returns: [], live: "step:work" };

const drone = (key: string, touches: DroneIn["touches"]): DroneIn => ({
  key,
  kind: key.startsWith("job:") ? "job" : "session",
  title: key,
  root: "/r",
  said: "",
  hue: "calm",
  spin: true,
  hover: true,
  ping: false,
  stuck: false,
  flare: false,
  working: true,
  touches,
  shown: true,
});

const scene = (drones: DroneIn[], clashes?: { path: string; root: string; keys: string[] }[]) =>
  sceneOf({ mission, repo: { root: "/r", name: "r" }, drones, job: "job:j1", serves: new Map(), pad: "slot-2", big: 1.3, wide: true, ...(clashes === undefined ? {} : { clashes }) });

test("a Job with no observed edit has a route and no plots, and its drone is over the step at work", () => {
  const { layout, route } = scene([drone("job:j1", [])]);

  expect(layout.territories[0]!.tiles).toEqual([]);
  expect(layout.over.get("job:j1")).toEqual([]);
  expect(route.pins.map((one) => one.id)).toEqual(["step:plan", "step:work", "step:review"]);
  expect(route.live).toEqual(route.pins[1]!.at);
});

test("the plots are the areas its drones were observed editing", () => {
  const { layout } = scene([drone("job:j1", [{ path: "crates/fleet", w: 4 }]), drone("session:s1", [{ path: "docs", w: 1 }])]);

  expect(layout.territories[0]!.tiles.map((one) => one.path).sort()).toEqual(["crates/fleet", "docs"]);
});

test("two drones over one plot are not a clash; Fleet's clash marks the plot, and only where the scene draws one", () => {
  const drones = [drone("job:j1", [{ path: "crates/fleet", w: 4 }]), drone("session:s1", [{ path: "crates/fleet", w: 1 }])];
  expect(scene(drones).layout.clashes).toEqual([]);

  const clash = { path: "crates/fleet", root: "/r", keys: ["job:j1", "session:s1"] };
  const elsewhere = { path: "apps/desktop", root: "/r", keys: ["job:j1"] };
  expect(scene(drones, [clash, elsewhere]).layout.clashes).toEqual([clash]);
});
