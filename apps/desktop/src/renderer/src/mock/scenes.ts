// Scenes for the mock's sketches, authored with a few helpers. **Mock only**: a Drone would hand
// Fleet the same shape, and `parseScene` is what checks it where it is drawn.

import type { Scene, SceneEdge, SceneIcon, SceneNode, WirePart } from "@armada/components";

type More = { icon?: SceneIcon; step?: number; lang?: string };

const extra = ({ icon, step, lang }: More) => ({
  ...(icon === undefined ? {} : { icon }),
  ...(step === undefined ? {} : { step }),
  ...(lang === undefined ? {} : { lang }),
});

export const box = (id: string, x: number, y: number, title: string, body?: string, more: More = {}): SceneNode => ({
  id, kind: "box", x, y, title, ...(body === undefined ? {} : { body }), ...extra(more),
});
export const code = (id: string, x: number, y: number, lang: string, body: string, title?: string): SceneNode => ({
  id, kind: "code", x, y, lang, body, ...(title === undefined ? {} : { title }),
});
export const group = (id: string, x: number, y: number, w: number, h: number, title: string): SceneNode => ({ id, kind: "group", x, y, w, h, title });
export const lane = (id: string, x: number, y: number, w: number, h: number, title: string, icon?: SceneIcon): SceneNode => ({
  id, kind: "lane", x, y, w, h, title, ...(icon === undefined ? {} : { icon }),
});
export const label = (id: string, x: number, y: number, title: string): SceneNode => ({ id, kind: "label", x, y, title });
export const wire = (id: string, x: number, y: number, part: WirePart, title: string): SceneNode => ({ id, kind: "wire", x, y, wire: part, title });
export const arrow = (from: string, to: string, text?: string, flow?: boolean): SceneEdge => ({
  id: `${from}-${to}`, from, to, ...(text === undefined ? {} : { label: text }), ...(flow === undefined ? {} : { flow }),
});
export const scene = (nodes: SceneNode[], edges: SceneEdge[]): Scene => ({ nodes, edges });

// ── Decision 1: where the clock lives ────────────────────────────────────────────────────────────

const caller = box("caller", -340, 60, "Caller", "append(batch)", { icon: "terminal", step: 1 });
const writer = box("writer", 0, 60, "Writer", "write(batch)\nnow()  reads the wall clock", { icon: "hard-drive", step: 2 });
const disk = box("disk", 340, 60, "Record file", "one line per batch", { icon: "file", step: 3 });
const crate = (h: number) => group("crate", -20, 10, 280, h, "store crate");

export const SHAPE_NOW = scene(
  [label("title", -340, -40, "Clock and writer today"), crate(170), caller, writer, disk],
  [arrow("caller", "writer", "append"), arrow("writer", "disk", "flush")],
);

export const SHAPE_SPLIT = scene(
  [
    label("title", -340, -40, "Clock and writer today"),
    crate(330),
    caller,
    { ...writer, body: "write(batch, hour)\ntakes the hour it is given" },
    disk,
    box("clock", 0, 230, "Clock", "now() -> Hour\nthe only reader of the wall", { icon: "clock", step: 2 }),
  ],
  [arrow("caller", "writer", "append"), arrow("writer", "disk", "flush"), arrow("clock", "writer", "hour")],
);

export const SHAPE_WRAP = scene(
  [
    label("title", -340, -40, "Clock and writer today"),
    crate(330),
    caller,
    writer,
    disk,
    box("wrapper", 0, 230, "Wrapper", "wraps Writer\nnow() stays inside, behind a trait", { icon: "layers", step: 2 }),
  ],
  [arrow("caller", "wrapper", "append"), arrow("wrapper", "writer", "delegates"), arrow("writer", "disk", "flush")],
);

export const SHAPE_LATER = scene(
  [
    label("title", -340, -40, "Clock and writer today"),
    crate(170),
    caller,
    writer,
    disk,
    box("later", 340, 200, "A later Job", "split the clock out", { icon: "rocket" }),
  ],
  [arrow("caller", "writer", "append"), arrow("writer", "disk", "flush"), arrow("later", "writer", "touches", false)],
);

// ── Decision 2: how the tests hold the hour ──────────────────────────────────────────────────────

const lanes = [lane("l-fixture", -40, 0, 940, 150, "Fixture"), lane("l-store", -40, 170, 940, 150, "Store", "hard-drive")];
const fixture = box("fx", 0, 50, "Fixture", "calls writer.append", { icon: "terminal", step: 1 });
const asserts = box("ass", 640, 50, "Assertion", "line carries the hour", { icon: "scale", step: 3 });
const lanesOk = lanes.map((one, at) => (at === 0 ? { ...one, icon: "terminal" as const } : one));

export const TESTS_NOW = scene(
  [...lanesOk, fixture, box("wall", 330, 220, "Wall clock", "reads the hour as it runs", { icon: "clock", step: 2 }), asserts],
  [arrow("fx", "wall", "reads"), arrow("wall", "ass", "flaky near the hour")],
);

export const TESTS_PIN = scene(
  [...lanesOk, fixture, box("wall", 330, 220, "Pinned hour", "returns 02:00\nin every test", { icon: "lock", step: 2 }), asserts],
  [arrow("fx", "wall", "reads"), arrow("wall", "ass", "stable")],
);

export const TESTS_FAKE = scene(
  [...lanesOk, fixture, box("fake", 330, 220, "Fake clock", "tick(n) moves it on\nthe test says when", { icon: "clock", step: 2 }), asserts],
  [arrow("fx", "fake", "ticks"), arrow("fake", "ass", "stable")],
);

// ── The Judge's question: a retry loop with a cap ────────────────────────────────────────────────

export const BACKOFF = scene(
  [
    group("loop", -30, -20, 700, 300, "retry loop"),
    box("wait", 0, 40, "Waiting", "sleep(backoff)", { icon: "clock", step: 1 }),
    box("retry", 400, 40, "Retrying", "send(batch)\nwait doubles on a miss", { icon: "webhook", step: 2 }),
    box("capped", 400, 190, "Capped", "backoff = CAP", { icon: "lock", step: 3 }),
    box("failed", 0, 190, "Failed", "give up, say why", { icon: "shield-check", step: 4 }),
    code("snippet", 740, 20, "rust", "let next = (prev * 2)\n    .min(CAP);", "backoff"),
    wire("nav", 740, 170, "nav", "Settings"),
    wire("input", 740, 225, "input", "Retry cap"),
    wire("button", 740, 280, "button", "Save"),
  ],
  [arrow("wait", "retry", "wakes"), arrow("retry", "capped", "at the cap"), arrow("capped", "failed", "fails")],
);
