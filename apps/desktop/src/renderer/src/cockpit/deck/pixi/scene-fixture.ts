// A small field read into a canvas scene the way `Battlefield.tsx` reads one, for the tests of the stage and of the hit layer: one
// repository with two plots, two bays, a flag for main and a pull request on its route, and two Sessions, one writing and one holding a call.

import { ShieldCheck } from "lucide-react";

import { frameOf } from "../frame";
import { FLAT, bigOf, layoutOf, type DroneIn, type PrIn, type RepoIn, type Size } from "../layout";
import type { PadIn } from "../tip";
import { sceneOf, type PixiSceneIn } from "./scene";

export const PANE: Size = { width: 900, height: 560 };

const drone = (key: string, extra: Partial<DroneIn>): DroneIn => ({
  key,
  kind: "session",
  title: key.replace("session:", ""),
  root: "/r",
  said: "Working",
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

export const REPO: RepoIn = {
  root: "/r",
  name: "r",
  pads: ["slot-1", "slot-2"],
  goal: true,
  areas: [{ dir: "crates", kids: [["fleet", 40, ["lib.rs"]], ["store", 12, []]] }],
};

export const DRONES: DroneIn[] = [
  drone("session:Writer", { doing: "write", spin: true, hover: true, working: true, slot: "slot-1", touches: [{ path: "crates/fleet", w: 1 }] }),
  drone("session:Caller", { said: "Needs you", hue: "ask", ping: true, slot: "slot-2" }),
];

export const PRS: PrIn[] = [{ key: "pr:7", number: 7, stage: "checking", root: "/r" } as PrIn];

export const PADS = new Map<string, PadIn>([
  ["slot-1", { key: "slot-1", slot: 1, bay: "held", word: "Held", repo: "r", Glyph: ShieldCheck }],
  ["slot-2", { key: "slot-2", slot: 2, bay: "held", word: "Held", repo: "r", Glyph: ShieldCheck }],
]);

/** The scene of that field, with each part of what the page hands the canvas open to change. */
export function sceneFor({ size = PANE, ...over }: Partial<Parameters<typeof sceneOf>[0]> & { size?: Size } = {}): PixiSceneIn {
  const world = { width: size.width * 1.6, height: size.height * 1.6 };
  const big = bigOf(size);
  const drones = over.drones ?? DRONES;
  const prs = over.prs ?? PRS;
  const plays = over.plays ?? [];
  const layout = over.layout ?? layoutOf([REPO], drones, world, big, prs, []);
  const frame = over.frame ?? frameOf({ layout, drones, plays, big });
  const calls = over.calls ?? new Map([["session:Caller", "ask" as const]]);
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, { x: pad.x, y: pad.y * FLAT }] as const)));
  return sceneOf({
    layout,
    world,
    big,
    frame,
    drones,
    pads: PADS,
    goals: new Map([["/r", { root: "/r", said: "Green", red: false }]]),
    prs,
    plays,
    readable: true,
    builds: [],
    selected: undefined,
    hot: undefined,
    hotTile: undefined,
    hotPart: undefined,
    names: [],
    focus: undefined,
    labels: new Set(),
    clashed: new Map(),
    calls,
    callOf: (key) => calls.get(key),
    loops: new Set(),
    locked: false,
    trails: new Map(),
    padAt,
    ...over,
  });
}
