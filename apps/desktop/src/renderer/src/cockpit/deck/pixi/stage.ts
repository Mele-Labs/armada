// The canvas the map is drawn on, and when it draws. **A frame is drawn when something changed and not otherwise**: a new layout, the
// camera moved, a plot's height easing. There is no loop. A change asks for one frame on the next animation frame; a change that eases
// (the camera to a lock-on, a plot rising, a landed ring fading) starts the ticker, which runs for as long as something eases and
// then stops itself. A map nobody is touching costs nothing.
//
// The scene graph, back to front: the ground, the plots, the air, what lies on the ground under the drones (a Job's pad among it), the files' bars and their
// glow (over the drones' shadows, so a file a drone is on still shows), the lines they leave, the standing pictures, the drones, the effects.
// The ground and the plots are built again only when the tree or what stands on a plot changes, never for a file, a glow or a drone.
// Continuous motion belongs to the few drones the map animates at once and to the plays an event started; the ticker runs while either
// does, at half rate where the camera is still, and stops when neither does. A folded pane, a hidden window and an idle one draw nothing.

// A page whose policy has no `unsafe-eval` cannot use Pixi's generated uniform code; this swaps it for the same code written out.
import "pixi.js/unsafe-eval";
import { Application, Container, Ticker } from "pixi.js";

import type { Cam } from "../camera";
import type { Size } from "../layout";
import { drawAir } from "./air";
import { over, type Clock } from "./clock";
import { Beams, drawDoings } from "./doings";
import { Fliers } from "./fliers";
import { Workers } from "./workers";
import { drawGround } from "./ground";
import { drawLifted } from "./lifted";
import { drawPlays } from "./motion";
import { PlaneBuilder, PlaneLayer, PlaneView } from "./plane";
import { readPalette, type Palette } from "./palette";
import { alongOf } from "../layout";
import { easeOut, lerp, project, viewOf, type View } from "./project";
import type { PixiSceneIn } from "./scene";
import { same } from "./same";
import { standsOf, type StandSpec } from "./stands";
import { drawTerrain } from "./terrain";
import { drawTiles, tilesIn, type TilesIn } from "./tiles";
import { drawMarks } from "./marks";
import { drawBlame, drawLinks, drawPad, padMoving, thinkMoving } from "./off-floor";

type Stood = { spec: StandSpec; node: Container };
type Tween<T> = { from: T; to: T; start: number; ms: number };

const sameCam = (a: Cam, b: Cam) => a.x === b.x && a.y === b.y && a.eye === b.eye && a.tilt === b.tilt;

export class Stage {
  /** The camera as drawn this frame. The drones group and the hit layer read it to place what they add. */
  shown: View = { x: 0, y: 0, eye: 1, pitch: 0, width: 0, height: 0 };
  readonly stands = new Container();
  readonly drones = new Container();
  readonly effects = new Container();
  private app: Application | undefined;
  private gone = false;
  private readonly host: HTMLElement;
  private readonly planeView = new PlaneView();
  private terrain: PlaneLayer | undefined;
  private tiles: PlaneLayer | undefined;
  private marks: PlaneLayer | undefined;
  private marksGlow: PlaneLayer | undefined;
  /** What the plots and the bars were last drawn from, so a scene that changes neither leaves them as they are. */
  private tilesDrawn: { of: TilesIn; heights: ReadonlyMap<string, number> } | undefined;
  private marksDrawn: { of: unknown; heights: ReadonlyMap<string, number> } | undefined;
  private air: PlaneLayer | undefined;
  private ground: PlaneLayer | undefined;
  private groundGlow: PlaneLayer | undefined;
  private lines: PlaneLayer | undefined;
  private linesGlow: PlaneLayer | undefined;
  private fliers: Fliers | undefined;
  private workers: Workers | undefined;
  private beams: Beams | undefined;
  private readonly playStart = new Map<string, number>();
  private palette: Palette | undefined;
  private frame = 0;
  private quiet = false;
  /** The pane is folded away or has no size: nothing is drawn or built for it until it is shown again. */
  private hidden = false;
  /** A scene was handed over while nothing was drawn: it is taken in when the stage is seen again. */
  private pending = false;
  /** The palette or the fonts changed while folded: everything is drawn again from them when the stage is seen again. */
  private pendingForce = false;
  private size: Size = { width: 0, height: 0 };
  private cam: Cam | undefined;
  private scene: PixiSceneIn | undefined;
  private drawn: PixiSceneIn | undefined;
  private camTween: Tween<View> | undefined;
  private riseTween: Tween<ReadonlyMap<string, number>> | undefined;
  private heights = new Map<string, number>();
  /** The heights as drawn: where the plots stand, and the lift a play gives some of them for a moment. */
  private risen: ReadonlyMap<string, number> = new Map();
  private landed = new Map<string, number>();
  private stood = new Map<string, Stood>();
  private readonly listeners = new Set<(view: View) => void>();
  private fonts = 0;
  private themed: MutationObserver | undefined;
  private still: MediaQueryList | undefined;

  private constructor(host: HTMLElement) {
    this.host = host;
  }

  /** Starts the canvas in `host`. It is ready a moment later; whatever it is told before then is kept and drawn once it is. */
  static start(host: HTMLElement): Stage {
    const stage = new Stage(host);
    stage.init().catch((error: unknown) => console.error("the map's canvas did not start", error));
    return stage;
  }

  private async init(): Promise<void> {
    const app = new Application();
    await app.init({
      preference: "webgl",
      antialias: true,
      backgroundAlpha: 0,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
      width: Math.max(1, this.size.width),
      height: Math.max(1, this.size.height),
      // No loop of its own: this class asks for each frame, and starts the ticker only while something eases.
      autoStart: false,
      sharedTicker: false,
      powerPreference: "low-power",
      eventFeatures: { move: false, globalMove: false, click: false, wheel: false },
    });
    // Starting a renderer starts Pixi's system ticker, which then asks for an animation frame every frame for as long as one lives, and an
    // idle map must ask for none. It only sweeps unused textures, which a handful of name labels do not need. A stage that was dropped while
    // it started starts it too, so this runs on that path as well.
    Ticker.system.stop();
    if (this.gone) {
      app.destroy(true, { children: true });
      return;
    }
    this.app = app;
    // The pane was measured while this was starting.
    if (this.size.width > 0) app.renderer.resize(this.size.width, this.size.height);
    app.stage.eventMode = "none";
    app.stage.interactiveChildren = false;
    app.canvas.setAttribute("aria-hidden", "true");
    this.host.append(app.canvas);
    this.palette = readPalette(this.host);
    this.terrain = new PlaneLayer(this.planeView);
    this.tiles = new PlaneLayer(this.planeView);
    this.marks = new PlaneLayer(this.planeView);
    this.marksGlow = new PlaneLayer(this.planeView, true);
    this.air = new PlaneLayer(this.planeView);
    this.ground = new PlaneLayer(this.planeView);
    this.groundGlow = new PlaneLayer(this.planeView, true);
    this.lines = new PlaneLayer(this.planeView);
    this.linesGlow = new PlaneLayer(this.planeView, true);
    this.fliers = new Fliers(this.drones);
    this.workers = new Workers(this.drones);
    this.beams = new Beams(this.effects);
    this.stands.sortableChildren = true;
    app.stage.addChild(this.terrain.mesh, this.tiles.mesh, this.air.mesh, this.ground.mesh, this.groundGlow.mesh, this.marks.mesh, this.marksGlow.mesh, this.lines.mesh, this.linesGlow.mesh, this.stands, this.drones, this.effects);
    app.ticker.add(this.tick);
    // A name drawn before its font arrived is drawn again once it has.
    void document.fonts.ready.then(() => this.restyle());
    // A theme put on the page, a mod's edited, or reduced motion asked for changes the tokens under every colour here. The loader adopts
    // its sheet before it sets `data-theme`, so the tokens are already the new ones when this runs.
    // `data-still` and `data-idle` (`still.ts`) are a window nobody is looking at: nothing is drawn and the ticker stops until they are cleared.
    this.themed = new MutationObserver((records) => {
      if (records.some((one) => one.attributeName === "data-theme")) this.restyle();
      this.settle();
    });
    this.themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-still", "data-idle"] });
    this.settle();
    this.still = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.still.addEventListener("change", this.restyle);
    this.resync(true);
  }

  /** The palette read again from the page as it is now, and everything drawn from it drawn again. The pictures that keep what they were drawn with are told by `fonts`. */
  private readonly restyle = (): void => {
    if (this.gone || this.app === undefined) return;
    this.fonts += 1;
    this.palette = readPalette(this.host);
    this.resync(true);
  };

  /** Takes in the scene now, or, folded, once the pane is shown again: fonts arriving or a theme put on build nothing for a folded pane. */
  private resync(force: boolean): void {
    if (!this.hidden) {
      this.sync(force);
      return;
    }
    this.pending = true;
    this.pendingForce ||= force;
  }

  /** Whether the window is hidden, unfocused or idle, or the pane folded: the ticker stops with the pose held, and starts again when it is not. */
  private settle(): void {
    const root = document.documentElement;
    const quiet = this.hidden || root.hasAttribute("data-still") || root.hasAttribute("data-idle");
    if (quiet === this.quiet) return;
    this.quiet = quiet;
    if (quiet) {
      cancelAnimationFrame(this.frame);
      this.frame = 0;
      this.app?.ticker.stop();
    } else if (this.hidden === false && this.pending) {
      const force = this.pendingForce;
      this.pending = false;
      this.pendingForce = false;
      this.sync(force);
    } else this.wake();
  }

  /** The pane folded away, or shown again. Folded, a new scene is kept and built only once it is shown. */
  setHidden(hidden: boolean): void {
    if (hidden === this.hidden) return;
    this.hidden = hidden;
    this.settle();
  }

  // ---- What the page tells it -------------------------------------------------------------------------------

  setSize(size: Size): void {
    if (size.width === this.size.width && size.height === this.size.height) return;
    this.size = size;
    this.app?.renderer.resize(size.width, size.height);
    // A pane that changes size moves nothing to ease: the camera is drawn at once for the new one.
    if (this.cam !== undefined) this.setCamera(this.cam, true, true);
  }

  /** `live` is a hand on the map: the camera follows it at once. Anything else eases, over the time the CSS camera took. */
  setCamera(cam: Cam, live: boolean, force = false): void {
    if (!force && this.cam !== undefined && sameCam(this.cam, cam)) return;
    this.cam = cam;
    if (this.size.width === 0) return;
    const to = viewOf(cam, this.size);
    const ms = this.palette?.camMs ?? 0;
    if (live || ms <= 0 || this.shown.width === 0) {
      this.camTween = undefined;
      this.show(to);
    } else {
      this.camTween = { from: { ...this.shown, width: to.width, height: to.height }, to, start: performance.now(), ms };
      this.wake();
    }
  }

  setScene(scene: PixiSceneIn): void {
    if (this.scene !== undefined && same(this.scene, scene)) return;
    this.scene = scene;
    // A folded pane builds nothing; under `data-still` or `data-idle` the scene is still taken in, as it was, so a frame drawn later is current.
    if (this.hidden) {
      this.pending = true;
      return;
    }
    this.sync(false);
  }

  /** Where a point of the scene is on the canvas now. */
  project(at: Parameters<typeof project>[1]): ReturnType<typeof project> {
    return project(this.shown, at);
  }

  /** Called with the view each time it changes, for what is placed over the canvas from script. */
  onView(listener: (view: View) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  destroy(): void {
    this.gone = true;
    cancelAnimationFrame(this.frame);
    this.listeners.clear();
    this.themed?.disconnect();
    this.still?.removeEventListener("change", this.restyle);
    const app = this.app;
    this.app = undefined;
    if (app === undefined) return;
    app.ticker.remove(this.tick);
    app.ticker.stop();
    this.fliers?.destroy();
    this.workers?.destroy();
    this.beams?.destroy();
    this.terrain?.destroy();
    this.tiles?.destroy();
    this.marks?.destroy();
    this.marksGlow?.destroy();
    this.air?.destroy();
    this.ground?.destroy();
    this.groundGlow?.destroy();
    this.lines?.destroy();
    this.linesGlow?.destroy();
    app.destroy({ removeView: true }, { children: true });
  }

  // ---- Drawing ---------------------------------------------------------------------------------------------

  /** Takes in a new scene: rebuilds what its change reaches, and starts what eases. */
  private sync(force: boolean): void {
    const { app, palette, scene } = this;
    if (app === undefined || palette === undefined || scene === undefined) return;
    const was = this.drawn;
    this.drawn = scene;
    // The floor is the repository's whole tree: it is drawn again for a new tree or ground, never because a drone, a file or a glow moved.
    const ground = force || was === undefined || !same(was.layout.territories, scene.layout.territories) || was.pads !== scene.pads || was.goals !== scene.goals || was.hotPart !== scene.hotPart || was.world !== scene.world;
    if (force) {
      this.tilesDrawn = undefined;
      this.marksDrawn = undefined;
    }
    if (ground) this.build(this.terrain!, (b) => drawTerrain(b, scene, palette));
    this.restand(scene, palette);
    this.noticePlays(scene);
    this.fliers?.sync(scene, palette, this.fonts);
    this.workers?.sync(scene, palette, this.fonts);
    if (force || was === undefined || was.crates !== scene.crates) this.build(this.air!, (b) => drawAir(b, scene, palette));
    this.noticeLandings(scene, was === undefined);
    const toward = scene.heights;
    const first = was === undefined || this.heights.size === 0;
    const moving = [...toward].some(([path, height]) => (this.heights.get(path) ?? height) !== height);
    if (first || palette.riseMs <= 0) {
      this.heights = new Map(toward);
      this.riseTween = undefined;
    } else if (moving) this.riseTween = { from: new Map(this.heights), to: toward, start: performance.now(), ms: palette.riseMs };
    this.rebuildTiles();
    this.rebuildFx();
    if (this.cam !== undefined && this.shown.width === 0) this.setCamera(this.cam, true, true);
    this.placeStands();
    this.placeFliers();
    this.wake();
  }

  /** A play is dated by the first time the canvas sees it, and forgotten once the log no longer lists it. */
  private noticePlays(scene: PixiSceneIn): void {
    const now = performance.now();
    const live = new Set<string>();
    for (const play of scene.plays) {
      live.add(play.id);
      if (!this.playStart.has(play.id)) this.playStart.set(play.id, now);
    }
    for (const id of this.playStart.keys()) if (!live.has(id)) this.playStart.delete(id);
  }

  private clock(): Clock {
    const now = performance.now();
    return { now, pulse: this.palette?.pulseMs ?? 0, since: (id, delay) => now - (this.playStart.get(id) ?? now) - delay };
  }

  /** The height a plot stands at as it is drawn now, which is not where it is going while it eases. */
  readonly floorOf = (tile: string | undefined): number => (tile === undefined ? 0 : (this.risen.get(tile) ?? this.drawn?.heights.get(tile) ?? 0));

  /** A plot a play touches rises and settles: up over the first 38% of 0.9 of a pulse, and down over the rest. */
  private liftOf(path: string, clock: Clock): number {
    const bump = this.scene?.bumps.get(path);
    const run = clock.pulse * 0.9;
    if (bump === undefined || run <= 0) return 0;
    const t = clock.since(bump.id, bump.wait) / run;
    if (t <= 0 || t >= 1) return 0;
    const u = t < 0.38 ? t / 0.38 : 1 - (t - 0.38) / 0.62;
    return bump.amp * u * u * (3 - 2 * u);
  }

  private get lifting(): boolean {
    const { scene } = this;
    if (scene === undefined || scene.bumps.size === 0) return false;
    const clock = this.clock();
    return [...scene.bumps.keys()].some((path) => this.liftOf(path, clock) !== 0);
  }

  /** What lies on the ground under the drones, and the lines they leave: drawn again whenever the scene, a plot's height or a loop moves it. */
  private rebuildFx(): void {
    const { scene, palette } = this;
    if (scene === undefined || palette === undefined || this.ground === undefined) return;
    const clock = this.clock();
    const [shade, glow, lines, lit] = [new PlaneBuilder(), new PlaneBuilder(), new PlaneBuilder(), new PlaneBuilder()];
    drawPad(shade, glow, scene, palette, clock);
    drawGround(shade, glow, scene, palette, this.floorOf, clock);
    drawDoings(glow, scene, palette, this.floorOf, clock);
    drawPlays(glow, scene, palette, this.floorOf, clock);
    drawLifted(lines, lit, scene, palette, this.floorOf, clock);
    drawLinks(lines, scene, palette, clock);
    this.ground.set(shade);
    this.groundGlow!.set(glow);
    this.lines!.set(lines);
    this.linesGlow!.set(lit);
  }

  /** Everything that stands upright over the ground and follows the camera: the drones, and the lines between them and what they work on. */
  private placeFliers(): void {
    const clock = this.clock();
    this.fliers?.place(this.shown, this.floorOf, clock);
    this.workers?.place(this.shown, this.floorOf);
    if (this.scene !== undefined && this.palette !== undefined) this.beams?.update(this.scene, this.palette, this.shown, this.floorOf, clock);
  }

  private build(layer: PlaneLayer, draw: (b: PlaneBuilder) => void): void {
    const b = new PlaneBuilder();
    draw(b);
    layer.set(b);
  }

  private rebuildTiles(): void {
    const { scene, palette } = this;
    if (scene === undefined || palette === undefined || this.tiles === undefined) return;
    const clock = this.clock();
    this.risen = scene.bumps.size === 0 ? this.heights : new Map([...this.heights].map(([path, height]) => [path, height + this.liftOf(path, clock)] as const));
    const of = tilesIn(scene);
    const fading = this.fading;
    const kept = this.tilesDrawn;
    if (fading || kept === undefined || !same(kept.heights, this.risen) || !same(kept.of, of)) {
      this.build(this.tiles, (b) => drawTiles(b, of, palette, this.risen, (key) => this.ringAlpha(key)));
      this.tilesDrawn = { of, heights: this.risen };
    }
    this.rebuildMarks();
  }

  /** The bars, their glow, the touched plots and the route: drawn again when one of them or a plot's height changes. */
  private rebuildMarks(): void {
    const { scene, palette } = this;
    if (scene === undefined || palette === undefined || this.marks === undefined) return;
    const of = [scene.layout.bars, scene.glow, scene.touched, scene.route, scene.pad?.blame];
    const kept = this.marksDrawn;
    if (kept !== undefined && same(kept.heights, this.risen) && same(kept.of, of)) return;
    const [b, glow] = [new PlaneBuilder(), new PlaneBuilder()];
    drawMarks(b, glow, scene, palette, this.risen);
    drawBlame(b, glow, scene, palette, this.risen);
    this.marks.set(b);
    this.marksGlow!.set(glow);
    this.marksDrawn = { of, heights: this.risen };
  }

  /** A ring that has landed stays a moment, then fades; one already landed when it was first seen is not drawn. */
  private noticeLandings(scene: PixiSceneIn, first: boolean): void {
    const live = new Set<string>();
    for (const { build } of scene.builds) {
      if (build.landedAt === undefined) continue;
      live.add(build.key);
      if (!this.landed.has(build.key)) this.landed.set(build.key, first ? Number.NEGATIVE_INFINITY : performance.now());
    }
    for (const key of this.landed.keys()) if (!live.has(key)) this.landed.delete(key);
  }

  private ringAlpha(key: string): number {
    const born = this.landed.get(key);
    const pulse = this.palette?.pulseMs ?? 0;
    if (born === undefined) return 1;
    if (pulse <= 0) return 0;
    const over = performance.now() - born - pulse * 1.5;
    return over <= 0 ? 1 : Math.max(0, 1 - over / (pulse * 1.2));
  }

  /** The standing pictures: one kept for as long as its key and what it shows are the same, drawn again when either changes. */
  private restand(scene: PixiSceneIn, palette: Palette): void {
    const next = new Map<string, Stood>();
    for (const spec of standsOf(scene)) {
      const kept = this.stood.get(spec.key);
      spec.sig = `${spec.sig}:${this.fonts}`;
      if (kept !== undefined && kept.spec.sig === spec.sig) {
        kept.spec = spec;
        next.set(spec.key, kept);
        continue;
      }
      kept?.node.destroy({ children: true });
      const node = spec.make(palette);
      node.eventMode = "none";
      this.stands.addChild(node);
      next.set(spec.key, { spec, node });
    }
    for (const [key, one] of this.stood) if (!next.has(key)) one.node.destroy({ children: true });
    this.stood = next;
  }

  private placeStands(): void {
    if (this.shown.width === 0) return;
    const focus = Math.min(1.7, Math.max(1, 0.72 / this.shown.eye));
    const clock = this.clock();
    for (const { spec, node } of this.stood.values()) {
      // A name lying on a plot rides it as it rises.
      let where = spec.onTile === undefined ? spec.at : { ...spec.at, z: this.floorOf(spec.onTile) };
      node.alpha = 1;
      const hop = spec.hop;
      if (hop !== undefined && clock.pulse > 0) {
        const since = clock.since(hop.id, hop.delay);
        const t = Math.min(1, Math.max(0, since / (clock.pulse * (hop.landing ? 0.7 : 1.5))));
        // From where it was, along an arc that rises 34, to its place; a pull request that has landed goes the moment it arrives.
        where = alongOf(hop.from, spec.at, 34, t);
        if (hop.landing) node.alpha = 1 - Math.min(1, Math.max(0, (since - clock.pulse * 0.7) / 160));
      }
      const at = project(this.shown, where);
      node.visible = at.scale > 0 && node.alpha > 0;
      node.position.set(at.x, at.y);
      node.scale.set(this.shown.eye * at.scale * (spec.counter === true ? focus : 1));
      node.zIndex = spec.at.y;
    }
  }

  private show(view: View): void {
    this.shown = view;
    this.planeView.set(view);
    this.placeStands();
    this.placeFliers();
    for (const listener of this.listeners) listener(view);
    this.wake();
  }

  // ---- When ----------------------------------------------------------------------------------------------

  private get fading(): boolean {
    const pulse = this.palette?.pulseMs ?? 0;
    const now = performance.now();
    return pulse > 0 && [...this.landed.values()].some((born) => now - born < pulse * 2.7);
  }

  /** A drone's loop is running, or a play an event started has not finished. */
  private get moving(): boolean {
    const { scene } = this;
    const pulse = this.palette?.pulseMs ?? 0;
    if (scene === undefined || pulse <= 0) return false;
    const now = performance.now();
    const clock = this.clock();
    return scene.drones.some((one) => one.loop && one.aim !== undefined) || padMoving(scene, clock) || thinkMoving(scene, clock) || scene.plays.some((one) => !over(one.kind, one.delay, pulse, now - (this.playStart.get(one.id) ?? now)));
  }

  private get easing(): boolean {
    return this.camTween !== undefined || this.riseTween !== undefined || this.fading || this.moving;
  }

  /** Something changed: draw it. On the ticker if it will keep changing, otherwise once on the next animation frame. */
  private wake(): void {
    const app = this.app;
    if (app === undefined || this.quiet) return;
    if (this.easing) {
      cancelAnimationFrame(this.frame);
      this.frame = 0;
      // A loop or a play is smooth at thirty frames a second; only a hand or an easing camera needs more.
      app.ticker.maxFPS = this.camTween !== undefined || this.riseTween !== undefined ? 0 : 30;
      if (!app.ticker.started) app.ticker.start();
      return;
    }
    if (this.frame !== 0 || app.ticker.started) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.app?.render();
    });
  }

  /** One step of whatever eases. The application renders after this on the same tick; when nothing is left the ticker stops. */
  private readonly tick = (): void => {
    const now = performance.now();
    const cam = this.camTween;
    if (cam !== undefined) {
      const t = Math.min(1, (now - cam.start) / cam.ms);
      const k = easeOut(t);
      this.show({ x: lerp(cam.from.x, cam.to.x, k), y: lerp(cam.from.y, cam.to.y, k), eye: lerp(cam.from.eye, cam.to.eye, k), pitch: lerp(cam.from.pitch, cam.to.pitch, k), width: cam.to.width, height: cam.to.height });
      if (t >= 1) this.camTween = undefined;
    }
    const rise = this.riseTween;
    if (rise !== undefined) {
      const t = Math.min(1, (now - rise.start) / rise.ms);
      const k = easeOut(t);
      this.heights = new Map([...rise.to].map(([path, to]) => [path, lerp(rise.from.get(path) ?? to, to, k)] as const));
      if (t >= 1) this.riseTween = undefined;
    }
    if (rise !== undefined || this.fading || this.lifting) this.rebuildTiles();
    this.rebuildFx();
    this.placeStands();
    this.placeFliers();
    if (this.app !== undefined) this.app.ticker.maxFPS = cam !== undefined || rise !== undefined ? 0 : 30;
    if (!this.easing) this.app?.ticker.stop();
  };
}
