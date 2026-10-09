// The scene a sketch is drawn from, whoever drew it. **One format for a Drone's sketch and for the
// owner's pad**: a scene is the pad's Drawing (boxes, joins, hand-drawn strokes) with the kinds
// the pad has no way to say — a group, a swimlane, a code snippet, a wireframe part, an icon, a
// step order. `sceneOfDrawing` takes a pad's drawing as it is, so nothing the pad can draw is
// outside this.
//
// A scene arrives as `unknown` (a Drone wrote it) and `parseScene` is the only way in. **A scene
// that does not validate is `undefined` and draws nothing**: no partial drawing, no repaired one.

import type { SketchPoint, SketchStroke } from "../SketchPad/Ink";

export const SCENE_KINDS = ["box", "group", "lane", "label", "code", "wire"] as const;
export type SceneKind = (typeof SCENE_KINDS)[number];

/** What a wireframe node stands for, drawn as a block and never as a control. */
export const WIRE_PARTS = ["button", "input", "list", "card", "nav", "text"] as const;
export type WirePart = (typeof WIRE_PARTS)[number];

/** Registry glyphs a node may carry, by registry name. Anything else fails the scene. */
export const SCENE_ICONS = [
  "activity", "bot", "box", "clock", "cpu", "eye", "file", "git-branch", "git-merge", "globe", "hammer", "hard-drive",
  "layers", "lock", "package", "rocket", "scale", "scroll-text", "server", "settings", "shield-check", "terminal",
  "webhook", "wrench", "zap",
] as const;
export type SceneIcon = (typeof SCENE_ICONS)[number];

export type SceneNode = {
  id: string;
  kind: SceneKind;
  x: number;
  y: number;
  /** A group and a lane are drawn at this size; the rest take their words' height. */
  w?: number;
  h?: number;
  title?: string;
  body?: string;
  icon?: SceneIcon;
  /** The language a code node is read as, in its header band. */
  lang?: string;
  wire?: WirePart;
  /** Where the node comes in the order the scene plays its steps. */
  step?: number;
};

export type SceneEdge = { id: string; from: string; to: string; label?: string; flow?: boolean };

export type Scene = {
  nodes: readonly SceneNode[];
  edges: readonly SceneEdge[];
  strokes?: readonly SketchStroke[];
};

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const isText = (value: unknown): value is string => typeof value === "string";
const isNumber = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
const optional = <T>(value: unknown, is: (one: unknown) => one is T): boolean => value === undefined || is(value);
const oneOf = <T extends string>(list: readonly T[]) => (value: unknown): value is T => isText(value) && (list as readonly string[]).includes(value);

function nodeOf(value: unknown): SceneNode | undefined {
  if (!isRecord(value)) return undefined;
  const { id, kind, x, y, w, h, title, body, icon, lang, wire, step } = value;
  if (!isText(id) || id === "" || !oneOf(SCENE_KINDS)(kind) || !isNumber(x) || !isNumber(y)) return undefined;
  if (!optional(w, isNumber) || !optional(h, isNumber) || !optional(title, isText) || !optional(body, isText)) return undefined;
  if (!optional(icon, oneOf(SCENE_ICONS)) || !optional(lang, isText) || !optional(wire, oneOf(WIRE_PARTS)) || !optional(step, isNumber)) return undefined;
  if ((kind === "group" || kind === "lane") && (!isNumber(w) || !isNumber(h))) return undefined;
  return {
    id, kind, x, y,
    ...(w === undefined ? {} : { w: w as number }),
    ...(h === undefined ? {} : { h: h as number }),
    ...(title === undefined ? {} : { title: title as string }),
    ...(body === undefined ? {} : { body: body as string }),
    ...(icon === undefined ? {} : { icon: icon as SceneIcon }),
    ...(lang === undefined ? {} : { lang: lang as string }),
    ...(wire === undefined ? {} : { wire: wire as WirePart }),
    ...(step === undefined ? {} : { step: step as number }),
  };
}

function edgeOf(value: unknown): SceneEdge | undefined {
  if (!isRecord(value)) return undefined;
  const { id, from, to, label, flow } = value;
  if (!isText(id) || id === "" || !isText(from) || !isText(to)) return undefined;
  if (!optional(label, isText) || !(flow === undefined || typeof flow === "boolean")) return undefined;
  return { id, from, to, ...(label === undefined ? {} : { label: label as string }), ...(flow === undefined ? {} : { flow }) };
}

function strokeOf(value: unknown): SketchStroke | undefined {
  if (!isRecord(value) || !isText(value.id) || !Array.isArray(value.points)) return undefined;
  const points: SketchPoint[] = [];
  for (const point of value.points) {
    if (!isRecord(point) || !isNumber(point.x) || !isNumber(point.y)) return undefined;
    points.push({ x: point.x, y: point.y });
  }
  return { id: value.id, points };
}

/** All of it or nothing. Ids are unique per list and every edge joins two nodes the scene holds. */
export function parseScene(input: unknown): Scene | undefined {
  if (!isRecord(input) || !Array.isArray(input.nodes) || !Array.isArray(input.edges)) return undefined;
  const nodes = input.nodes.map(nodeOf);
  const edges = input.edges.map(edgeOf);
  if (nodes.some((one) => one === undefined) || edges.some((one) => one === undefined)) return undefined;
  const kept = nodes as SceneNode[];
  const joined = edges as SceneEdge[];
  const ids = new Set(kept.map((one) => one.id));
  if (ids.size !== kept.length || new Set(joined.map((one) => one.id)).size !== joined.length) return undefined;
  if (joined.some((one) => !ids.has(one.from) || !ids.has(one.to))) return undefined;
  let strokes: SketchStroke[] | undefined;
  if (input.strokes !== undefined) {
    if (!Array.isArray(input.strokes)) return undefined;
    const read = input.strokes.map(strokeOf);
    if (read.some((one) => one === undefined)) return undefined;
    strokes = read as SketchStroke[];
  }
  return { nodes: kept, edges: joined, ...(strokes === undefined ? {} : { strokes }) };
}

/** The pad's own drawing, as a scene: its boxes are box nodes and its joins are edges. */
export function sceneOfDrawing(drawing: {
  shapes: readonly { id: string; x: number; y: number; body: string }[];
  joins: readonly { id: string; from: string; to: string }[];
  strokes: readonly SketchStroke[];
}): Scene {
  return {
    nodes: drawing.shapes.map((one) => ({ id: one.id, kind: "box", x: one.x, y: one.y, body: one.body })),
    edges: drawing.joins.map((one) => ({ id: one.id, from: one.from, to: one.to })),
    strokes: drawing.strokes,
  };
}

export type SceneChange = "added" | "removed" | "changed" | "same";

export type SceneDiff = {
  /** The after scene with what the before scene had and the after lacks, kept where it was. */
  scene: Scene;
  nodes: ReadonlyMap<string, SceneChange>;
  edges: ReadonlyMap<string, SceneChange>;
};

const same = (a: object, b: object): boolean => JSON.stringify(a) === JSON.stringify(b);

/** `after` against `before`, by node and edge id. A node moved or reworded is changed. */
export function diffScenes(before: Scene, after: Scene): SceneDiff {
  const had = new Map(before.nodes.map((one) => [one.id, one]));
  const has = new Map(after.nodes.map((one) => [one.id, one]));
  const hadEdges = new Map(before.edges.map((one) => [one.id, one]));
  const hasEdges = new Map(after.edges.map((one) => [one.id, one]));
  const nodes = new Map<string, SceneChange>();
  const edges = new Map<string, SceneChange>();
  for (const one of after.nodes) {
    const was = had.get(one.id);
    nodes.set(one.id, was === undefined ? "added" : same(was, one) ? "same" : "changed");
  }
  for (const one of before.nodes) if (!has.has(one.id)) nodes.set(one.id, "removed");
  for (const one of after.edges) {
    const was = hadEdges.get(one.id);
    edges.set(one.id, was === undefined ? "added" : same(was, one) ? "same" : "changed");
  }
  for (const one of before.edges) if (!hasEdges.has(one.id)) edges.set(one.id, "removed");
  return {
    scene: {
      nodes: [...after.nodes, ...before.nodes.filter((one) => !has.has(one.id))],
      edges: [...after.edges, ...before.edges.filter((one) => !hasEdges.has(one.id))],
      ...(after.strokes === undefined ? {} : { strokes: after.strokes }),
    },
    nodes,
    edges,
  };
}

/** What a part is called when somebody asks about it. */
export function partLabel(scene: Scene, id: string): string {
  const node = scene.nodes.find((one) => one.id === id);
  if (node !== undefined) return node.title ?? node.body?.split("\n")[0] ?? node.id;
  const edge = scene.edges.find((one) => one.id === id);
  if (edge === undefined) return id;
  const name = (end: string) => partLabel(scene, end);
  return edge.label ?? `${name(edge.from)} to ${name(edge.to)}`;
}
