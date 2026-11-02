// The bridge take's console drawn on a canvas (mock): a cone section of a cockpit dashboard seen from the pilot seat, its facets joined edge to
// edge, the wings sweeping toward the viewer. Each facet is drawn flat, with its instrument, onto a texture, and the texture is put on the
// facet's projected quad, so the instrument skews with the surface. The lip, the back rim, the end walls and the hard shadows are flat polygons
// in the theme's tones. The page lays a transparent button over each facet (`ConsoleBays.tsx`), which keeps the presses, labels and tooltips.
// No gradients and no glow: the depth is the projection, the layering and the hard edges. Does not touch the map's stage.

import "pixi.js/unsafe-eval";
import { Application, Container, Graphics, PerspectiveMesh, RenderTexture, Text, Ticker } from "pixi.js";

import { hex, readPalette, type Rgba } from "./palette";

export type Kind = "matrix" | "beacon" | "radar" | "trace" | "scope" | "needles" | "usage" | "dispatch";
/** A dot of the Activity graph, 0 to 1 across the scope's box. */
export type Dot = { x: number; y: number; r: number; colour: string };
export type BayState = { kind: Kind | null; pressed: boolean; calling: boolean; lit: boolean; code: string; label: string; dots?: readonly Dot[] };
export type Rect = { x: number; y: number; w: number; h: number };
type P = { x: number; y: number };
type Quad = readonly [P, P, P, P];

/** Where each facet's button goes, and how tall the canvas has to be. */
export type Layout = { width: number; height: number; inner: Rect[]; rim: Rect | undefined };

/** How wide each facet is, in units of one ordinary facet, and which facet carries Dispatch as a key on its lip (none where Dispatch is a facet). */
export type Plan = { widths: readonly number[]; depths: readonly number[]; keyed: number | undefined };
const FW = 190;
const FH = 108;

// ---- The projection: a pinhole over a cone ----------------------------------------------------------------

const CONE = { R: 1250, U: 150, slope: (30 * Math.PI) / 180, step: 0.16, behind: 1100, focal: 1500, lipY: -190, raise: 60, thick: 30, rim: 17, key: 2.4 };

type World = { quads: Quad[]; lips: Quad[]; rims: Quad[]; ends: Quad[]; floor: Quad[]; shade: Quad[]; keyQuad: Quad | undefined; keyAspect: number; height: number };

function worldOf(width: number, plan: Plan): World {
  const { R, U: baseU, slope, step, behind, focal, lipY, raise, thick, rim, key } = CONE;
  const total = plan.widths.reduce((a, b) => a + b, 0);
  const edges = [-(total * step) / 2];
  for (const w of plan.widths) edges.push(edges.at(-1)! + w * step);
  const reach = (total * step) / 2;
  const at = (a: number, u: number, down = 0, up = 0): P => {
    const radius = R + u * Math.cos(slope);
    const x = radius * Math.sin(a);
    const y = lipY + raise * (a / reach) ** 2 + u * Math.sin(slope) - down + up;
    const z = behind + radius * Math.cos(a);
    return { x: (focal * x) / z, y: (-focal * y) / z };
  };
  const quads: Quad[] = [];
  const lips: Quad[] = [];
  const rims: Quad[] = [];
  const floor: Quad[] = [];
  const shade: Quad[] = [];
  let keyQuad: Quad | undefined;
  let keyAspect = 1;
  plan.widths.forEach((_, i) => {
    const [a, b] = [edges[i]!, edges[i + 1]!];
    const t = i === plan.keyed ? thick * key : thick;
    const U = baseU * plan.depths[i]!;
    quads.push([at(a, U), at(b, U), at(b, 0), at(a, 0)]);
    const lip: Quad = [at(a, 0), at(b, 0), at(b, 0, t), at(a, 0, t)];
    lips.push(lip);
    if (i === plan.keyed) {
      keyQuad = lip;
      keyAspect = (R * (b - a)) / t;
    }
    rims.push([at(a, U, 0, rim), at(b, U, 0, rim), at(b, U), at(a, U)]);
    floor.push([at(a, 0, t), at(b, 0, t), at(b, 0, t + thick * 0.9), at(a, 0, t + thick * 0.9)]);
    const near = U * 0.8;
    shade.push([at(a, U), at(b, U), at(b, near), at(a, near)]);
  });
  const ends: Quad[] = [edges[0]!, edges.at(-1)!].map((a, k) => {
    const U = baseU * plan.depths[k === 0 ? 0 : plan.depths.length - 1]!;
    return [at(a, 0), at(a, U), at(a, U, thick), at(a, 0, thick)] as const;
  });
  const every = [...quads, ...lips, ...rims, ...floor, ...ends].flat();
  const [minX, maxX] = [Math.min(...every.map((p) => p.x)), Math.max(...every.map((p) => p.x))];
  const [minY, maxY] = [Math.min(...every.map((p) => p.y)), Math.max(...every.map((p) => p.y))];
  const margin = 4;
  const scale = (width - margin * 2) / (maxX - minX);
  const move = (p: P): P => ({ x: (p.x - minX) * scale + margin, y: (p.y - minY) * scale + margin });
  const fit = (q: Quad): Quad => [move(q[0]), move(q[1]), move(q[2]), move(q[3])];
  return { quads: quads.map(fit), lips: lips.map(fit), rims: rims.map(fit), ends: ends.map(fit), floor: floor.map(fit), shade: shade.map(fit), keyQuad: keyQuad === undefined ? undefined : fit(keyQuad), keyAspect, height: Math.ceil((maxY - minY) * scale + margin * 2) };
}

const boxOf = (q: Quad): Rect => {
  const x = Math.max(q[0].x, q[3].x);
  const y = Math.max(q[0].y, q[1].y);
  return { x, y, w: Math.min(q[1].x, q[2].x) - x, h: Math.min(q[2].y, q[3].y) - y };
};

/** The room each facet's button has: the box inside its quad, so neighbours never overlap. The key on a lip comes last. */
export function layoutOf(width: number, plan: Plan): Layout {
  const world = worldOf(width, plan);
  const inner = world.quads.map(boxOf);
  if (world.keyQuad !== undefined) inner.push(boxOf(world.keyQuad));
  const first = world.rims[0]!;
  const last = world.rims.at(-1)!;
  return { width, height: world.height, inner, rim: { x: first[0].x, y: Math.min(first[0].y, last[1].y), w: last[1].x - first[0].x, h: first[3].y - first[0].y } };
}

// ---- Colours ------------------------------------------------------------------------------------------------

type Tones = { base: number; sunken: number; raised: number; strong: number; edge: number; quiet: number; fg: number; muted: number; subtle: number; accent: number; accentMuted: number; ask: number; bad: number; good: number; run: number; mono: string };

let scratch: CanvasRenderingContext2D | undefined;
const inks = new Map<string, number>();
/** Any CSS colour, `color-mix()` included, as 0xRRGGBB. */
function cssHex(css: string): number {
  const kept = inks.get(css);
  if (kept !== undefined) return kept;
  scratch ??= document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
  scratch.canvas.width = 1;
  scratch.canvas.height = 1;
  scratch.clearRect(0, 0, 1, 1);
  scratch.fillStyle = css;
  scratch.fillRect(0, 0, 1, 1);
  const [r, g, b] = scratch.getImageData(0, 0, 1, 1).data;
  const ink = (r! << 16) | (g! << 8) | b!;
  inks.set(css, ink);
  return ink;
}
function readTones(host: HTMLElement): Tones {
  const base = readPalette(host);
  const probe = document.createElement("span");
  host.append(probe);
  const colour = (name: string): number => {
    probe.style.color = "";
    probe.style.color = `var(${name})`;
    scratch ??= document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
    scratch.canvas.width = 1;
    scratch.canvas.height = 1;
    scratch.clearRect(0, 0, 1, 1);
    scratch.fillStyle = getComputedStyle(probe).color;
    scratch.fillRect(0, 0, 1, 1);
    const [r, g, b] = scratch.getImageData(0, 0, 1, 1).data;
    return (r! << 16) | (g! << 8) | b!;
  };
  const tones: Tones = {
    base: colour("--bg-base"),
    sunken: colour("--bg-sunken"),
    raised: colour("--bg-raised"),
    strong: colour("--border-strong"),
    edge: colour("--border-default"),
    quiet: colour("--border-subtle"),
    fg: colour("--fg-default"),
    muted: colour("--fg-muted"),
    subtle: colour("--fg-subtle"),
    accent: colour("--accent"),
    accentMuted: colour("--accent-muted"),
    ask: hex(base.ask as Rgba),
    bad: hex(base.bad as Rgba),
    good: hex(base.good as Rgba),
    run: hex(base.run as Rgba),
    mono: base.mono,
  };
  probe.remove();
  return tones;
}

// ---- A facet's flat picture ----------------------------------------------------------------------------------------

type Facet = { w: number; h: number; root: Container; g: Graphics; code: Text; label: Text; extra: Text; tex: RenderTexture; mesh: PerspectiveMesh };

const TAU = Math.PI * 2;

function draw(f: Facet, s: BayState, tone: Tones, t: number, lipKey: boolean): void {
  const g = f.g;
  const { w: W, h: H } = f;
  g.clear();
  const edge = s.pressed ? tone.accent : s.lit ? tone.fg : tone.strong;
  g.rect(0, 0, W, H).fill(tone.sunken);
  g.rect(2, 2, W - 4, H - 4).fill(s.pressed ? tone.accentMuted : tone.raised).stroke({ width: 4, color: edge });
  if (s.kind === null) {
    g.rect(2, 2, W - 4, H - 4).fill(tone.sunken).stroke({ width: 4, color: tone.edge });
    g.rect(10, 10, W - 20, H - 20).stroke({ width: 1.5, color: tone.quiet });
    f.code.text = "";
    f.label.text = "";
    f.extra.text = "";
    return;
  }
  const [x, y, w, h] = [10, 16, W - 20, H - 40];
  const [cx, cy] = [x + w / 2, y + h / 2];
  if (s.kind !== "dispatch") g.rect(x, y, w, h).fill(tone.base).stroke({ width: 1.5, color: tone.edge });
  switch (s.kind) {
    case "matrix": {
      const cols = Math.floor((w - 16) / 8.6);
      const lit = Math.round(cols * (0.55 + 0.25 * Math.sin(Math.floor(t * 2) * 1.7)));
      const rows = Math.max(3, Math.floor((h - 26) / 11));
      for (let c = 0; c < cols; c += 1) {
        for (let r = 0; r < rows; r += 1) g.rect(x + 8 + c * 8.6, y + 16 + r * 11, 6.4, 8).fill(c < lit ? tone.run : tone.strong);
      }
      for (let k = 0; k < 3; k += 1) g.rect(x + w - 10 - k * 9, y + 5, 5, 5).fill(k === 0 || s.pressed ? tone.run : tone.strong);
      break;
    }
    case "beacon": {
      const pulse = s.calling ? 0.65 + 0.35 * Math.abs(Math.sin(t * 3.4)) : 0;
      const k = Math.min(w, h) / 82;
      g.circle(cx, cy, 36 * k).fill(tone.sunken).stroke({ width: 4, color: tone.strong });
      g.circle(cx, cy, 24 * k).fill(s.calling ? tone.ask : tone.sunken).stroke({ width: 1.5, color: tone.ask });
      if (s.calling) g.circle(cx, cy, 24 * k).fill({ color: tone.ask, alpha: pulse });
      else g.circle(cx, cy, 20 * k).fill({ color: tone.ask, alpha: 0.22 });
      for (let n = 0; n < 12; n += 1) {
        const a = (n / 12) * TAU + (s.calling ? t * 1.6 : 0);
        g.moveTo(cx + Math.cos(a) * 31 * k, cy + Math.sin(a) * 31 * k).arc(cx, cy, 31 * k, a, a + 0.28).stroke({ width: 5 * k, color: tone.ask, alpha: s.calling ? 1 : 0.25 });
      }
      break;
    }
    case "radar": {
      const r = Math.min(w, h) / 2 - 3;
      if (w > h * 1.8) {
        for (const side of [-1, 1]) {
          for (let k = 0; k < 7; k += 1) g.rect(side < 0 ? x + 10 + k * 9 : x + w - 10 - k * 9 - 2, cy - 5 - (k % 3) * 3, 2, 10 + (k % 3) * 6).fill(k < 3 ? tone.run : tone.strong);
          for (let k = 0; k < 3; k += 1) g.rect(side < 0 ? x + 10 : x + w - 10 - 70, y + h - 12 - k * 7, 70, 2.4).fill(tone.edge);
        }
      }
      for (const k of [1, 0.75, 0.5, 0.25]) g.circle(cx, cy, r * k).stroke({ width: 1.4, color: k === 1 ? tone.strong : tone.edge });
      g.moveTo(cx - r, cy).lineTo(cx + r, cy).moveTo(cx, cy - r).lineTo(cx, cy + r).stroke({ width: 1, color: tone.edge });
      for (let k = 0; k < 72; k += 1) {
        const a = (k / 72) * TAU;
        const long = k % 6 === 0;
        g.moveTo(cx + Math.cos(a) * (r - (long ? 7 : 4)), cy + Math.sin(a) * (r - (long ? 7 : 4))).lineTo(cx + Math.cos(a) * r, cy + Math.sin(a) * r).stroke({ width: long ? 1.5 : 1, color: tone.subtle });
      }
      const sweep = t * 1.3;
      g.moveTo(cx, cy).arc(cx, cy, r - 2, sweep - 0.6, sweep).closePath().fill({ color: tone.run, alpha: 0.32 });
      g.moveTo(cx, cy).lineTo(cx + Math.cos(sweep) * (r - 2), cy + Math.sin(sweep) * (r - 2)).stroke({ width: 2, color: tone.run });
      for (const [bx, by] of [[0.3, -0.35], [-0.4, 0.3], [0.45, 0.2]] as const) g.circle(cx + bx * r, cy + by * r, 2.4).fill(tone.fg);
      g.circle(cx, cy, 3).fill(s.pressed ? tone.accent : tone.muted);
      break;
    }
    case "trace": {
      const shift = (t * 30) % 40;
      g.moveTo(x + 6, cy - 8);
      for (let px = x + 6; px <= x + w - 6; px += 2) {
        const p = (px - x + shift) % 40;
        const v = p < 8 ? 0 : p < 11 ? -(p - 8) * 5 : p < 15 ? -15 + (p - 11) * 8 : p < 19 ? 17 - (p - 15) * 4.2 : 0;
        g.lineTo(px, cy - 8 + v * 0.8);
      }
      g.stroke({ width: 1.8, color: tone.run });
      for (let px = x + 6; px < x + w - 6; px += 8) g.rect(px, y + h - 22, 1.2, ((px - x + shift) % 40) < 8 ? 12 : 6).fill(tone.strong);
      g.rect(cx, y + 6, 1.5, h - 12).fill(tone.ask);
      break;
    }
    case "scope": {
      g.rect(x + 6, cy, w - 12, 1.2).fill(tone.strong);
      for (let px = x + 8; px < x + w - 8; px += 8) g.rect(px, y + h - 9, 1.2, px % 40 < 8 ? 7 : 4).fill(tone.strong);
      g.rect(x + w - 8, y + h - 24, 1.5, 15).fill(tone.ask);
      for (const d of s.dots ?? []) g.circle(x + 6 + d.x * (w - 12), y + 6 + d.y * (h - 12), Math.max(1.6, d.r)).fill(cssHex(d.colour));
      break;
    }
    case "needles": {
      for (const [k, ink, rate] of [[0, tone.ask, 1.3], [1, tone.run, 0.9]] as const) {
        const px = x + w * (0.27 + 0.46 * k);
        const py = y + h * 0.62;
        g.moveTo(px - 26, py).arc(px, py, 26, Math.PI, 0).stroke({ width: 1.5, color: tone.strong });
        for (let q = 0; q <= 6; q += 1) {
          const a = Math.PI + (q / 6) * Math.PI;
          g.moveTo(px + Math.cos(a) * 26, py + Math.sin(a) * 26).lineTo(px + Math.cos(a) * 21, py + Math.sin(a) * 21).stroke({ width: 1, color: tone.strong });
        }
        const a = Math.PI * 1.5 + Math.sin(t * rate + k) * 0.7;
        g.moveTo(px, py).lineTo(px + Math.cos(a) * 23, py + Math.sin(a) * 23).stroke({ width: 2.2, color: ink });
        g.circle(px, py, 2.5).fill(tone.muted);
      }
      for (let k = 0; k < 3; k += 1) g.rect(x + 12, y + h - 18 + k * 5, w - 24, 2.4).fill(tone.edge);
      break;
    }
    case "usage": {
      for (const [row, share, ink] of [[0, 0.42, tone.run], [1, 0.71, tone.ask]] as const) {
        for (let c = 0; c < Math.floor((w - 16) / 7.6); c += 1) g.rect(x + 8 + c * 7.6, y + 24 + row * 22, 5.6, 9).fill(c / Math.floor((w - 16) / 7.6) < share ? ink : tone.strong);
      }
      break;
    }
    case "dispatch": {
      if (lipKey) {
        for (let k = 0; k < W / 9 + 1; k += 1) g.poly([k * 9, 0, k * 9 + 5, 0, k * 9, 10, k * 9 - 5, 10]).fill(tone.ask);
        g.circle(W / 2 - 38, H / 2 + 2, H * 0.28).fill(tone.edge);
        g.circle(W / 2 - 38, H / 2 - 2, H * 0.28).fill(tone.raised).stroke({ width: 3, color: s.pressed ? tone.accent : tone.strong });
        g.rect(W / 2 + 4, H / 2 - 3, 48, 6).fill(Math.floor(t * 1.4) % 2 === 0 ? tone.ask : tone.strong);
        break;
      }
      for (let k = 0; k < W / 9 + 1; k += 1) g.poly([k * 9, 8, k * 9 + 5, 8, k * 9, 20, k * 9 - 5, 20]).fill(tone.ask);
      g.rect(8, 24, W - 16, H - 32).fill(tone.sunken).stroke({ width: 3, color: tone.strong });
      const [dx, dy] = [W / 2, 24 + (H - 32 - 14) / 2 + 6];
      const r = Math.min(W / 2 - 24, (H - 52) / 2);
      g.circle(dx, dy + 6, r).fill(tone.edge);
      g.circle(dx, dy, r).fill(tone.raised).stroke({ width: 5, color: s.pressed ? tone.accent : s.lit ? tone.fg : tone.strong });
      g.circle(dx, dy, r * 0.78).stroke({ width: 1.5, color: tone.strong });
      g.rect(dx - 9, dy - r + 4, 18, 6).fill(Math.floor(t * 1.4) % 2 === 0 ? tone.ask : tone.strong);
      const k = r / 38;
      g.poly([dx - 14 * k, dy + 12 * k, dx + 16 * k, dy - 2 * k, dx - 6 * k, dy + 16 * k]).fill(tone.accent).poly([dx - 14 * k, dy + 12 * k, dx + 16 * k, dy - 2 * k, dx - 4 * k, dy + 2 * k]).fill(tone.raised);
      break;
    }
  }
  f.code.text = lipKey ? "" : s.code;
  f.code.style.fill = tone.subtle;
  f.label.text = s.kind === "usage" ? "" : s.label;
  f.label.style.fill = s.pressed || s.lit ? tone.fg : tone.muted;
  f.extra.text = s.kind === "usage" ? "plan-usage" : "";
  f.extra.style.fill = tone.subtle;
  f.label.x = lipKey ? W / 2 + 4 : (W - f.label.width) / 2;
  f.label.y = lipKey ? H / 2 + 8 : H - 20;
  f.extra.x = x + 8;
  f.extra.y = y + 6;
}

// ---- The stage ---------------------------------------------------------------------------------------------

export class ConsoleStage {
  private app: Application | undefined;
  private gone = false;
  private hidden = false;
  private width = 0;
  private world: World | undefined;
  private facets: Facet[] = [];
  private shell = new Graphics();
  private shade = new Graphics();
  private stencil: Text | undefined;
  private tone: Tones | undefined;
  private bays: BayState[] = [];
  private plan: Plan = { widths: [1, 1, 1, 1, 1], depths: [1, 1, 1, 1, 1], keyed: undefined };
  private source: () => BayState[] = () => [];
  private still = false;
  private watch: MediaQueryList | undefined;
  private themed: MutationObserver | undefined;

  private constructor(private readonly host: HTMLElement) {}

  static start(host: HTMLElement, plan: Plan): ConsoleStage {
    const stage = new ConsoleStage(host);
    stage.plan = plan;
    stage.init().catch((error: unknown) => console.error("the console's canvas did not start", error));
    return stage;
  }

  private async init(): Promise<void> {
    const app = new Application();
    await app.init({ preference: "webgl", antialias: true, backgroundAlpha: 0, autoDensity: true, resolution: Math.min(window.devicePixelRatio || 1, 2), width: Math.max(1, this.width), height: 120, autoStart: false, sharedTicker: false, powerPreference: "low-power" });
    Ticker.system.stop();
    if (this.gone) return void app.destroy(true, { children: true });
    this.app = app;
    app.stage.eventMode = "none";
    app.canvas.setAttribute("aria-hidden", "true");
    this.host.append(app.canvas);
    this.tone = readTones(this.host);
    this.build();
    app.ticker.maxFPS = 30;
    app.ticker.add(this.tick);
    this.watch = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.still = this.watch.matches;
    this.watch.addEventListener("change", this.motion);
    this.themed = new MutationObserver(() => {
      this.tone = readTones(this.host);
      this.rebuild();
    });
    this.themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-still", "data-idle"] });
    document.addEventListener("visibilitychange", this.settle);
    this.resize(this.width);
  }

  /** One facet for each width, and one more for the key on a lip. */
  private build(): void {
    const { app, tone } = this;
    if (app === undefined || tone === undefined) return;
    this.world = worldOf(Math.max(1, this.width), this.plan);
    const sizes = this.plan.widths.map((w, i) => ({ w: Math.round(FW * w), h: Math.round(FH * this.plan.depths[i]!) }));
    if (this.plan.keyed !== undefined) sizes.push({ w: Math.round(FW * this.plan.widths[this.plan.keyed]!), h: Math.round((FW * this.plan.widths[this.plan.keyed]!) / this.world!.keyAspect) });
    for (const { w, h } of sizes) {
      const root = new Container();
      const g = new Graphics();
      const style = { fontFamily: tone.mono, fontSize: 9, letterSpacing: 1.5, fill: tone.subtle };
      const code = new Text({ text: "", style: { ...style }, resolution: 2 });
      const label = new Text({ text: "", style: { ...style, fontSize: 11, letterSpacing: 0 }, resolution: 2 });
      const extra = new Text({ text: "", style: { ...style }, resolution: 2 });
      code.x = 8;
      code.y = 6;
      root.addChild(g, code, label, extra);
      const tex = RenderTexture.create({ width: w, height: h, resolution: 2 });
      const mesh = new PerspectiveMesh({ texture: tex, verticesX: 12, verticesY: 12 });
      this.facets.push({ w, h, root, g, code, label, extra, tex, mesh });
    }
    this.stencil = new Text({ text: "SYS 01   NAV 0412.7   RNG 0.42   HDG 271", style: { fontFamily: tone.mono, fontSize: 9, letterSpacing: 2, fill: tone.subtle }, resolution: 2 });
    const keyed = this.plan.keyed === undefined ? [] : [this.facets.at(-1)!.mesh];
    const flat = this.plan.keyed === undefined ? this.facets : this.facets.slice(0, -1);
    app.stage.addChild(this.shell, ...flat.map((f) => f.mesh), this.shade, ...keyed, this.stencil);
  }

  private readonly motion = (): void => {
    this.still = this.watch?.matches ?? false;
    this.settle();
  };

  /** The ticker runs only while something moves: nothing under reduced motion, a hidden window or a folded panel, where one frame is held. */
  private readonly settle = (): void => {
    const root = document.documentElement;
    const quiet = this.hidden || document.hidden || root.hasAttribute("data-still") || root.hasAttribute("data-idle");
    if (quiet || this.still) this.app?.ticker.stop();
    else this.app?.ticker.start();
    if (this.app !== undefined && (quiet || this.still)) this.frame();
  };

  setHidden(hidden: boolean): void {
    this.hidden = hidden;
    this.settle();
  }

  /** The canvas's width in pixels; the height it needs comes back as the layout. */
  resize(width: number): Layout {
    this.width = Math.max(1, Math.round(width));
    const layout = layoutOf(this.width, this.plan);
    if (this.app !== undefined && this.tone !== undefined) {
      this.world = worldOf(this.width, this.plan);
      this.app.renderer.resize(this.width, layout.height);
      this.rebuild();
    }
    return layout;
  }

  /** Where each frame reads the bays from: the page's buttons, which hold the state. */
  setSource(source: () => BayState[]): void {
    this.source = source;
  }

  /** Something the page changed: draw once more where no frame is coming. */
  poke(): void {
    if (this.app !== undefined && (this.still || !this.app.ticker.started)) this.frame();
  }

  /** The shell, the lips and the shadows, flat polygons in the theme's tones. */
  private rebuild(): void {
    const { world, tone } = this;
    if (world === undefined || tone === undefined) return;
    const flat = (q: Quad) => q.flatMap((p) => [p.x, p.y]);
    const poly = (g: Graphics, q: Quad, fill: number, line: number | undefined, alpha = 1, width = 1.5) => {
      g.poly(flat(q)).fill({ color: fill, alpha });
      if (line !== undefined) g.poly(flat(q)).stroke({ width, color: line });
    };
    this.shell.clear();
    this.shade.clear();
    world.floor.forEach((q) => poly(this.shell, q, tone.base, undefined, 0.9));
    world.rims.forEach((q) => poly(this.shell, q, tone.edge, tone.muted, 1, 2));
    world.ends.forEach((q) => poly(this.shell, q, tone.sunken, tone.strong, 1, 2));
    world.shade.forEach((q) => poly(this.shade, q, tone.base, undefined, 0.55));
    world.quads.forEach((q) => poly(this.shade, q, tone.base, tone.strong, 0, 3));
    world.lips.forEach((q) => poly(this.shade, q, tone.sunken, tone.strong, this.plan.keyed === undefined ? 1 : 1, 2.5));
    world.lips.forEach((q) => this.shade.moveTo(q[0].x, q[0].y).lineTo(q[1].x, q[1].y).stroke({ width: 3, color: tone.muted }));
    world.quads.forEach((q) => this.shade.moveTo(q[0].x, q[0].y).lineTo(q[3].x, q[3].y).stroke({ width: 3.5, color: tone.muted }));
    this.facets.forEach((f, i) => {
      const q = i < world.quads.length ? world.quads[i]! : world.keyQuad;
      if (q !== undefined) f.mesh.setCorners(q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y);
    });
    const first = world.rims[0]!;
    const mid = world.rims[Math.floor(world.rims.length / 2)]!;
    if (this.stencil !== undefined) {
      this.stencil.x = first[0].x + 14;
      this.stencil.y = (first[0].y + first[3].y) / 2 - 5 + (mid[0].y - first[0].y) * 0.15;
      this.stencil.style.fill = tone.subtle;
      this.stencil.visible = this.width >= 760;
    }
    this.frame();
  }

  private readonly tick = (): void => this.frame();

  private frame(): void {
    const { app, tone } = this;
    if (app === undefined || tone === undefined || this.world === undefined) return;
    const t = this.still ? 0 : performance.now() / 1000;
    this.bays = this.source();
    this.facets.forEach((f, i) => {
      const state = this.bays[i] ?? { kind: null, pressed: false, calling: false, lit: false, code: "", label: "" };
      draw(f, state, tone, t, this.plan.keyed !== undefined && i === this.plan.widths.length);
      app.renderer.render({ container: f.root, target: f.tex, clear: true, clearColor: [0, 0, 0, 0] });
    });
    app.renderer.render(app.stage);
  }

  destroy(): void {
    this.gone = true;
    this.themed?.disconnect();
    this.watch?.removeEventListener("change", this.motion);
    document.removeEventListener("visibilitychange", this.settle);
    const app = this.app;
    if (app === undefined) return;
    this.app = undefined;
    app.ticker.remove(this.tick);
    app.ticker.stop();
    this.facets.forEach((f) => (f.tex.destroy(true), f.root.destroy({ children: true })));
    app.destroy({ removeView: true }, { children: true });
  }
}
