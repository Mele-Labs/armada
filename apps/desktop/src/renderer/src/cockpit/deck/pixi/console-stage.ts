// The bridge take's console drawn on a canvas (mock): a cone section of a cockpit dashboard seen from the pilot seat, its facets joined edge to
// edge, the wings sweeping toward the viewer. Each facet is drawn flat, with its instrument, onto a texture (`console-art.ts`), and the texture is put
// on the facet's projected quad, so the instrument skews with the surface. Hood, lip, front wall and ribs are bands of flat polygons, lit from above.
// The page lays a transparent button and a straight label over each facet (`ConsoleBays.tsx`), which keep the presses, tooltips and words.
// No gradients and no glow: the depth is the projection, the layering and the hard edges. Does not touch the map's stage.

import "pixi.js/unsafe-eval";
import { Application, Container, Graphics, PerspectiveMesh, RenderTexture, Ticker } from "pixi.js";

import { drawLive, drawPlate, keySpots, materialsOf, readTones, type BayState, type Materials, type Paint, type Tones } from "./console-art";

export type { BayState, Dot, Kind } from "./console-art";
export type Rect = { x: number; y: number; w: number; h: number };
type P = { x: number; y: number };
type Quad = readonly [P, P, P, P];

/** Where a bay's label stands: its centre, the angle of the edge it follows, and its size in pixels. */
export type Label = { x: number; y: number; angle: number; size: number };
/** Where each facet's button goes, and how tall the canvas has to be. */
export type Layout = { width: number; height: number; inner: Rect[]; labels: Label[]; rim: Rect | undefined };

/** How wide each facet is, in units of one ordinary facet, and which facet carries Dispatch as a key on its lip (none where Dispatch is a facet). */
export type Plan = { widths: readonly number[]; depths: readonly number[]; keyed: number | undefined };
const FW = 190;
const FH = 108;
/** A facet's texture holds this many texels for each device pixel it lands on, so it is never magnified and nothing minifies enough to shimmer. */
const TEXELS = 1.5;

// ---- The projection: a pinhole over a cone ----------------------------------------------------------------

const CONE = { R: 1250, U: 150, slope: (30 * Math.PI) / 180, step: 0.16, behind: 1100, focal: 1500, lipY: -190, raise: 60, thick: 30, rim: 17, key: 2.4, rib: 0.0034 };

/** A piece of a structural rib, by the face it crosses and which of its three columns: lit, flat, shaded. */
type RibFace = "rim" | "deck" | "lip" | "wall";
type World = { quads: Quad[]; lips: Quad[]; rims: Quad[]; ends: Quad[]; floor: Quad[]; shade: Quad[]; ribs: { face: RibFace; column: number; quad: Quad }[]; keyQuad: Quad | undefined; keyAspect: number; height: number };

function worldOf(width: number, plan: Plan): World {
  const { R, U: baseU, slope, step, behind, focal, lipY, raise, thick, rim, key, rib } = CONE;
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
  const tOf = (i: number) => (i === plan.keyed ? thick * key : thick);
  const uOf = (i: number) => baseU * plan.depths[i]!;
  const quads: Quad[] = [];
  const lips: Quad[] = [];
  const rims: Quad[] = [];
  const floor: Quad[] = [];
  const shade: Quad[] = [];
  let keyQuad: Quad | undefined;
  let keyAspect = 1;
  plan.widths.forEach((_, i) => {
    const [a, b] = [edges[i]!, edges[i + 1]!];
    const t = tOf(i);
    const U = uOf(i);
    quads.push([at(a, U), at(b, U), at(b, 0), at(a, 0)]);
    const lip: Quad = [at(a, 0), at(b, 0), at(b, 0, t), at(a, 0, t)];
    lips.push(lip);
    if (i === plan.keyed) {
      keyQuad = lip;
      keyAspect = (R * (b - a)) / t;
    }
    rims.push([at(a, U, 0, rim), at(b, U, 0, rim), at(b, U), at(a, U)]);
    floor.push([at(a, 0, t), at(b, 0, t), at(b, 0, t + thick * 0.9), at(a, 0, t + thick * 0.9)]);
    const near = U - 12;
    shade.push([at(a, U), at(b, U), at(b, near), at(a, near)]);
  });
  const ends: Quad[] = [edges[0]!, edges.at(-1)!].map((a, k) => {
    const U = uOf(k === 0 ? 0 : plan.widths.length - 1);
    return [at(a, 0), at(a, U), at(a, U, thick), at(a, 0, thick * 1.9)] as const;
  });
  // A rib stands on every join, proud of the taller neighbour; the two at the ends are heavier.
  const ribs: World["ribs"] = [];
  edges.forEach((a, n) => {
    const around = [n - 1, n].filter((i) => i >= 0 && i < plan.widths.length);
    const U = Math.max(...around.map(uOf));
    const t = Math.max(...around.map(tOf));
    const half = rib * (n === 0 || n === edges.length - 1 ? 1.6 : 1);
    const columns = [a - half, a - half * 0.35, a + half * 0.45, a + half];
    for (let column = 0; column < 3; column += 1) {
      const [l, r] = [columns[column]!, columns[column + 1]!];
      ribs.push({ face: "rim", column, quad: [at(l, U, 0, rim), at(r, U, 0, rim), at(r, U), at(l, U)] });
      ribs.push({ face: "deck", column, quad: [at(l, U), at(r, U), at(r, 0), at(l, 0)] });
      ribs.push({ face: "lip", column, quad: [at(l, 0), at(r, 0), at(r, 0, t), at(l, 0, t)] });
      ribs.push({ face: "wall", column, quad: [at(l, 0, t), at(r, 0, t), at(r, 0, t + thick * 0.9), at(l, 0, t + thick * 0.9)] });
    }
  });
  const every = [...quads, ...lips, ...rims, ...floor, ...ends, ...ribs.map((one) => one.quad)].flat();
  const [minX, maxX] = [Math.min(...every.map((p) => p.x)), Math.max(...every.map((p) => p.x))];
  const [minY, maxY] = [Math.min(...every.map((p) => p.y)), Math.max(...every.map((p) => p.y))];
  const margin = 4;
  const scale = (width - margin * 2) / (maxX - minX);
  const move = (p: P): P => ({ x: (p.x - minX) * scale + margin, y: (p.y - minY) * scale + margin });
  const fit = (q: Quad): Quad => [move(q[0]), move(q[1]), move(q[2]), move(q[3])];
  return { quads: quads.map(fit), lips: lips.map(fit), rims: rims.map(fit), ends: ends.map(fit), floor: floor.map(fit), shade: shade.map(fit), ribs: ribs.map((one) => ({ ...one, quad: fit(one.quad) })), keyQuad: keyQuad === undefined ? undefined : fit(keyQuad), keyAspect, height: Math.ceil((maxY - minY) * scale + margin * 2) };
}

const boxOf = (q: Quad): Rect => {
  const x = Math.max(q[0].x, q[3].x);
  const y = Math.max(q[0].y, q[1].y);
  return { x, y, w: Math.min(q[1].x, q[2].x) - x, h: Math.min(q[2].y, q[3].y) - y };
};

/** A point of a quad, 0 to 1 across and down. */
const inQuad = (q: Quad, x: number, y: number): P => {
  const [ax, ay] = [q[0].x + (q[1].x - q[0].x) * x, q[0].y + (q[1].y - q[0].y) * x];
  const [bx, by] = [q[3].x + (q[2].x - q[3].x) * x, q[3].y + (q[2].y - q[3].y) * x];
  return { x: ax + (bx - ax) * y, y: ay + (by - ay) * y };
};
/** The part of a quad between two fractions across and two down. */
const partOf = (q: Quad, x0: number, y0: number, x1: number, y1: number): Quad => [inQuad(q, x0, y0), inQuad(q, x1, y0), inQuad(q, x1, y1), inQuad(q, x0, y1)];
const span = (a: P, b: P) => Math.hypot(b.x - a.x, b.y - a.y);

/** Each label follows the lip below its facet; a keyed facet's lip is Dispatch's, so its label keeps to the plate's foot. */
function labelsOf(world: World, plan: Plan): Label[] {
  const edge = (q: Quad) => Math.atan2(q[1].y - q[0].y, q[1].x - q[0].x);
  const labels = world.lips.map((lip, i): Label => {
    const quad = world.quads[i]!;
    const tall = span(lip[0], lip[3]);
    const size = tall < 14 ? 10 : tall < 18 ? 11 : 12;
    if (i === plan.keyed) {
      const spot = inQuad(quad, 0.5, 1 - 8 / span(quad[0], quad[3]));
      return { ...spot, angle: edge(quad), size: 12 };
    }
    return { ...inQuad(lip, 0.5, 0.58), angle: edge(lip), size };
  });
  if (world.keyQuad !== undefined) labels.push({ ...inQuad(world.keyQuad, keySpots(span(world.keyQuad[3], world.keyQuad[2])).word, 0.6), angle: edge(world.keyQuad), size: 12 });
  return labels;
}

/** The room each facet's button has: the box inside its quad, so neighbours never overlap, down to the foot of its lip so its label is part of the press. The key on a lip comes last. */
export function layoutOf(width: number, plan: Plan): Layout {
  const world = worldOf(width, plan);
  const inner = world.quads.map((q, i): Rect => {
    const box = boxOf(q);
    const lip = world.lips[i]!;
    return i === plan.keyed ? box : { ...box, h: Math.max(lip[2].y, lip[3].y) - box.y };
  });
  if (world.keyQuad !== undefined) inner.push(boxOf(world.keyQuad));
  const first = world.rims[0]!;
  const last = world.rims.at(-1)!;
  return { width, height: world.height, inner, labels: labelsOf(world, plan), rim: { x: first[0].x, y: Math.min(first[0].y, last[1].y), w: last[1].x - first[0].x, h: first[3].y - first[0].y } };
}

// ---- The stage ---------------------------------------------------------------------------------------------

type Facet = {
  /** The texture's unscaled root, holding `art`, which is scaled from the art's units to the texture's texels. */
  root: Container;
  art: Container;
  plate: Graphics;
  /** The screen's moving layer. A lit-screen effect would attach here, once, as a filter on this layer. */
  live: Graphics;
  /** The surface's true proportions, which keep a circle on it a circle before the projection. */
  aspect: number;
  paint: Paint;
  tex: RenderTexture;
  mesh: PerspectiveMesh;
  drawn: string;
};

const EMPTY = (): BayState => ({ kind: null, pressed: false, calling: false, lit: false, focused: false });

export class ConsoleStage {
  private app: Application | undefined;
  private gone = false;
  private hidden = false;
  private width = 0;
  private world: World | undefined;
  private facets: Facet[] = [];
  private shell = new Graphics();
  private over = new Graphics();
  private ribs = new Graphics();
  private tone: Tones | undefined;
  private mat: Materials | undefined;
  private bays: BayState[] = [];
  private plan: Plan = { widths: [1, 1, 1, 1, 1], depths: [1, 1, 1, 1, 1], keyed: undefined };
  private source: () => BayState[] = () => [];
  private still = false;
  private watch: MediaQueryList | undefined;
  private density: MediaQueryList | undefined;
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
    await app.init({ preference: "webgl", antialias: true, backgroundAlpha: 0, autoDensity: true, resolution: densityNow(), width: Math.max(1, this.width), height: 120, autoStart: false, sharedTicker: false, powerPreference: "low-power" });
    Ticker.system.stop();
    if (this.gone) return void app.destroy(true, { children: true });
    this.app = app;
    app.stage.eventMode = "none";
    app.canvas.setAttribute("aria-hidden", "true");
    this.host.append(app.canvas);
    this.setTone();
    this.build();
    app.ticker.maxFPS = 30;
    app.ticker.add(this.tick);
    this.watch = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.still = this.watch.matches;
    this.watch.addEventListener("change", this.motion);
    this.themed = new MutationObserver(() => {
      this.setTone();
      this.rebuild();
    });
    this.themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-still", "data-idle"] });
    document.addEventListener("visibilitychange", this.settle);
    this.watchDensity();
    this.resize(this.width);
  }

  private setTone(): void {
    this.tone = readTones(this.host);
    this.mat = materialsOf(this.tone);
  }

  /** A window dragged to another display changes the density with no change of size, and the textures follow it. */
  private watchDensity(): void {
    this.density?.removeEventListener("change", this.redensify);
    this.density = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
    this.density.addEventListener("change", this.redensify);
  }
  private readonly redensify = (): void => {
    this.watchDensity();
    this.resize(this.width);
  };

  /** One facet for each width, and one more for the key on a lip. */
  private build(): void {
    const { app } = this;
    if (app === undefined) return;
    this.world = worldOf(Math.max(1, this.width), this.plan);
    const shapes = this.plan.widths.map((w, i) => (FW * w) / (FH * this.plan.depths[i]!));
    if (this.plan.keyed !== undefined) shapes.push(this.world.keyAspect);
    for (const aspect of shapes) {
      const root = new Container();
      const art = new Container();
      const plate = new Graphics();
      const live = new Graphics();
      art.addChild(plate, live);
      root.addChild(art);
      const tex = RenderTexture.create({ width: 2, height: 2, resolution: 1, antialias: true });
      const mesh = new PerspectiveMesh({ texture: tex, verticesX: 20, verticesY: 20 });
      this.facets.push({ root, art, plate, live, aspect, paint: { W: 1, H: 1, f: 1, lipKey: false, named: false }, tex, mesh, drawn: "" });
    }
    const keyed = this.plan.keyed === undefined ? [] : [this.facets.at(-1)!.mesh];
    const flat = this.plan.keyed === undefined ? this.facets : this.facets.slice(0, -1);
    app.stage.addChild(this.shell, ...flat.map((f) => f.mesh), this.over, ...keyed, this.ribs);
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
      this.app.renderer.resize(this.width, layout.height, densityNow());
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

  /** Each facet's texture takes the size it lands at on screen, in device pixels. */
  private fitFacets(): void {
    const { world, app } = this;
    if (world === undefined || app === undefined) return;
    this.facets.forEach((f, i) => {
      const q = i < world.quads.length ? world.quads[i]! : world.keyQuad;
      if (q === undefined) return;
      f.mesh.setCorners(q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y);
      const across = Math.max(span(q[0], q[1]), span(q[3], q[2]));
      const down = Math.max(span(q[0], q[3]), span(q[1], q[2]));
      const W = across;
      const H = W / f.aspect;
      const lipKey = i === this.plan.widths.length;
      f.paint = { W, H, f: down / H, lipKey, named: i === this.plan.keyed };
      const texels = app.renderer.resolution * TEXELS;
      const [tw, th] = [Math.ceil(across * texels), Math.ceil(down * texels)];
      if (f.tex.width !== tw || f.tex.height !== th) {
        f.tex.destroy(true);
        f.tex = RenderTexture.create({ width: tw, height: th, resolution: 1, antialias: true });
        f.mesh.texture = f.tex;
        f.mesh.setCorners(q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y);
      }
      f.art.scale.set(tw / W, th / H);
      f.drawn = "";
    });
  }

  /** The shell: hood, lips, front wall, end cheeks and ribs, each a stack of hard bands. */
  private rebuild(): void {
    const { world, mat } = this;
    if (world === undefined || mat === undefined) return;
    const fill = (g: Graphics, q: Quad, colour: number, alpha = 1) => g.poly(q.flatMap((p) => [p.x, p.y])).fill({ color: colour, alpha });
    /** A quad cut across into bands, each to the fraction it ends at. */
    const bands = (g: Graphics, q: Quad, list: readonly (readonly [number, number])[]) => {
      let from = 0;
      for (const [to, colour] of list) {
        fill(g, partOf(q, 0, from, 1, to), colour);
        from = to;
      }
    };
    const bolt = (g: Graphics, q: Quad, x: number, y: number, r: number) => {
      const p = inQuad(q, x, y);
      g.circle(p.x, p.y + 0.6, r + 0.7).fill(mat.gap);
      g.circle(p.x, p.y, r).fill(mat.cap);
      g.rect(p.x - r * 0.65, p.y - 0.4, r * 1.3, 0.8).fill(mat.gap);
    };
    const { shell, over, ribs } = this;
    shell.clear();
    over.clear();
    ribs.clear();
    const big = this.width >= 900;
    world.ends.forEach((q) => {
      fill(shell, q, mat.hullShade);
      fill(shell, [q[0], q[1], inQuad(q, 1, 0.14), inQuad(q, 0, 0.14)], mat.hood);
    });
    world.floor.forEach((q) => {
      bands(shell, q, [[0.28, mat.gap], [0.8, mat.wall], [0.9, mat.kick], [1, mat.gap]]);
      const count = Math.max(4, Math.round(span(q[0], q[1]) / 11));
      const from = count * 0.3;
      for (let k = Math.round(from); k < count - from; k += 1) fill(shell, partOf(q, (k + 0.2) / count, 0.42, (k + 0.65) / count, 0.68), mat.gap);
    });
    world.rims.forEach((q) => {
      bands(shell, q, [[0.14, mat.glint], [0.44, mat.cap], [0.84, mat.hood], [1, mat.hullShade]]);
      for (const x of [0.34, 0.66]) {
        fill(shell, partOf(q, x, 0, x + 0.006, 0.84), mat.gap);
        fill(shell, partOf(q, x + 0.006, 0, x + 0.012, 0.84), mat.glint);
      }
      if (big) for (const x of [0.04, 0.96]) bolt(shell, q, x, 0.62, 1.7);
    });
    world.shade.forEach((q) => fill(over, q, mat.gap, 0.42));
    world.lips.forEach((q, i) => {
      if (i === this.plan.keyed) return;
      bands(over, q, [[0.08, mat.glint], [0.2, mat.cap], [0.9, mat.fascia], [1, mat.hullShade]]);
      if (span(q[0], q[3]) >= 14) for (const x of [0.035, 0.965]) bolt(over, q, x, 0.56, big ? 2 : 1.5);
    });
    const faces: Record<string, readonly [number, number, number]> = {
      rim: [mat.glint, mat.cap, mat.hood],
      deck: [mat.cap, mat.hood, mat.hullShade],
      lip: [mat.cap, mat.hood, mat.hullShade],
      wall: [mat.kick, mat.wall, mat.gap],
    };
    world.ribs.forEach(({ face, column, quad }) => fill(ribs, quad, faces[face]![column]!));
    this.fitFacets();
    this.frame();
  }

  private readonly tick = (): void => this.frame();

  private frame(): void {
    const { app, tone, mat } = this;
    if (app === undefined || tone === undefined || mat === undefined || this.world === undefined) return;
    const t = this.still ? 0 : performance.now() / 1000;
    this.bays = this.source();
    this.facets.forEach((f, i) => {
      const state = this.bays[i] ?? EMPTY();
      const key = `${state.kind}|${state.pressed}|${state.lit}|${state.focused}`;
      if (f.drawn !== key) {
        drawPlate(f.plate, f.paint, state, tone, mat);
        f.drawn = key;
      }
      drawLive(f.live, f.paint, state, tone, mat, t);
      app.renderer.render({ container: f.root, target: f.tex, clear: true, clearColor: [0, 0, 0, 0] });
    });
    app.renderer.render(app.stage);
  }

  destroy(): void {
    this.gone = true;
    this.themed?.disconnect();
    this.watch?.removeEventListener("change", this.motion);
    this.density?.removeEventListener("change", this.redensify);
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

const densityNow = () => Math.min(window.devicePixelRatio || 1, 2);
