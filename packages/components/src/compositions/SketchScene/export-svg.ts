// A scene as one SVG string, still, in the colours the tokens hold when it is made. **Pure**: it is handed
// where every box and arrow ended up (what `SketchScene` measured) and draws exactly that, so the picture
// is the drawing as it stands. No external address anywhere in it: the CSS is inline and every colour,
// length and face is a token value written into the file.
//
// Nothing here moves. The web page (`export-html.ts`) lays its player over the same markup, which is why
// every node and arrow carries `data-node` / `data-edge`.

import { pathAt, type LabelPlace, type Rect } from "./edge-labels";
import { CHANGE_SAID, partLabel, type Scene, type SceneChange, type SceneNode } from "./scene";
import { screenSteps } from "./screen";
import { pathOf } from "../SketchPad/ink-path";

/** The tokens the drawing reads, by name. The caller resolves each against the theme in force. */
export const EXPORT_TOKENS = [
  "--bg-base", "--bg-sunken", "--bg-raised", "--bg-hover", "--fg-default", "--fg-muted", "--fg-inverse", "--border-subtle", "--border-default", "--border-strong",
  "--status-running", "--status-not-started", "--status-completed-success", "--status-completed-failed", "--status-awaiting-review", "--status-rejected",
  "--edge-focus", "--edge-active", "--row-tint-recent", "--font-mono", "--font-sans", "--text-xs", "--text-sm", "--text-base", "--leading-xs", "--leading-sm",
  "--space-1", "--space-2", "--space-3", "--space-4", "--space-6", "--weight-heading", "--tracking-caps", "--radius-sm", "--radius-md", "--radius-lg", "--ease", "--duration-base",
  // What a screen's own stylesheet reads, which the export writes in beside its markup.
  "--border-width", "--space-8", "--space-12", "--text-2xs", "--text-xl", "--leading-2xs", "--leading-base", "--leading-xl", "--weight-medium", "--weight-body",
  "--h-control-sm", "--h-badge",
] as const;

/** The lengths the layout needs as numbers, in px: read from the tokens, or these where one cannot be read. */
export type ExportMetrics = {
  textXs: number;
  textSm: number;
  leadingXs: number;
  leadingSm: number;
  space1: number;
  space2: number;
  space3: number;
  edge: number;
  radiusSm: number;
  radiusMd: number;
  /** Air round the drawing, so the frame is never tight on a box. */
  pad: number;
};

export const DEFAULT_METRICS: ExportMetrics = {
  textXs: 12, textSm: 13, leadingXs: 16, leadingSm: 20, space1: 4, space2: 8, space3: 12, edge: 2, radiusSm: 3, radiusMd: 5, pad: 24,
};

/** About how wide the sans face is for its size, a character on average: wide enough that a guess never runs past its box. */
export const SANS = 0.56;
/** The mono face is as wide as 0.6 of its size, every character: a code node's body. */
const MONO = 0.6;
const ICON_PX = 12;

/** Where every part of the scene was drawn. A node or arrow with no entry is left out. */
export type ExportGeometry = {
  rects: ReadonlyMap<string, Rect>;
  paths: ReadonlyMap<string, string>;
  places: ReadonlyMap<string, LabelPlace>;
};

/**
 * One screen as markup: how it stands at rest and how it stands at each step an element of it carries, from
 * `ScreenView`, so the file draws it as the canvas does. `top` is where the sheet starts under the node's title.
 */
export type ScreenMarkup = { top: number; rest: string; steps: Readonly<Record<number, string>> };

/** The screens of a scene by node id, and the stylesheet they are drawn with. A node with no entry is drawn as its frame and name. */
export type ExportScreens = { css: string; nodes: Readonly<Record<string, ScreenMarkup>> };

/** What the owner and a diff did to the scene, as the canvas draws it. */
export type ExportState = {
  change?: ReadonlyMap<string, SceneChange>;
  mine?: ReadonlySet<string>;
  struck?: ReadonlySet<string>;
};

export type SketchSvgInput = {
  scene: Scene;
  geometry: ExportGeometry;
  metrics?: ExportMetrics;
  /** Token name to resolved value; written into the file so it needs no stylesheet of Bridge's. */
  tokens: Readonly<Record<string, string>>;
  /** Inner markup of each glyph a node carries, 24 units square, drawn as strokes. */
  icons?: Readonly<Partial<Record<string, string>>>;
  /** What each `screen` node draws. Only the markup given is written, so a still picture is handed the rest alone. */
  screens?: ExportScreens;
  state?: ExportState;
  /** Each body as the canvas broke it into lines, by node id: the browser's own wrapping, which a count of characters only approximates. */
  bodyLines?: Readonly<Record<string, readonly string[]>>;
  /** The name a reader hears for the drawing. */
  title: string;
  /** A file of its own carries the SVG namespace; markup inside a page does not need it. */
  standalone: boolean;
  /** Extra rules, appended: the page's player lives there. */
  extraCss?: string;
};

const GROUND = new Set(["group", "lane"]);
const SVG_NS = "http://www.w3.org/2000/svg";
const XHTML_NS = "http://www.w3.org/1999/xhtml";

export const xml = (text: string): string => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
const num = (n: number) => String(Math.round(n * 100) / 100);

const sentence = (name: string) => name.charAt(0).toUpperCase() + name.slice(1).replace(/-/g, " ");

/** Where a word may break across lines: after a hyphen between two letters or digits, as a browser breaks it. */
const pieces = (word: string): string[] => word.split(/(?<=[A-Za-z0-9]-)(?=[A-Za-z0-9])/);

/**
 * Lines of a body at `cols` characters: its own breaks kept, words kept whole except at a hyphen, and a piece
 * longer than a line cut. Where the canvas has drawn the body, its own lines are used instead (`bodyLines`).
 */
export function wrapBody(text: string, cols: number): string[] {
  const width = Math.max(1, cols);
  const out: string[] = [];
  for (const paragraph of text.split(/\r?\n/)) {
    let line = "";
    for (const word of paragraph.split(/ +/)) {
      if (word === "" && line === "") continue;
      pieces(word).forEach((piece, at) => {
        let rest = piece;
        while (rest.length > width) {
          if (line !== "") {
            out.push(line);
            line = "";
          }
          out.push(rest.slice(0, width));
          rest = rest.slice(width);
        }
        const joiner = at === 0 && line !== "" ? " " : "";
        if (line === "") line = rest;
        else if (line.length + joiner.length + rest.length <= width) line += joiner + rest;
        else {
          out.push(line);
          line = rest;
        }
      });
    }
    out.push(line);
  }
  return out;
}

const headHeight = (m: ExportMetrics) => m.leadingXs + m.space2 * 2;

/** What a box takes to hold its words at width `w`: the header band, the body and the border, and never less than `h` asked. */
export function nodeHeight(node: SceneNode, w: number, m: ExportMetrics): number {
  const floor = node.h ?? 0;
  if (node.kind === "label") return Math.max(floor, m.leadingSm);
  const hasHead = node.title !== undefined || node.icon !== undefined;
  const cols = Math.floor((w - m.edge * 2 - m.space3 * 2) / (m.textXs * (node.kind === "code" ? MONO : SANS)));
  const body = node.body === undefined ? 0 : wrapBody(node.body, cols).length * m.leadingXs + m.space2 * 2;
  return Math.max(floor, m.edge * 2 + (hasHead ? headHeight(m) : 0) + body);
}

type Frame = { x: number; y: number; w: number; h: number };

/** The drawing's frame: every box, every arrow's words and path, and every hand line, with air round it. */
export function frameOf(scene: Scene, geometry: ExportGeometry, m: ExportMetrics = DEFAULT_METRICS): Frame {
  const xs: number[] = [];
  const ys: number[] = [];
  const take = (x: number, y: number) => {
    xs.push(x);
    ys.push(y);
  };
  for (const node of scene.nodes) {
    const r = geometry.rects.get(node.id);
    if (r === undefined) continue;
    take(r.x, r.y);
    take(r.x + r.w, r.y + r.h);
  }
  for (const [id, path] of geometry.paths) {
    if (!scene.edges.some((one) => one.id === id)) continue;
    for (let t = 0; t <= 10; t += 1) {
      const at = pathAt(path, t / 10);
      if (at !== undefined) take(at.x, at.y);
    }
  }
  for (const p of geometry.places.values()) {
    take(p.x - p.w / 2, p.y - p.h / 2);
    take(p.x + p.w / 2, p.y + p.h / 2);
  }
  for (const stroke of scene.strokes ?? []) for (const point of stroke.points) take(point.x, point.y);
  if (xs.length === 0) return { x: 0, y: 0, w: m.pad * 2, h: m.pad * 2 };
  const x = Math.floor(Math.min(...xs) - m.pad);
  const y = Math.floor(Math.min(...ys) - m.pad);
  return { x, y, w: Math.ceil(Math.max(...xs) + m.pad) - x, h: Math.ceil(Math.max(...ys) + m.pad) - y };
}

/**
 * The player's order: what each step lights and which arrows carry its dot. Read from the scene as the canvas reads it.
 * `inner` lists the screens that hold the step in one of their elements: they ring that element, and do not stand back.
 */
export type StepPlan = { at: number; nodes: string[]; edges: string[]; inner?: string[] }[];

export function stepPlan(scene: Scene): StepPlan {
  const inside = (one: SceneNode) => (one.screen === undefined ? [] : screenSteps(one.screen));
  const numbers = [...new Set([...scene.nodes.flatMap((one) => [one.step, ...inside(one)]), ...scene.edges.map((one) => one.step)].filter((one): one is number => one !== undefined))].sort((a, b) => a - b);
  return numbers.map((at) => {
    const nodes = [...new Set([...scene.nodes.filter((one) => one.step === at).map((one) => one.id), ...scene.edges.filter((one) => one.step === at).map((one) => one.to)])];
    const inner = scene.nodes.filter((one) => !nodes.includes(one.id) && inside(one).includes(at)).map((one) => one.id);
    return { at, nodes, edges: scene.edges.filter((one) => one.step === at).map((one) => one.id), ...(inner.length === 0 ? {} : { inner }) };
  });
}

/** The drawing's own rules. Colours are variables so the one block of token values is the whole theme. */
export const EXPORT_CSS = `
.sk { font-family: var(--font-sans); }
.sk-ground { fill: var(--bg-sunken); }
.sk-node { --hue: var(--status-running); }
.sk-node[data-kind="code"] { --hue: var(--status-not-started); }
.sk-node[data-kind="wire"], .sk-node[data-kind="screen"] { --hue: var(--border-strong); }
.sk-node[data-change="added"], .sk-edge[data-change="added"] { --hue: var(--status-completed-success); }
.sk-node[data-change="removed"], .sk-edge[data-change="removed"] { --hue: var(--status-completed-failed); }
.sk-node[data-change="changed"], .sk-edge[data-change="changed"] { --hue: var(--status-awaiting-review); }
.sk-node[data-mine], .sk-edge[data-mine] { --hue: var(--status-rejected); }
.sk-node[data-dim], .sk-edge[data-dim] { opacity: 0.35; }
.sk-node[data-struck], .sk-edge[data-struck] { opacity: 0.5; }
.sk-node[data-struck] text, .sk-node[data-change="removed"] .sk-body, .sk-node[data-change="removed"] .sk-title { text-decoration: line-through; }
.sk-fill { fill: color-mix(in srgb, var(--hue) var(--row-tint-recent), var(--bg-raised)); }
.sk-border { fill: none; stroke: var(--hue); stroke-width: var(--edge-active); }
.sk-frame { fill: color-mix(in srgb, var(--hue) var(--row-tint-recent), var(--bg-raised)); stroke: var(--hue); stroke-width: var(--edge-active); }
.sk-node[data-kind="group"] .sk-frame, .sk-node[data-kind="lane"] .sk-frame { fill: none; stroke-dasharray: var(--space-2) var(--space-2); }
.sk-node[data-kind="wire"] .sk-frame, .sk-node[data-kind="screen"] .sk-frame { fill: var(--bg-sunken); stroke-dasharray: var(--space-2) var(--space-2); }
.sk-ring { display: none; fill: none; stroke: var(--edge-focus); stroke-width: var(--edge-active); }
.sk-node[data-current] .sk-ring { display: inline; }
.sk-head { fill: var(--hue); }
.sk-title { fill: var(--fg-inverse); font-size: var(--text-xs); font-weight: var(--weight-heading); dominant-baseline: central; }
.sk-lang { fill: var(--fg-inverse); opacity: 0.8; font-size: var(--text-xs); text-anchor: end; dominant-baseline: central; }
.sk-icon { color: var(--fg-inverse); }
.sk-body { fill: var(--fg-default); font-size: var(--text-xs); white-space: pre; dominant-baseline: central; }
.sk-node[data-kind="code"] .sk-body { font-family: var(--font-mono); }
.sk-label { fill: var(--fg-muted); font-size: var(--text-sm); font-weight: var(--weight-heading); dominant-baseline: central; }
.sk-wire-title { fill: var(--fg-muted); font-size: var(--text-sm); font-weight: var(--weight-heading); dominant-baseline: central; }
.sk-caption { fill: var(--fg-muted); font-family: var(--font-sans); font-size: var(--text-sm); font-weight: var(--weight-heading); dominant-baseline: central; }
.sk-screen[data-off] { display: none; }
:where(.sk-screen) *, :where(.sk-screen) ::before, :where(.sk-screen) ::after { box-sizing: border-box; margin: 0; padding: 0; border: 0 solid; }
:where(.sk-screen) svg { display: block; vertical-align: middle; }
:where(.sk-screen) table { border-collapse: collapse; text-indent: 0; border-color: inherit; }
.sk-wire-line { fill: var(--border-strong); }
.sk-wire-input { fill: none; stroke: var(--border-strong); stroke-width: var(--edge-active); }
.sk-edge { --hue: var(--fg-muted); }
.sk-edge__line { fill: none; stroke: var(--hue); stroke-width: var(--edge-active); }
.sk-edge__head { fill: var(--hue); }
.sk-edge[data-flow] .sk-edge__line { stroke-dasharray: var(--space-2) var(--space-2); }
.sk-edge[data-struck] .sk-edge__line { stroke-dasharray: var(--space-1) var(--space-1); }
.sk-edge[data-active] .sk-edge__line { stroke: var(--edge-focus); }
.sk-edge[data-active] .sk-edge__head { fill: var(--edge-focus); }
.sk-edge__backing { fill: var(--bg-sunken); rx: var(--radius-sm); }
.sk-edge__label { fill: var(--fg-muted); font-size: var(--text-xs); text-anchor: middle; dominant-baseline: central; }
.sk-ink { fill: none; stroke: var(--fg-muted); stroke-width: var(--edge-active); stroke-linecap: round; stroke-linejoin: round; }
`;

/** Token values as the declarations of one rule. */
export function tokenDeclarations(tokens: Readonly<Record<string, string>>): string {
  return Object.entries(tokens)
    .filter(([, value]) => value.trim() !== "")
    .map(([name, value]) => `${name}: ${value.trim().replace(/[<>]/g, "")};`)
    .join(" ");
}

function nodeSvg(node: SceneNode, scene: Scene, input: SketchSvgInput, m: ExportMetrics): string {
  const r = input.geometry.rects.get(node.id);
  if (r === undefined) return "";
  const state = input.state ?? {};
  const change = state.change?.get(node.id);
  const mine = state.mine?.has(node.id) ?? false;
  const struck = state.struck?.has(node.id) ?? false;
  const name = mine ? (node.body?.trim() ? `Your box: ${node.body}` : "Your box") : partLabel(scene, node.id);
  const said = `${mine ? "" : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${name}`;
  const { w, h } = r;
  const e = m.edge;
  const ground = GROUND.has(node.kind);
  const attrs = [
    `class="sk-node"`, `data-node="${xml(node.id)}"`, `data-kind="${node.kind}"`,
    ...(ground ? ['data-ground=""'] : []),
    ...(change === undefined ? [] : [`data-change="${change}"`]),
    ...(mine ? ['data-mine=""'] : []), ...(struck ? ['data-struck=""'] : []),
    `role="group"`, `aria-label="${xml(said)}"`, `transform="translate(${num(r.x)} ${num(r.y)})"`,
  ].join(" ");
  const parts: string[] = [];
  const clip = `sk-clip-${xml(node.id).replace(/[^A-Za-z0-9_-]/g, "_")}`;
  const mid = (top: number, line: number) => num(top + line / 2);
  const glyph = node.icon === undefined ? undefined : input.icons?.[node.icon];
  const sheet = input.screens?.nodes[node.id];

  const head = (): number => {
    if (node.title === undefined && node.icon === undefined) return 0;
    const bandH = headHeight(m);
    let x = e + m.space3;
    const items: string[] = [];
    if (glyph !== undefined && node.icon !== undefined) {
      items.push(
        `<g class="sk-icon" role="img" aria-label="${xml(sentence(node.icon))}"><svg x="${num(x)}" y="${num(e + (bandH - ICON_PX) / 2)}" width="${ICON_PX}" height="${ICON_PX}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${glyph}</svg></g>`,
      );
      x += ICON_PX + m.space2;
    }
    if (node.title !== undefined) items.push(`<text class="sk-title" x="${num(x)}" y="${mid(e, bandH)}">${xml(node.title)}</text>`);
    if (node.lang !== undefined) items.push(`<text class="sk-lang" x="${num(w - e - m.space3)}" y="${mid(e, bandH)}">${xml(node.lang)}</text>`);
    if (ground) {
      const chip = x + (node.title === undefined ? 0 : node.title.length * m.textXs * SANS) + m.space3;
      parts.push(`<rect class="sk-head" x="${num(e)}" y="${num(e)}" width="${num(Math.min(chip, w - e * 2))}" height="${num(bandH)}" rx="${num(m.radiusMd)}"/>`);
    } else {
      parts.push(`<rect class="sk-head" x="${num(e)}" y="${num(e)}" width="${num(w - e * 2)}" height="${num(bandH)}"/>`);
    }
    parts.push(...items);
    return bandH;
  };
  const body = (top: number, text: string) => {
    const cols = Math.floor((w - e * 2 - m.space3 * 2) / (m.textXs * (node.kind === "code" ? MONO : SANS)));
    const lines = input.bodyLines?.[node.id] ?? wrapBody(text, cols);
    const tspans = lines.map((line, at) => `<tspan x="${num(e + m.space3)}" y="${mid(top + m.space2 + at * m.leadingXs, m.leadingXs)}">${xml(line === "" ? " " : line)}</tspan>`).join("");
    parts.push(`<text class="sk-body">${tspans}</text>`);
  };

  if (node.kind === "label") {
    parts.push(`<text class="sk-label" x="0" y="${num(h / 2)}">${xml(node.title ?? node.body ?? "")}</text>`);
  } else if (mine) {
    parts.push(`<rect class="sk-frame" x="${num(e / 2)}" y="${num(e / 2)}" width="${num(w - e)}" height="${num(h - e)}" rx="${num(m.radiusMd)}"/>`);
    body(e, node.body ?? "");
  } else if (node.kind === "screen" && sheet !== undefined) {
    // The sheet as the canvas draws it, under its name: the screen's own markup and stylesheet in a foreignObject.
    if (node.title !== undefined) parts.push(`<text class="sk-caption" x="0" y="${num((sheet.top - m.space2) / 2)}">${xml(node.title)}</text>`);
    const html = (markup: string) => {
      const bare = markup.replace(/ xmlns="[^"]*"/g, "");
      return input.standalone ? bare.replace(/<svg /g, `<svg xmlns="${SVG_NS}" `) : bare;
    };
    const variants: [string, string][] = [["rest", sheet.rest], ...Object.entries(sheet.steps).map(([at, markup]): [string, string] => [at, markup])];
    for (const [at, markup] of variants) {
      const body = `<div${input.standalone ? ` xmlns="${XHTML_NS}"` : ""}>${html(markup)}</div>`;
      parts.push(`<g class="sk-screen" data-at="${at}"${at === "rest" ? "" : ' data-off=""'}><foreignObject x="0" y="${num(sheet.top)}" width="${num(w)}" height="${num(Math.max(0, h - sheet.top))}" overflow="visible">${body}</foreignObject></g>`);
    }
  } else if (node.kind === "screen") {
    // The screen's outer frame and its name, for a picture handed no markup to draw it with.
    parts.push(`<rect class="sk-frame" x="${num(e / 2)}" y="${num(e / 2)}" width="${num(w - e)}" height="${num(h - e)}" rx="${num(m.radiusMd)}"/>`);
    if (node.title !== undefined) parts.push(`<text class="sk-wire-title" x="${num(e + m.space3)}" y="${mid(e, headHeight(m))}">${xml(node.title)}</text>`);
  } else if (node.kind === "wire") {
    parts.push(`<rect class="sk-frame" x="${num(e / 2)}" y="${num(e / 2)}" width="${num(w - e)}" height="${num(h - e)}" rx="${num(m.radiusMd)}"/>`);
    const part = node.wire ?? "card";
    const title = node.title ?? "";
    const x = e + m.space3;
    const inner = w - e * 2 - m.space3 * 2;
    if (part === "button") {
      parts.push(`<rect class="sk-wire-line" x="${num(e)}" y="${num(e)}" width="${num(w - e * 2)}" height="${num(h - e * 2)}" rx="${num(m.radiusSm)}"/>`);
      parts.push(`<text class="sk-body" x="${num(w / 2)}" y="${num(h / 2)}" text-anchor="middle">${xml(title)}</text>`);
    } else if (part === "input") {
      parts.push(`<rect class="sk-wire-input" x="${num(e + m.space2)}" y="${num(e + m.space2)}" width="${num(w - (e + m.space2) * 2)}" height="${num(h - (e + m.space2) * 2)}"/>`);
      parts.push(`<text class="sk-wire-title" x="${num(x + m.space2)}" y="${num(h / 2)}">${xml(title)}</text>`);
    } else if (part === "nav") {
      parts.push(`<text class="sk-wire-title" x="${num(x)}" y="${num(h / 2)}">${xml(title)}</text>`);
    } else {
      let y = e + m.space3;
      if (title !== "") {
        parts.push(`<text class="sk-wire-title" x="${num(x)}" y="${mid(y, m.leadingSm)}">${xml(title)}</text>`);
        y += m.leadingSm + m.space2;
      }
      const lines = part === "text" ? [1, 1] : [1, 1, 0.6];
      for (const share of lines) {
        parts.push(`<rect class="sk-wire-line" x="${num(x)}" y="${num(y)}" width="${num(inner * share)}" height="${num(m.space2)}" rx="${num(m.radiusSm)}"/>`);
        y += m.space2 * 2;
      }
    }
  } else if (ground) {
    if (node.kind === "lane") {
      parts.push(`<path class="sk-frame" d="M0 ${num(e / 2)}H${num(w)}M0 ${num(h - e / 2)}H${num(w)}"/>`);
    } else {
      parts.push(`<rect class="sk-frame" x="${num(e / 2)}" y="${num(e / 2)}" width="${num(w - e)}" height="${num(h - e)}" rx="${num(m.radiusMd)}"/>`);
    }
    head();
  } else {
    parts.push(`<clipPath id="${clip}"><rect x="0" y="0" width="${num(w)}" height="${num(h)}" rx="${num(m.radiusMd)}"/></clipPath>`);
    parts.push(`<g clip-path="url(#${clip})"><rect class="sk-fill" x="0" y="0" width="${num(w)}" height="${num(h)}"/>`);
    const bandH = head();
    if (node.body !== undefined) body(e + bandH, node.body);
    parts.push(`</g>`);
    parts.push(`<rect class="sk-border" x="${num(e / 2)}" y="${num(e / 2)}" width="${num(w - e)}" height="${num(h - e)}" rx="${num(m.radiusMd)}"/>`);
  }
  if (sheet !== undefined && node.kind === "screen") parts.push(`<rect class="sk-ring" x="${num(-e)}" y="${num(sheet.top - e)}" width="${num(w + e * 2)}" height="${num(h - sheet.top + e * 2)}" rx="${num(m.radiusSm + e)}"/>`);
  else if (node.kind !== "label") parts.push(`<rect class="sk-ring" x="${num(-e)}" y="${num(-e)}" width="${num(w + e * 2)}" height="${num(h + e * 2)}" rx="${num(m.radiusMd + e)}"/>`);
  return `<g ${attrs}>${parts.join("")}</g>`;
}

function edgeSvg(at: number, input: SketchSvgInput): string {
  const edge = input.scene.edges[at]!;
  const path = input.geometry.paths.get(edge.id);
  if (path === undefined) return "";
  const state = input.state ?? {};
  const change = state.change?.get(edge.id);
  const mine = state.mine?.has(edge.id) ?? false;
  const struck = state.struck?.has(edge.id) ?? false;
  const said = `${mine ? "Yours, " : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${edge.label ?? partLabel(input.scene, edge.id)}`;
  const place = input.geometry.places.get(edge.id);
  const head = `sk-head-${String(at)}`;
  const attrs = [
    `class="sk-edge"`, `data-edge="${xml(edge.id)}"`,
    ...(edge.flow !== false ? ['data-flow=""'] : []),
    ...(change === undefined ? [] : [`data-change="${change}"`]),
    ...(mine ? ['data-mine=""'] : []), ...(struck ? ['data-struck=""'] : []),
    `role="group"`, `aria-label="${xml(said)}"`,
  ].join(" ");
  const words =
    place === undefined || edge.label === undefined
      ? ""
      : `<g class="sk-edge__words" role="img" aria-label="${xml(edge.label)}"><rect class="sk-edge__backing" x="${num(place.x - place.w / 2)}" y="${num(place.y - place.h / 2)}" width="${num(place.w)}" height="${num(place.h)}"/>${place.lines
          .map((line, index) => `<text class="sk-edge__label" x="${num(place.x)}" y="${num(place.y - place.h / 2 + place.pad + (index + 0.5) * place.lineH)}">${xml(line)}</text>`)
          .join("")}</g>`;
  return `<g ${attrs}><defs><marker id="${head}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse"><path class="sk-edge__head" d="M0 0L10 5L0 10z"/></marker></defs><path class="sk-edge__line" d="${xml(path)}" marker-end="url(#${head})"/>${words}</g>`;
}

/** The scene as an SVG document or element, framed to what it holds. */
export function sketchSvg(input: SketchSvgInput): string {
  const m = input.metrics ?? DEFAULT_METRICS;
  const frame = frameOf(input.scene, input.geometry, m);
  const grounds = input.scene.nodes.filter((one) => GROUND.has(one.kind));
  const solids = input.scene.nodes.filter((one) => !GROUND.has(one.kind));
  const ink = (input.scene.strokes ?? []).map((one) => `<path class="sk-ink" d="${xml(pathOf(one.points))}" role="img" aria-label="Drawn by hand"/>`).join("");
  const css = `.sk { ${tokenDeclarations(input.tokens)} }${EXPORT_CSS}${input.screens?.css ?? ""}${input.extraCss ?? ""}`;
  return [
    `<svg ${input.standalone ? 'xmlns="http://www.w3.org/2000/svg" ' : ""}class="sk armada-sketch-export" viewBox="${frame.x} ${frame.y} ${frame.w} ${frame.h}" width="${frame.w}" height="${frame.h}" role="group" aria-label="${xml(input.title)}">`,
    `<title>${xml(input.title)}</title>`,
    `<style>${css}</style>`,
    `<rect class="sk-ground" x="${frame.x}" y="${frame.y}" width="${frame.w}" height="${frame.h}"/>`,
    ...grounds.map((one) => nodeSvg(one, input.scene, input, m)),
    // Over the boxes, as the canvas draws them, so no box hides where an arrow goes.
    ...solids.map((one) => nodeSvg(one, input.scene, input, m)),
    ...input.scene.edges.map((_, at) => edgeSvg(at, input)),
    ink,
    `</svg>`,
  ].join("");
}
