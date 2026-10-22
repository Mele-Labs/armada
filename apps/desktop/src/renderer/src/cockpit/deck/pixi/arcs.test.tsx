// The arc between this Job's drone and another Job's while one messages the other, as the scene the canvas draws: from the sender's drone to
// the receiver's, none where either drone is not on the field, the other drone at full strength with its name while linked, and the
// packets still under reduced motion. Browser: the drone is drawn by the real Pixi picture.

import { expect, test } from "vitest";

import type { LinkIn } from "../off-floor";
import type { DroneIn } from "../layout";
import { arcsMoving, packetsAt } from "./arcs";
import { GHOST, alphaOf, drawDrone } from "./drones";
import { readPalette } from "./palette";
import { DRONES, sceneFor } from "./scene-fixture";

const OTHER: DroneIn = { key: "session:Other", kind: "session", title: "Other", root: "/r", said: "Working", hue: "calm", spin: true, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [{ path: "crates/store", w: 4 }], shown: true, ghost: true };
const OWN = DRONES[0]!.key;
const link = (sent: boolean): LinkIn => ({ key: "message:Other", from: sent ? OWN : OTHER.key, to: sent ? OTHER.key : OWN, other: OTHER.key, title: "Other", quote: "Is the index rebuilt?", sent });

test("the arc runs from the sender's drone to the receiver's, and none is drawn where the other Job has no drone on the field", () => {
  const drones = [...DRONES, { ...OTHER, linked: true as const }];
  const sending = sceneFor({ drones, links: [link(true)] });
  const own = sending.drones.find((one) => one.drone.key === OWN)!;
  const other = sending.drones.find((one) => one.drone.key === OTHER.key)!;
  expect(sending.arcs).toHaveLength(1);
  expect(sending.arcs[0]).toMatchObject({ other: OTHER.key, from: own.flying, to: other.flying, sent: true });
  expect(sending.arcs[0]!.mid.z).toBeGreaterThan(Math.max(own.flying.z, other.flying.z));
  const replying = sceneFor({ drones, links: [link(false)] }).arcs[0]!;
  expect([replying.from, replying.to]).toEqual([other.flying, own.flying]);
  expect(sceneFor({ drones: DRONES, links: [link(true)] }).arcs).toEqual([]);
  expect(sceneFor({ drones }).arcs).toEqual([]);
});

test("the other Job's drone comes up to full strength with its name while linked, and is faded again without the link", () => {
  const host = document.createElement("div");
  document.body.append(host);
  const palette = readPalette(host);
  const specOf = (one: DroneIn) => sceneFor({ drones: [...DRONES, one], labels: new Set([OTHER.key]) }).drones.find((it) => it.drone.key === OTHER.key)!;
  const linked = specOf({ ...OTHER, linked: true });
  const faded = specOf(OTHER);
  expect(linked.ghost).toBe(false);
  expect(alphaOf(linked)).toBe(1);
  expect(faded.ghost).toBe(true);
  expect(alphaOf(faded)).toBe(GHOST);
  // Still giving way: it raises no plot and aims no action.
  expect(linked.touches).toEqual([]);
  expect(linked.aim).toBeUndefined();
  // The shell and its name beside it, where a faded drone has the shell alone.
  expect(drawDrone(linked, 1, palette).body.children.length).toBe(2);
  expect(drawDrone(faded, 1, palette).body.children.length).toBe(1);
  host.remove();
});

test("the packets travel only while the arc loops and motion is on, and under reduced motion stand still, spread along it", () => {
  const drones = [...DRONES, { ...OTHER, linked: true as const }];
  const looping = sceneFor({ drones, links: [link(true)], linkLoops: new Set(["message:Other"]) });
  const arc = looping.arcs[0]!;
  expect(arc.loop).toBe(true);
  const still = { pulse: 0, since: () => 0 };
  expect(arcsMoving(looping, { ...still, now: 1000 })).toBe(false);
  expect(packetsAt(arc, { now: 1000, pulse: 0 })).toEqual(packetsAt(arc, { now: 2200, pulse: 0 }));
  expect(packetsAt(arc, { now: 1000, pulse: 0 }).map((t) => Math.round(t * 6))).toEqual([1, 3, 5]);
  expect(arcsMoving(looping, { ...still, now: 1000, pulse: 1200 })).toBe(true);
  expect(packetsAt(arc, { now: 1000, pulse: 1200 })).not.toEqual(packetsAt(arc, { now: 1600, pulse: 1200 }));
  // An arc that is not one of the loops stands still with motion on.
  const resting = sceneFor({ drones, links: [link(true)] });
  expect(arcsMoving(resting, { ...still, now: 1000, pulse: 1200 })).toBe(false);
  expect(packetsAt(resting.arcs[0]!, { now: 1000, pulse: 1200 })).toEqual(packetsAt(arc, { now: 1000, pulse: 0 }));
});
