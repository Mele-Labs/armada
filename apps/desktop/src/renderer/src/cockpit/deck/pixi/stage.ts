// The canvas the map is drawn on, and when it draws. **A frame is drawn when something changed and not otherwise**: a new layout, the
// camera moved, a plot's height easing. There is no loop. A change asks for one frame on the next animation frame; a change that eases
// (the camera to a lock-on, a plot rising, a landed ring fading) starts the ticker, which runs for as long as something eases and
// then stops itself. A map nobody is touching costs nothing.
//
// The scene graph, back to front: the ground, the plots, the air, the standing pictures, the drones, the effects. The last two are
// empty here; the group that draws drones and effects puts them in.

// A page whose policy has no `unsafe-eval` cannot use Pixi's generated uniform code; this swaps it for the same code written out.
import "pixi.js/unsafe-eval";
import { Application, Container } from "pixi.js";

import type { Cam } from "../camera";
import type { Size } from "../layout";
import { drawAir } from "./air";
import { PlaneBuilder, PlaneLayer, PlaneView } from "./plane";
import { readPalette, type Palette } from "./palette";
import { easeOut, lerp, project, viewOf, type View } from "./project";
import type { PixiSceneIn } from "./scene";
import { standsOf, type StandSpec } from "./stands";
import { drawTerrain } from "./terrain";
import { drawTiles } from "./tiles";

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
  private air: PlaneLayer | undefined;
  private palette: Palette | undefined;
  private frame = 0;
  private size: Size = { width: 0, height: 0 };
  private cam: Cam | undefined;
  private scene: PixiSceneIn | undefined;
  private drawn: PixiSceneIn | undefined;
  private camTween: Tween<View> | undefined;
  private riseTween: Tween<ReadonlyMap<string, number>> | undefined;
  private heights = new Map<string, number>();
  private landed = new Map<string, number>();
  private stood = new Map<string, Stood>();
  private readonly listeners = new Set<(view: View) => void>();
  private fonts = 0;

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
    this.air = new PlaneLayer(this.planeView);
    this.stands.sortableChildren = true;
    app.stage.addChild(this.terrain.mesh, this.tiles.mesh, this.air.mesh, this.stands, this.drones, this.effects);
    app.ticker.add(this.tick);
    // A name drawn before its font arrived is drawn again once it has.
    void document.fonts.ready.then(() => {
      if (this.gone) return;
      this.fonts += 1;
      this.palette = readPalette(this.host);
      this.sync(true);
    });
    this.sync(true);
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
    this.scene = scene;
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
    const app = this.app;
    this.app = undefined;
    if (app === undefined) return;
    app.ticker.remove(this.tick);
    app.ticker.stop();
    this.terrain?.destroy();
    this.tiles?.destroy();
    this.air?.destroy();
    app.destroy({ removeView: true }, { children: true });
  }

  // ---- Drawing ---------------------------------------------------------------------------------------------

  /** Takes in a new scene: rebuilds what its change reaches, and starts what eases. */
  private sync(force: boolean): void {
    const { app, palette, scene } = this;
    if (app === undefined || palette === undefined || scene === undefined) return;
    const was = this.drawn;
    this.drawn = scene;
    const ground = force || was === undefined || was.layout !== scene.layout || was.pads !== scene.pads || was.goals !== scene.goals || was.route !== scene.route || was.pinned !== scene.pinned || was.world !== scene.world;
    if (ground) this.build(this.terrain!, (b) => drawTerrain(b, scene, palette));
    this.restand(scene, palette);
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
    if (this.cam !== undefined && this.shown.width === 0) this.setCamera(this.cam, true, true);
    this.placeStands();
    this.wake();
  }

  private build(layer: PlaneLayer, draw: (b: PlaneBuilder) => void): void {
    const b = new PlaneBuilder();
    draw(b);
    layer.set(b);
  }

  private rebuildTiles(): void {
    const { scene, palette } = this;
    if (scene === undefined || palette === undefined || this.tiles === undefined) return;
    this.build(this.tiles, (b) => drawTiles(b, scene, palette, this.heights, (key) => this.ringAlpha(key)));
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
    for (const { spec, node } of this.stood.values()) {
      const at = project(this.shown, spec.at);
      node.visible = at.scale > 0;
      node.position.set(at.x, at.y);
      node.scale.set(this.shown.eye * at.scale * (spec.counter === true ? focus : 1));
      node.zIndex = spec.at.y;
    }
  }

  private show(view: View): void {
    this.shown = view;
    this.planeView.set(view);
    this.placeStands();
    for (const listener of this.listeners) listener(view);
    this.wake();
  }

  // ---- When ----------------------------------------------------------------------------------------------

  private get fading(): boolean {
    const pulse = this.palette?.pulseMs ?? 0;
    const now = performance.now();
    return pulse > 0 && [...this.landed.values()].some((born) => now - born < pulse * 2.7);
  }

  private get easing(): boolean {
    return this.camTween !== undefined || this.riseTween !== undefined || this.fading;
  }

  /** Something changed: draw it. On the ticker if it will keep changing, otherwise once on the next animation frame. */
  private wake(): void {
    const app = this.app;
    if (app === undefined) return;
    if (this.easing) {
      cancelAnimationFrame(this.frame);
      this.frame = 0;
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
    if (rise !== undefined || this.fading) this.rebuildTiles();
    if (!this.easing) this.app?.ticker.stop();
  };
}
