// Which names the map draws: a drone's, and a plot's. No two meet, and none lies over a drone that is not the one it names, except
// the name of the drone that is picked or under the pointer, which is always drawn.

import { expect, test } from "vitest";

import { bodiesOf, labelsOf, meets, plotNamed, type Box } from "./labels";
import type { DroneIn } from "./layout";

const drone = (key: string, extra: Partial<DroneIn> = {}): DroneIn => ({
  key,
  kind: "session",
  title: `Title of ${key}`,
  root: "/r",
  said: "Waiting",
  hue: "ask",
  spin: false,
  hover: false,
  ping: true,
  stuck: false,
  flare: false,
  working: false,
  touches: [],
  shown: true,
  ...extra,
});
const at = (map: Record<string, [number, number]>) => (key: string) => (map[key] === undefined ? undefined : { x: map[key]![0], y: map[key]![1], up: 40 });

test("two names that would meet are not both drawn, the one that needs him more is", () => {
  const drones = [drone("session:a"), drone("session:b", { hue: "bad", flare: true })];
  const place = at({ "session:a": [100, 200], "session:b": [110, 200] });
  const bodies = bodiesOf(drones, place, 1);
  const { labels, taken } = labelsOf({ drones, place, bodies, big: 1, readable: true, selected: undefined, hot: undefined });

  expect(labels.size).toBe(1);
  expect(labels.has("session:b")).toBe(true);
  for (let i = 0; i < taken.length; i += 1) for (let j = i + 1; j < taken.length; j += 1) expect(meets(taken[i]!, taken[j]!)).toBe(false);
});

test("the picked drone is always named", () => {
  const drones = [drone("session:a", { hue: "calm", ping: false }), drone("session:b")];
  const place = at({ "session:a": [100, 200], "session:b": [105, 200] });
  const bodies = bodiesOf(drones, place, 1);
  const { labels } = labelsOf({ drones, place, bodies, big: 1, readable: true, selected: "session:a", hot: undefined });
  expect(labels.has("session:a")).toBe(true);
});

test("no names at all while the type is too small to read, and a calm drone is named only when picked or hovered", () => {
  const drones = [drone("session:a"), drone("session:c", { hue: "calm", ping: false })];
  const place = at({ "session:a": [100, 200], "session:c": [400, 200] });
  const bodies = bodiesOf(drones, place, 1);
  expect(labelsOf({ drones, place, bodies, big: 1, readable: false, selected: "session:a", hot: undefined }).labels.size).toBe(0);
  const named = labelsOf({ drones, place, bodies, big: 1, readable: true, selected: undefined, hot: undefined }).labels;
  expect(named.has("session:a")).toBe(true);
  expect(named.has("session:c")).toBe(false);
});

test("a plot's name is drawn only where it meets no drone and no name already drawn, and always while the pointer is on it", () => {
  const body: Box = { l: 0, r: 100, t: 0, b: 50 };
  const clear: Box = { l: 200, r: 260, t: 0, b: 16 };
  const over: Box = { l: 40, r: 100, t: 10, b: 26 };
  expect(plotNamed({ box: clear, wanted: true, readable: true, hot: false, bodies: [{ ...body }], taken: [] })).toBe(true);
  expect(plotNamed({ box: over, wanted: true, readable: true, hot: false, bodies: [{ ...body }], taken: [] })).toBe(false);
  expect(plotNamed({ box: over, wanted: true, readable: true, hot: true, bodies: [{ ...body }], taken: [] })).toBe(true);
  expect(plotNamed({ box: clear, wanted: true, readable: false, hot: false, bodies: [], taken: [] })).toBe(false);
  expect(plotNamed({ box: clear, wanted: false, readable: true, hot: false, bodies: [], taken: [] })).toBe(false);
  expect(plotNamed({ box: clear, wanted: true, readable: true, hot: false, bodies: [], taken: [{ ...clear, l: 210 }] })).toBe(false);
});
