// What the canvas console is made of: the materials, mixed from the theme's tokens, and each facet's flat picture. A facet is a fixed plate,
// redrawn only when its state changes, and a live layer on the recessed screen.
// Units are CSS pixels across, and 1/f CSS pixels down, f being how far the surface is foreshortened: `hy` is the height of a line that lands at
// 1.5 CSS pixels. Depth comes from hard-edged bands, lit from above, with soft light from the screens (`drawLights`), the owner's exception to the no-glows rule.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { Graphics } from "pixi.js";

import { glyphOf } from "../flight-glyphs";
import { hex, readPalette, type Rgba } from "./palette";

export type Kind = "matrix" | "beacon" | "radar" | "trace" | "scope" | "needles" | "key" | "dispatch";
/** A dot of the Activity graph, 0 to 1 across the scope's box. */
export type Dot = { x: number; y: number; r: number; colour: string };
/** `focused` is the keyboard's focus: the ring for it is drawn here, so it lies on the surface rather than square across it. */
export type BayState = { kind: Kind | null; pressed: boolean; calling: boolean; /** Something new to read: steady, as a call pulses. */ lamp?: boolean; lit: boolean; focused: boolean; dots?: readonly Dot[]; /** The registry glyph a `key` facet wears. */ glyph?: string };

// ---- Materials ------------------------------------------------------------------------------------------------

const mix = (a: number, b: number, k: number): number => {
  const at = (shift: number) => Math.round(((a >> shift) & 255) * (1 - k) + ((b >> shift) & 255) * k) << shift;
  return at(16) | at(8) | at(0);
};
/** A token taken down by this share: a shadow on it, and no colour of its own. */
const deeper = (a: number, k: number): number => mix(a, 0, k);

export type Tones = { base: number; sunken: number; raised: number; strong: number; edge: number; quiet: number; fg: number; muted: number; subtle: number; accent: number; accentMuted: number; ask: number; bad: number; good: number; run: number };

/** Every surface of the console, named for what it is. */
export type Materials = ReturnType<typeof materialsOf>;
export function materialsOf(t: Tones) {
  return {
    /** The top of the hood and the nose of the lip, where the light lands. */
    glint: mix(t.strong, t.muted, 0.55),
    cap: mix(t.strong, t.muted, 0.2),
    hood: mix(t.edge, t.strong, 0.55),
    /** The facet plates. */
    hull: mix(t.raised, t.edge, 0.7),
    hullShade: mix(t.sunken, t.raised, 0.5),
    /** The band the labels stand on. */
    fascia: mix(t.raised, t.edge, 0.3),
    /** The front wall under the lip. */
    wall: mix(t.raised, t.edge, 0.2),
    kick: mix(t.edge, t.strong, 0.45),
    gap: deeper(t.sunken, 0.45),
    /** A screen and the wall of the well it sits in: the near wall catches the light, the far one is in shadow. */
    screen: deeper(t.sunken, 0.25),
    screenShade: deeper(t.sunken, 0.6),
    wellFar: deeper(t.sunken, 0.4),
    wellSide: mix(t.base, t.raised, 0.5),
    wellNear: mix(t.edge, t.strong, 0.7),
    /** Instrument ink that is off. */
    off: mix(t.raised, t.edge, 0.55),
    offTop: mix(t.edge, t.strong, 0.6),
    runTop: mix(t.run, t.fg, 0.45),
    askDark: mix(t.ask, t.sunken, 0.72),
    askLine: mix(t.ask, t.sunken, 0.45),
    askTop: mix(t.ask, t.fg, 0.55),
  };
}

export function readTones(host: HTMLElement): Tones & { mono: string } {
  const base = readPalette(host);
  const probe = document.createElement("span");
  host.append(probe);
  const colour = (name: string): number => {
    probe.style.color = "";
    probe.style.color = `var(${name})`;
    return cssHex(getComputedStyle(probe).color);
  };
  const tones = {
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

// ---- A facet's measures ------------------------------------------------------------------------------------------

type Rect = { x: number; y: number; w: number; h: number };
type Look = { tone: Tones; mat: Materials; W: number; H: number; f: number; u: number; v: number; hy: number };
/** A look and the screen's floor, which is where an instrument draws. */
type Box = Look & Rect & { cx: number; cy: number };

const TAU = Math.PI * 2;
const clamp = (n: number, low: number, high: number) => Math.min(high, Math.max(low, n));

const lookOf = (tone: Tones, mat: Materials, W: number, H: number, f: number): Look => {
  const v = 1 / f;
  return { tone, mat, W, H, f, v, u: clamp(W / 180, 0.5, 1.4), hy: 1.5 * v };
};
const inset = (r: Rect, x: number, y: number): Rect => ({ x: r.x + x, y: r.y + y, w: r.w - x * 2, h: r.h - y * 2 });

/** A rectangle with its corners cut. */
const cut = (g: Graphics, r: Rect, c: number, cy: number) => g.poly([r.x + c, r.y, r.x + r.w - c, r.y, r.x + r.w, r.y + cy, r.x + r.w, r.y + r.h - cy, r.x + r.w - c, r.y + r.h, r.x + c, r.y + r.h, r.x, r.y + r.h - cy, r.x, r.y + cy]);

/** A round-headed fastener, its slot turned so no two read alike. */
function bolt(g: Graphics, l: Look, x: number, y: number, r: number, turn: number): void {
  g.circle(x, y + 0.7 * l.v, r + 0.8).fill(l.mat.gap);
  g.circle(x, y, r).fill(l.mat.cap);
  const [dx, dy] = [Math.cos(turn) * r * 0.75, Math.sin(turn) * r * 0.75];
  g.moveTo(x - dx, y - dy).lineTo(x + dx, y + dy).stroke({ width: Math.max(1, r * 0.45), color: l.mat.gap });
}

const BEZEL = (u: number) => clamp(2.5 * u, 1.5, 2.5);
/** The recessed well: a bevelled frame in the state's colour, the four walls going down, and the floor with the far wall's shadow across it. */
function well(g: Graphics, l: Look, r: Rect, frame: number, floor: number): Rect {
  const { u, v, mat } = l;
  const [bx, by] = [BEZEL(u), BEZEL(u) * v];
  const [dx, dy] = [Math.max(2, 4.5 * u), Math.max(2, 4.5 * u) * v];
  const c = 8 * u;
  cut(g, r, c, c * v).fill(frame);
  const o = inset(r, bx, by);
  const c2 = Math.max(1, c - bx);
  cut(g, o, c2, c2 * v).fill(mat.wellSide);
  const n = inset(o, dx, dy);
  const c3 = Math.max(1, c2 - dx * 0.5);
  g.poly([o.x + c2, o.y, o.x + o.w - c2, o.y, n.x + n.w - c3, n.y, n.x + c3, n.y]).fill(mat.wellFar);
  g.poly([o.x + c2, o.y + o.h, o.x + o.w - c2, o.y + o.h, n.x + n.w - c3, n.y + n.h, n.x + c3, n.y + n.h]).fill(mat.wellNear);
  cut(g, n, c3, c3 * v).fill(floor);
  g.rect(n.x + c3 * 0.5, n.y, n.w - c3, Math.max(l.hy, 3 * u * v)).fill(mat.screenShade);
  return n;
}

/** A row of hard slots, the vents on a plate. */
function slots(g: Graphics, l: Look, x: number, y: number, w: number, h: number): void {
  const count = Math.max(2, Math.min(5, Math.floor(h / (l.hy * 4.4))));
  const step = h / count;
  for (let k = 0; k < count; k += 1) {
    g.rect(x, y + k * step, w, step * 0.56).fill(l.mat.gap);
    g.rect(x, y + k * step + step * 0.56, w, l.hy * 0.5).fill(l.mat.wellNear);
  }
}

/** `side`: how much of each end the plate covers, so no stripe is left as a sliver there. */
function hazard(g: Graphics, l: Look, y: number, h: number, side = 0): void {
  const step = 15 * l.u;
  g.rect(0, y, l.W, h).fill(l.mat.gap);
  const [top, foot] = [y + l.hy, y + h - l.hy];
  const lean = (foot - top) * 0.55;
  for (let k = -1; k < l.W / step + 2; k += 1) {
    if (side > 0 && (k * step + step * 0.5 < side + step * 0.5 || k * step - lean > l.W - side - step * 0.5)) continue;
    g.poly([k * step, top, k * step + step * 0.5, top, k * step + step * 0.5 - lean, foot, k * step - lean, foot]).fill(l.tone.ask);
  }
  g.rect(0, y + h - l.hy * 0.6, l.W, l.hy * 0.6).fill(l.mat.wellNear);
}

/** The plate under every facet: a block of hull with a hard bevel down each side, a seam, and a fastener in each corner where they would still read. */
function plateOf(g: Graphics, l: Look): void {
  const { W, H, hy, mat } = l;
  g.rect(0, 0, W, H).fill(mat.hull);
  g.rect(0, 0, W, hy).fill(mat.hullShade);
  g.rect(0, H - hy, W, hy).fill(mat.hood);
}
function fasten(g: Graphics, l: Look, mx: number, my: number): void {
  if (l.W < 118) return;
  const r = 2.4 * l.u + 0.4;
  bolt(g, l, mx, my, r, 0.6);
  bolt(g, l, l.W - mx, my, r, 2.1);
  bolt(g, l, mx, l.H - my, r, 1.2);
  bolt(g, l, l.W - mx, l.H - my, r, -0.4);
}

// ---- The instruments -------------------------------------------------------------------------------------------

type Instrument = { fixed: (g: Graphics, b: Box, s: BayState) => void; live?: (g: Graphics, b: Box, s: BayState, t: number) => void };

const matrixCells = (b: Box) => {
  const cw = clamp(b.w / 15, 4.5, 10);
  const gap = Math.max(1.8, cw * 0.26);
  const pad = Math.max(3, cw * 0.5);
  const ch = cw * 0.5 * b.v;
  const gy = gap * b.v;
  const cols = Math.max(4, Math.floor((b.w - pad * 2 + gap) / (cw + gap)));
  const rows = Math.max(2, Math.floor((b.h - pad * 2 * b.v + gy) / (ch + gy)));
  const [x0, y0] = [b.cx - (cols * (cw + gap) - gap) / 2, b.cy - (rows * (ch + gy) - gy) / 2];
  return { cw, ch, cols, rows, at: (c: number, r: number) => [x0 + c * (cw + gap), y0 + r * (ch + gy)] as const };
};

/** Tick marks round an arc, heavy every `each`. */
function ticks(g: Graphics, cx: number, cy: number, r: number, from: number, to: number, count: number, each: number, inner: number, width: number, colour: number): void {
  for (let k = 0; k <= count; k += 1) {
    const a = from + ((to - from) * k) / count;
    const heavy = k % each === 0;
    const start = r - (heavy ? inner : inner * 0.55);
    g.moveTo(cx + Math.cos(a) * start, cy + Math.sin(a) * start).lineTo(cx + Math.cos(a) * r, cy + Math.sin(a) * r).stroke({ width: heavy ? width * 1.4 : width, color: colour });
  }
}

function grid(g: Graphics, b: Box, across: number, down: number): void {
  for (let k = 1; k < across; k += 1) g.rect(b.x + (b.w * k) / across, b.y, 1.5, b.h).fill(b.mat.off);
  for (let k = 1; k < down; k += 1) g.rect(b.x, b.y + (b.h * k) / down, b.w, b.hy * 0.8).fill(b.mat.off);
}

/** The level bars beside the radar; `litOnly` leaves out those that are off. */
function radarBars(g: Graphics, b: Box, x: number, room: number, side: number, litOnly: boolean): void {
  const bars = Math.max(3, Math.floor(room / 11));
  const ground = b.y + b.h * 0.62;
  for (let k = 0; k < bars; k += 1) {
    const on = (k + side) % 3 === 0;
    if (!on && litOnly) continue;
    const tall = b.h * (0.16 + 0.085 * ((k * 3 + side * 2) % 5));
    g.rect(x + (k * room) / bars, ground - tall, (room / bars) * 0.62, tall).fill(on ? b.tone.run : b.mat.offTop);
  }
}

const collar = (g: Graphics, b: Box, r: number, ring: number) => {
  g.circle(b.cx, b.cy + ring * 0.8, r).fill(b.mat.gap);
  g.circle(b.cx, b.cy, r).fill(b.mat.cap);
  g.circle(b.cx, b.cy, r - ring).fill(b.mat.gap);
};

/** The side of lucide's own drawing board. */
const BOARD = 24;
const drawn = new Map<string, string>();

/** A registry glyph stroked on the plate, `size` across and centred on (`cx`, `cy`). The canvas reads lucide's own SVG, which the rest of the app shows. */
function glyphOn(g: Graphics, name: string, cx: number, cy: number, size: number, stroke: number): void {
  const colour = `#${stroke.toString(16).padStart(6, "0")}`;
  const at = `${name}|${colour}`;
  let svg = drawn.get(at);
  if (svg === undefined) {
    svg = renderToStaticMarkup(createElement(glyphOf(name, "zap")))
      .replace(/ stroke="currentColor"/, "")
      .replace(/<(path|circle|rect|line|polyline|polygon|ellipse)\b/g, `<$1 fill="none" stroke="${colour}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"`);
    drawn.set(at, svg);
  }
  const k = size / BOARD;
  g.save().setTransform(k, 0, 0, k, cx - (BOARD / 2) * k, cy - (BOARD / 2) * k).svg(svg).restore();
}

const INSTRUMENTS: Record<Exclude<Kind, "dispatch">, Instrument> = {
  matrix: {
    fixed(g, b) {
      const { cw, ch, cols, rows, at } = matrixCells(b);
      for (let c = 0; c < cols; c += 1) {
        for (let r = 0; r < rows; r += 1) {
          const [x, y] = at(c, r);
          g.rect(x, y, cw, ch).fill(b.mat.off);
          g.rect(x, y, cw, Math.min(ch * 0.3, b.hy)).fill(b.mat.offTop);
        }
      }
    },
    live(g, b, _s, t) {
      const { cw, ch, cols, rows, at } = matrixCells(b);
      for (let r = 0; r < rows; r += 1) {
        const lit = Math.round(cols * (0.58 + 0.28 * Math.sin(Math.floor(t * 2) * 1.7 + r * 1.9)));
        for (let c = 0; c < lit; c += 1) {
          const [x, y] = at(c, r);
          g.rect(x, y, cw, ch).fill(b.tone.run);
          g.rect(x, y, cw, Math.min(ch * 0.3, b.hy)).fill(b.mat.runTop);
        }
      }
    },
  },
  beacon: {
    fixed(g, b) {
      const r = Math.min(b.w, b.h) / 2 - 1;
      if (b.w > b.h * 1.5) {
        const side = (b.w - r * 2) / 2 - 8 * b.u;
        const [high, tall] = [b.cy - r * 0.6, r * 1.2];
        slots(g, b, b.x + 5 * b.u, high, side - 2, tall);
        slots(g, b, b.x + b.w - 5 * b.u - side + 2, high, side - 2, tall);
      }
      collar(g, b, r, r * 0.2);
      g.circle(b.cx, b.cy, r * 0.66).fill(b.mat.askDark);
    },
    live(g, b, s, t) {
      const r = Math.min(b.w, b.h) / 2 - 1;
      if (s.calling) g.circle(b.cx, b.cy, r * 0.66).fill({ color: b.tone.ask, alpha: 0.7 + 0.3 * Math.abs(Math.sin(t * 3.4)) });
      for (let n = 0; n < 12; n += 1) {
        const a = (n / 12) * TAU + (s.calling ? t * 1.6 : 0);
        g.moveTo(b.cx + Math.cos(a) * r * 0.81, b.cy + Math.sin(a) * r * 0.81).arc(b.cx, b.cy, r * 0.81, a, a + 0.3).stroke({ width: r * 0.14, color: s.calling ? b.tone.ask : b.mat.askDark });
      }
      for (const k of [0.46, 0.27]) g.circle(b.cx, b.cy, r * k).stroke({ width: Math.max(b.hy * 0.8, r * 0.05), color: s.calling ? b.mat.askLine : b.mat.gap });
      g.circle(b.cx, b.cy, r * 0.11).fill(s.calling ? b.mat.askTop : b.mat.askLine);
    },
  },
  radar: {
    fixed(g, b) {
      const r = Math.min(b.w, b.h) / 2 - 1;
      const ring = Math.max(b.hy, 3.2 * b.u);
      const room = (b.w - r * 2) / 2 - 10 * b.u;
      if (room > 36) {
        for (const side of [0, 1]) {
          const x = side === 0 ? b.x + 7 * b.u : b.x + b.w - 7 * b.u - room;
          g.rect(side === 0 ? x + room + 3 * b.u : x - 3 * b.u - 1.5, b.y, 1.5, b.h).fill(b.mat.off);
          radarBars(g, b, x, room, side, false);
          const ground = b.y + b.h * 0.62;
          g.rect(x, ground + b.hy, room, b.hy).fill(b.mat.offTop);
          const rule = Math.max(b.hy, 2.6 * b.u * b.v);
          for (let k = 0; k < 2; k += 1) g.rect(x, b.y + b.h * (0.74 + k * 0.13), room * (0.5 + ((k + side) % 2) * 0.35), rule).fill(b.mat.offTop);
        }
      }
      collar(g, b, r, ring);
      g.circle(b.cx, b.cy, r - ring).fill(mix(b.mat.screen, b.tone.accentMuted, 0.4));
      for (const k of r > 24 ? [0.68, 0.36] : [0.5]) g.circle(b.cx, b.cy, r * k).stroke({ width: b.hy * 0.8, color: b.mat.offTop });
      g.moveTo(b.cx - r + ring, b.cy).lineTo(b.cx + r - ring, b.cy).moveTo(b.cx, b.cy - r + ring).lineTo(b.cx, b.cy + r - ring).stroke({ width: b.hy * 0.8, color: b.mat.off });
      ticks(g, b.cx, b.cy, r - ring, 0, TAU, r > 40 ? 48 : r > 24 ? 24 : 12, r > 40 ? 6 : 3, Math.max(4, r * 0.1), Math.max(1.5, b.hy * 0.7), b.tone.subtle);
    },
    live(g, b, s, t) {
      const r = Math.min(b.w, b.h) / 2 - 1 - Math.max(b.hy, 3.2 * b.u);
      const sweep = t * 1.3;
      g.moveTo(b.cx, b.cy).arc(b.cx, b.cy, r - 1, sweep - 1.1, sweep - 0.5).closePath().fill({ color: b.tone.run, alpha: 0.14 });
      g.moveTo(b.cx, b.cy).arc(b.cx, b.cy, r - 1, sweep - 0.5, sweep).closePath().fill({ color: b.tone.run, alpha: 0.32 });
      g.moveTo(b.cx, b.cy).lineTo(b.cx + Math.cos(sweep) * (r - 1), b.cy + Math.sin(sweep) * (r - 1)).stroke({ width: Math.max(2.5, b.hy), color: b.tone.run });
      for (const [bx, by] of [[0.3, -0.35], [-0.4, 0.3], [0.45, 0.2]] as const) g.circle(b.cx + bx * r, b.cy + by * r, Math.max(2.2, r * 0.05)).fill(b.tone.fg);
      g.circle(b.cx, b.cy, Math.max(2.5, r * 0.06)).fill(s.pressed ? b.tone.accent : b.tone.muted);
    },
  },
  trace: {
    fixed(g, b) {
      grid(g, b, Math.max(3, Math.round(b.w / 30)), 3);
    },
    live(g, b, s, t) {
      const period = Math.max(26, b.w / 3.4);
      const k = period / 40;
      const amp = Math.min(1, (b.h * 0.4) / 17);
      const shift = (t * 30) % period;
      const base = b.cy + 2 * b.v;
      g.moveTo(b.x + 3, base);
      for (let px = b.x + 3; px <= b.x + b.w - 3; px += 1.5) {
        const p = ((px - b.x + shift) % period) / k;
        const y = p < 8 ? 0 : p < 11 ? -(p - 8) * 5 : p < 15 ? -15 + (p - 11) * 8 : p < 19 ? 17 - (p - 15) * 4.2 : 0;
        g.lineTo(px, base + y * amp);
      }
      g.stroke({ width: Math.max(2.4, b.hy * 1.1), color: b.tone.run, join: "bevel" });
      g.rect(b.x + b.w * 0.55, b.y, Math.max(2, 2.4 * b.u), b.h).fill(b.tone.ask);
      if (s.lamp === true) g.circle(b.x + b.w - 6 * b.u, b.y + 6 * b.u, Math.max(3, 3 * b.u)).fill(b.tone.accent);
    },
  },
  scope: {
    fixed(g, b) {
      grid(g, b, Math.max(3, Math.round(b.w / 30)), 3);
      g.rect(b.x + b.w - 7 * b.u, b.y, Math.max(2, 2.4 * b.u), b.h).fill(b.tone.ask);
    },
    live(g, b, s) {
      for (const d of s.dots ?? []) g.circle(b.x + 4 + d.x * (b.w - 8), b.y + 3 * b.v + d.y * (b.h - 6 * b.v), Math.max(2.4, d.r * 1.1)).fill(cssHex(d.colour));
    },
  },
  key: {
    // A cap standing on the screen's floor, its top edge lit as the matrix cells' are, and the glyph on it. The word is the lip's, DOM text.
    fixed(g, b, s) {
      const side = Math.min(b.w, b.h) * 0.78;
      const [x, y] = [b.cx - side / 2, b.cy - side / 2];
      g.roundRect(x, y + side * 0.08, side, side, side * 0.12).fill(b.mat.gap);
      g.roundRect(x, y, side, side, side * 0.12).fill(b.mat.hood);
      g.rect(x + side * 0.06, y, side * 0.88, Math.min(side * 0.1, b.hy * 2.5)).fill(b.mat.glint);
      glyphOn(g, s.glyph ?? "zap", b.cx, b.cy, side * 0.5, s.pressed ? b.tone.accent : b.tone.fg);
    },
  },
  needles: {
    fixed(g, b) {
      for (const [k, px, r, py] of dials(b)) {
        const ring = Math.max(b.hy, 3 * b.u);
        g.moveTo(px - r, py).arc(px, py, r, Math.PI, Math.PI * 1.68).stroke({ width: ring, color: b.mat.offTop });
        g.moveTo(px + Math.cos(Math.PI * 1.68) * r, py + Math.sin(Math.PI * 1.68) * r).arc(px, py, r, Math.PI * 1.68, TAU).stroke({ width: ring, color: k === 0 ? b.tone.ask : b.tone.run });
        ticks(g, px, py, r - ring / 2 - 1, Math.PI, TAU, 8, 2, Math.max(3, r * 0.2), 1.6, b.tone.subtle);
      }
    },
    live(g, b, _s, t) {
      for (const [k, px, r, py] of dials(b)) {
        const a = Math.PI * 1.5 + Math.sin(t * (k === 0 ? 1.3 : 0.9) + k) * 0.7;
        g.moveTo(px, py).lineTo(px + Math.cos(a) * r * 0.82, py + Math.sin(a) * r * 0.82).stroke({ width: Math.max(2.6, b.hy), color: k === 0 ? b.tone.ask : b.tone.run });
        g.circle(px, py, Math.max(2.4, r * 0.13)).fill(b.mat.cap);
      }
    },
  },
};

const dials = (b: Box): (readonly [number, number, number, number])[] => {
  const r = Math.min(b.w * 0.2, b.h * 0.72);
  const py = b.y + b.h * 0.5 + r * 0.42;
  return [[0, b.x + b.w * 0.27, r, py], [1, b.x + b.w * 0.73, r, py]];
};

// ---- Dispatch: a guarded cap on the Map's lip ----------------------------------------------------------------------

function capOf(g: Graphics, l: Look, x: number, y: number, r: number, s: BayState): void {
  const { mat, tone, v, u } = l;
  const lift = Math.max(2, 5 * u) * v;
  g.circle(x, y + lift * 1.4, r + 1).fill(mat.gap);
  g.circle(x, y + lift, r).fill(mat.hullShade);
  g.rect(x - r, y, r * 2, lift).fill(mat.hullShade);
  g.circle(x, y, r).fill(s.pressed ? tone.accent : s.lit ? tone.fg : mat.cap);
  const inner = r - Math.max(2.5, r * 0.11);
  g.circle(x, y, inner).fill(s.pressed ? tone.accentMuted : mat.hood);
  g.circle(x, y, inner * 0.8).fill(s.pressed ? tone.accentMuted : mat.hull);
  planeOf(g, x, y, r, tone.accent);
  g.poly(shadeOfPlane(x, y, r)).fill(mix(tone.accent, tone.sunken, 0.45));
}

function planeOf(g: Graphics, x: number, y: number, r: number, colour: number): void {
  const k = r / 38;
  g.poly([x - 21 * k, y + 5 * k, x + 23 * k, y - 15 * k, x + 7 * k, y + 19 * k, x - 2 * k, y + 8 * k]).fill(colour);
}
const shadeOfPlane = (x: number, y: number, r: number) => {
  const k = r / 38;
  return [x - 2 * k, y + 8 * k, x + 23 * k, y - 15 * k, x + 7 * k, y + 19 * k];
};

/** Where the Dispatch cap stands on its plate. */
function capSpotOf(l: Look): { x: number; y: number; r: number } {
  const { W, H, u, v } = l;
  const strip = 9 * u * v;
  const rest = H - strip;
  return { x: W * keySpots(W).cap, y: strip + rest * 0.45, r: Math.min(rest * 0.38, 34 * u) };
}

/** How the key on a lip is set out, across its width: narrow, it drops its vents and lamp so the cap and the word have the room. */
export const keySpots = (W: number) => (W < 260 ? { cap: 0.2, word: 0.6, full: false } : { cap: 0.4, word: 0.61, full: true });

function keyPlate(g: Graphics, l: Look, s: BayState): void {
  const { W, H, u, v, mat } = l;
  const strip = 9 * u * v;
  plateOf(g, l);
  hazard(g, l, 0, strip);
  const rest = H - strip;
  const spots = keySpots(W);
  const cap = capSpotOf(l);
  capOf(g, l, cap.x, cap.y, cap.r, s);
  if (spots.full) {
    const inset = 18 * u;
    const room = W * 0.15;
    slots(g, l, inset, strip + rest * 0.26, room, rest * 0.5);
    slots(g, l, W - inset - room, strip + rest * 0.26, room, rest * 0.5);
  }
  for (const x of [8 * u, W - 8 * u]) {
    bolt(g, l, x, strip + rest * 0.28, 2.4 * u + 0.4, x);
    bolt(g, l, x, strip + rest * 0.76, 2.4 * u + 0.4, x + 1);
  }
  g.rect(0, H - v, W, v).fill(mat.gap);
}

// ---- A facet --------------------------------------------------------------------------------------------------

/** The screen's floor for a facet, or undefined where it has no screen. `named`: the bottom of the plate is kept for its label, where its lip is taken. */
function floorOf(l: Look, named: boolean): Box {
  const { W, H, u, v } = l;
  const [mx, my] = [6 * u, 6 * u * v];
  const room = { x: mx, y: my, w: W - mx * 2, h: H - my * 2 - (named ? 15 * v : 0) };
  const floor = inset(room, BEZEL(u) + Math.max(2, 4.5 * u), (BEZEL(u) + Math.max(2, 4.5 * u)) * v);
  return { ...l, ...floor, cx: floor.x + floor.w / 2, cy: floor.y + floor.h / 2 };
}

export type Paint = { W: number; H: number; f: number; lipKey: boolean; named: boolean };

/** The fixed part of a facet. */
export function drawPlate(g: Graphics, p: Paint, s: BayState, tone: Tones, mat: Materials): void {
  const l = lookOf(tone, mat, p.W, p.H, p.f);
  g.clear();
  paintPlate(g, l, p, s);
  if (s.focused && p.lipKey) {
    const [x, y] = [2.5, 2.5 * l.v];
    g.rect(0, 0, p.W, y).rect(0, p.H - y, p.W, y).rect(0, y, x, p.H - y * 2).rect(p.W - x, y, x, p.H - y * 2).fill(tone.fg);
  }
}

function paintPlate(g: Graphics, l: Look, p: Paint, s: BayState): void {
  const { tone, mat } = l;
  if (s.kind === "dispatch") return keyPlate(g, l, s);
  const b = floorOf(l, p.named);
  const room: Rect = { x: 6 * l.u, y: 6 * l.u * l.v, w: p.W - 12 * l.u, h: p.H - 12 * l.u * l.v - (p.named ? 15 * l.v : 0) };
  plateOf(g, l);
  if (s.kind === null) {
    const n = well(g, l, room, mat.hullShade, mat.wellFar);
    const count = Math.max(3, Math.floor(n.w / 16));
    for (let k = 0; k < count; k += 1) g.rect(n.x + ((k + 0.3) * n.w) / count, n.y + n.h * 0.68, (n.w / count) * 0.4, n.h * 0.16).fill(mat.wellSide);
    fasten(g, l, 5 * l.u, 5 * l.u * l.v);
    return;
  }
  well(g, l, room, s.focused ? tone.fg : s.pressed ? tone.accent : s.lit ? tone.muted : mat.gap, s.pressed ? mix(mat.screen, tone.accentMuted, 0.65) : mat.screen);
  fasten(g, l, 5 * l.u, 5 * l.u * l.v);
  INSTRUMENTS[s.kind].fixed(g, b, s);
}

/** Dispatch's arming lamp: where it stands, if the plate has room for it, and whether it is on at this moment. */
function lampOf(l: Look): Rect | undefined {
  const { u, v, hy, W, H } = l;
  if (!keySpots(W).full) return undefined;
  const strip = 9 * u * v;
  return { x: W * 0.53, y: strip + (H - strip) * 0.19, w: W * 0.16, h: Math.max(hy * 1.4, 4 * u * v) };
}
const armed = (t: number) => Math.floor(t * 1.4) % 2 === 0;

/** What moves on a facet's screen, drawn each frame. */
export function drawLive(g: Graphics, p: Paint, s: BayState, tone: Tones, mat: Materials, t: number): void {
  g.clear();
  if (s.kind === null) return;
  const l = lookOf(tone, mat, p.W, p.H, p.f);
  if (s.kind === "dispatch") {
    const lamp = lampOf(l);
    if (lamp !== undefined) g.rect(lamp.x, lamp.y, lamp.w, lamp.h).fill(armed(t) ? tone.ask : mat.off);
    return;
  }
  INSTRUMENTS[s.kind].live?.(g, floorOf(l, p.named), s, t);
}

// ---- The lights ----------------------------------------------------------------------------------------

/** What gives light on a facet besides its moving marks: the screen's own glass, and the marks of the plate that are lit. The stage blurs this with what moves, once, and adds it over the whole console. Dispatch's lamp blinks, so its facet is redrawn each frame. */
export function drawLights(g: Graphics, p: Paint, s: BayState, tone: Tones, mat: Materials, t: number): void {
  g.clear();
  if (s.kind === null) return;
  const l = lookOf(tone, mat, p.W, p.H, p.f);
  if (s.kind === "dispatch") {
    const cap = capSpotOf(l);
    g.circle(cap.x, cap.y, cap.r * 0.8).fill({ color: tone.accent, alpha: 0.32 });
    planeOf(g, cap.x, cap.y, cap.r, tone.accent);
    const lamp = lampOf(l);
    if (lamp !== undefined && armed(t)) g.rect(lamp.x - 3 * l.u, lamp.y - 3 * l.u * l.v, lamp.w + 6 * l.u, lamp.h + 6 * l.u * l.v).fill(tone.ask);
    return;
  }
  // A key throws no light: it is a cap, not a screen.
  if (s.kind === "key") return;
  const b = floorOf(l, p.named);
  g.rect(b.x, b.y, b.w, b.h).fill({ color: glassOf(s, tone), alpha: s.pressed ? GLASS * 0.7 : GLASS });
  if (s.kind === "needles") INSTRUMENTS[s.kind].fixed(g, b, s);
  if (s.kind === "radar") {
    const room = (b.w - (Math.min(b.w, b.h) - 2)) / 2 - 10 * b.u;
    if (room > 36) for (const side of [0, 1]) radarBars(g, b, side === 0 ? b.x + 7 * b.u : b.x + b.w - 7 * b.u - room, room, side, true);
  }
}
/** How strongly a screen's glass is lit. */
const GLASS = 0.25;
/** The colour a facet's screen throws, or undefined where it has no screen. */
export const glassOf = (s: BayState, tone: Tones): number => (s.kind === "beacon" && s.calling ? tone.ask : s.pressed ? tone.accent : tone.run);
