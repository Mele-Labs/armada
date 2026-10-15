// What the battlefield works out for a frame: a drone flies clear of the tallest plot in reach, and a drone aims only at a plot its
// observed edits name. A drone with no known path has nothing to aim at.

import { expect, test } from "vitest";

import { frameOf } from "./frame";
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

const repo: RepoIn = { root: "/r", name: "r", pads: ["slot-1", "slot-2"], goal: true, areas: [{ dir: "crates", kids: [["fleet", 100]] }, { dir: "docs", kids: [] }] };

test("a drone with no known path aims at nothing, and a working one aims at the plot its edits name", () => {
  const resting = drone("session:rest", { slot: "slot-1", doing: "write" });
  const editing = drone("session:edit", { slot: "slot-2", doing: "write", working: true, touches: [{ path: "crates/fleet", weight: 3 }] });
  const layout = layoutOf([repo], [resting, editing], SIZE, BIG);
  const frame = frameOf({ layout, drones: [resting, editing], plays: [], big: BIG });

  expect(frame.aimOf(resting)).toBeUndefined();
  expect(frame.aimOf(editing)?.path).toBe("crates/fleet");
  expect(frame.doers.map((one) => one.key)).toEqual(["session:edit"]);
});

test("a hidden drone is not a doer, and a drone flies higher while it hovers than while it rests", () => {
  const hidden = drone("session:hid", { doing: "run", shown: false, touches: [{ path: "crates/fleet", weight: 1 }] });
  const rests = drone("session:a", { slot: "slot-1" });
  const hovers = drone("session:b", { slot: "slot-1", hover: true });
  const layout = layoutOf([repo], [hidden, rests, hovers], SIZE, BIG);
  const frame = frameOf({ layout, drones: [hidden, rests, hovers], plays: [], big: BIG });

  expect(frame.doers).toEqual([]);
  expect(frame.altOf(hovers)).toBeGreaterThan(frame.altOf(rests));
});

test("the mission's own drone hovers clear of the pin it is over", () => {
  const job = drone("job:j", { kind: "job", slot: "slot-1" });
  const layout = layoutOf([repo], [job], SIZE, BIG);
  const plain = frameOf({ layout, drones: [job], plays: [], big: BIG });
  const mission = frameOf({ layout, drones: [job], plays: [], big: BIG, missionJob: "job:j" });

  expect(mission.altOf(job)).toBeGreaterThan(plain.altOf(job));
});
