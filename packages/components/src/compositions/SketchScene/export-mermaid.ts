// A scene as Mermaid `flowchart` text, for somebody who pastes it where Mermaid draws. Mermaid lays the
// boxes out itself, so where they sit and the order they play in are not carried; what is: each box's
// words, each arrow and its words, and every group or lane that holds a box, as a `subgraph`.

import type { Scene, SceneColor, SceneNode, SceneShape } from "./scene";
import { screenGuess } from "./screen";

const GROUND = new Set(["group", "lane"]);
const DEFAULT_W = 240;
const DEFAULT_H = 90;

/**
 * Text for inside a quoted Mermaid label. Mermaid reads `#name;` and `#NN;` as entities inside a quote,
 * so `#` goes first and every character that would end the quote, open a tag or start markdown goes as one.
 */
export function mermaidText(text: string): string {
  return text
    .replace(/#/g, "#35;")
    .replace(/"/g, "#quot;")
    .replace(/&/g, "#amp;")
    .replace(/</g, "#lt;")
    .replace(/>/g, "#gt;")
    .replace(/`/g, "#96;")
    .replace(/\r?\n/g, "<br/>");
}

const quoted = (text: string) => `"${text}"`;

/** What a box says: its title over its body, or the words of a label. */
function words(node: SceneNode): string {
  if (node.kind === "label") return mermaidText(node.title ?? node.body ?? node.id);
  const head = node.title === undefined ? [] : [`<b>${mermaidText(node.title)}</b>`];
  const lang = node.lang === undefined ? [] : [mermaidText(node.lang)];
  const body = node.body === undefined || node.body === "" ? [] : [mermaidText(node.body)];
  const all = [...head, ...lang, ...body];
  return all.length === 0 ? mermaidText(node.id) : all.join("<br/>");
}

/** Mermaid's own outline for each of a box's shapes. */
const SHAPED: Record<Exclude<SceneShape, "box">, (label: string) => string> = {
  pill: (label) => `([${label}])`,
  circle: (label) => `((${label}))`,
  diamond: (label) => `{${label}}`,
  cylinder: (label) => `[(${label})]`,
  hexagon: (label) => `{{${label}}}`,
};

const frame = (node: SceneNode, text: string): string => {
  const label = quoted(text);
  if (node.kind === "box" && node.shape !== undefined && node.shape !== "box") return SHAPED[node.shape](label);
  if (node.kind === "code") return `[[${label}]]`;
  if (node.kind === "wire" || node.kind === "screen") return `([${label}])`;
  return `[${label}]`;
};

type Box = { x: number; y: number; r: number; b: number };
const boxOf = (node: SceneNode): Box => {
  const guess = node.screen === undefined ? { w: DEFAULT_W, h: DEFAULT_H } : screenGuess(node.screen, node.w);
  return { x: node.x, y: node.y, r: node.x + (node.w ?? guess.w), b: node.y + (node.h ?? guess.h) };
};
const area = (box: Box) => (box.r - box.x) * (box.b - box.y);
const holdsPoint = (box: Box, x: number, y: number) => x >= box.x && x <= box.r && y >= box.y && y <= box.b;
const holdsBox = (outer: Box, inner: Box) => inner.x >= outer.x && inner.y >= outer.y && inner.r <= outer.r && inner.b <= outer.b;

/** `LR` for a scene wider than it is tall, as most flows are drawn, and `TD` otherwise. */
function direction(scene: Scene): "LR" | "TD" {
  const boxes = scene.nodes.filter((one) => !GROUND.has(one.kind)).map(boxOf);
  if (boxes.length === 0) return "TD";
  const w = Math.max(...boxes.map((one) => one.r)) - Math.min(...boxes.map((one) => one.x));
  const h = Math.max(...boxes.map((one) => one.b)) - Math.min(...boxes.map((one) => one.y));
  return w > h * 1.2 ? "LR" : "TD";
}

/** `hues` is each colour's value in the theme it is made in; a colour with none is left to Mermaid's own. */
export function sceneMermaid(scene: Scene, hues: Partial<Record<SceneColor, string>> = {}): string {
  // Mermaid ids are its own: a scene's ids may be words it reserves (`end`) or hold characters it parses.
  const ids = new Map(scene.nodes.map((one, at) => [one.id, `n${String(at + 1)}`]));
  const grounds = scene.nodes.filter((one) => GROUND.has(one.kind));

  // The smallest ground that holds a thing is its parent: a box by its middle, a ground by all of it.
  const parentOf = (node: SceneNode): SceneNode | undefined => {
    const box = boxOf(node);
    const holds = grounds
      .filter((one) => one.id !== node.id)
      .filter((one) => (GROUND.has(node.kind) ? holdsBox(boxOf(one), box) && area(boxOf(one)) > area(box) : holdsPoint(boxOf(one), (box.x + box.r) / 2, (box.y + box.b) / 2)))
      .sort((a, b) => area(boxOf(a)) - area(boxOf(b)));
    return holds[0];
  };
  const children = new Map<string, SceneNode[]>();
  const adopt = (child: SceneNode) => {
    const parent = parentOf(child);
    if (parent !== undefined) children.set(parent.id, [...(children.get(parent.id) ?? []), child]);
    return parent;
  };
  const roots: SceneNode[] = [];
  for (const one of scene.nodes) if (adopt(one) === undefined) roots.push(one);

  // A ground is a `subgraph` where it holds something, and a plain box where an arrow joins it, and nothing otherwise.
  const joined = new Set(scene.edges.flatMap((one) => [one.from, one.to]));
  const holding = (node: SceneNode): boolean => (children.get(node.id) ?? []).some((one) => !GROUND.has(one.kind) || holding(one));
  const lines: string[] = [`flowchart ${direction(scene)}`];

  const emit = (node: SceneNode, depth: number) => {
    const pad = "  ".repeat(depth);
    const id = ids.get(node.id)!;
    if (!GROUND.has(node.kind)) {
      lines.push(`${pad}${id}${frame(node, words(node))}`);
      return;
    }
    if (!holding(node)) {
      if (joined.has(node.id)) lines.push(`${pad}${id}${frame(node, words({ ...node, kind: "box" }))}`);
      return;
    }
    lines.push(`${pad}subgraph ${id}[${quoted(mermaidText(node.title ?? node.body?.split("\n")[0] ?? node.id))}]`);
    for (const child of children.get(node.id) ?? []) emit(child, depth + 1);
    lines.push(`${pad}end`);
  };
  for (const one of roots) emit(one, 1);

  // `linkStyle` counts arrows in the order they are written, so the count is of what was written.
  const links: string[] = [];
  let written = 0;
  for (const edge of scene.edges) {
    const from = ids.get(edge.from);
    const to = ids.get(edge.to);
    if (from === undefined || to === undefined) continue;
    const text = edge.label === undefined || edge.label.trim() === "" ? undefined : mermaidText(edge.label.trim());
    lines.push(text === undefined ? `  ${from} --> ${to}` : `  ${from} -- ${quoted(text)} --> ${to}`);
    const hue = edge.color === undefined ? undefined : hues[edge.color];
    if (hue !== undefined) links.push(`  linkStyle ${String(written)} stroke:${hue}`);
    written += 1;
  }
  for (const node of scene.nodes) {
    const hue = node.color === undefined ? undefined : hues[node.color];
    const id = ids.get(node.id);
    if (hue !== undefined && id !== undefined && !GROUND.has(node.kind)) lines.push(`  style ${id} stroke:${hue}`);
  }
  lines.push(...links);
  return `${lines.join("\n")}\n`;
}
