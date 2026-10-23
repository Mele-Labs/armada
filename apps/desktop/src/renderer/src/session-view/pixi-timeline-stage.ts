// The canvas the Pixi take of the Artifacts timeline is drawn on. It is started as the deck's map is (`cockpit/deck/pixi/stage.ts`): no
// loop of its own, WebGL with the generated-code-free shaders, the renderer's resolution the screen's so type is crisp, and the colours read
// from the tokens the page is wearing (`palette.ts`), never written here. **A frame is drawn when something changed**, on the next animation
// frame, and not otherwise. The canvas is hidden from the accessibility tree: what a person presses, tabs to and has read to them is the layer
// of buttons over it (`PixiTimeline.tsx`), as the map's is `Hits.tsx`. The time axis is not drawn here: it is a strip of the page that stays at the
// top while the tracks scroll under it, and the canvas draws the grid its ticks stand on.

import "pixi.js/unsafe-eval";
import { Application, CanvasTextMetrics, Container, Graphics, Text, TextStyle } from "pixi.js";
import { SquareTerminal } from "lucide-react";

import { css, iconGlyph } from "../cockpit/deck/pixi/glyphs";
import { hex, mix, readPalette, type Palette, type Rgba } from "../cockpit/deck/pixi/palette";
import type { Timeline } from "./artifact-timeline";
import { LEFT, MOST_TITLE, POINT, RIGHT, STEM, ticksFor, xOf, type Row, type Tracks, type View } from "./pixi-timeline";

export type Drawn = {
  timeline: Timeline;
  tracks: Tracks;
  view: View;
  width: number;
  height: number;
  /** The point that has the keyboard's focus, drawn ringed. */
  focus: string | undefined;
  /** 0 to 1: how far from the left the lines have grown. */
  reveal: number;
};

const TITLE_FONT = 13;
/** The alpha of a branch where it passes behind another line, and how close to that line's y it is when it does. */
const BEHIND = 0.22;
const BEHIND_Y = 9;
const SAMPLES = 28;

// ---- Type, measured --------------------------------------------------------------------------------------------------------------

const widths = new Map<string, number>();
const styles = new Map<string, TextStyle>();

/** The width a title is drawn at in `family`: the same metrics the canvas lays its type out with, so the layout knows what a title needs. */
export function titleWidth(family: string, text: string): number {
  const key = `${family}|${text}`;
  let width = widths.get(key);
  if (width === undefined) {
    let style = styles.get(family);
    if (style === undefined) {
      style = new TextStyle({ fontFamily: family, fontSize: TITLE_FONT });
      styles.set(family, style);
    }
    width = CanvasTextMetrics.measureText(text, style).width;
    widths.set(key, width);
  }
  return width;
}

/** Measured again, as after a font arrives. */
export const forgetWidths = (): void => widths.clear();

/** `title` cut with an ellipsis to the most that fits in `room`, or whole where it does. */
function fit(family: string, title: string, room: number): string {
  if (titleWidth(family, title) <= room) return title;
  let low = 0;
  let high = title.length;
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    if (titleWidth(family, `${title.slice(0, mid)}…`) <= room) low = mid;
    else high = mid - 1;
  }
  return low === 0 ? "" : `${title.slice(0, low).trimEnd()}…`;
}

/** The top of a row's title: above its line on the far side from the Job, or below it. */
const titleY = (row: Row): number => row.y + (row.side === "up" ? -POINT / 2 - 20 : POINT / 2 + 2);

type Mark = { box: Container; x: number; y: number };

export class TimelineStage {
  private app: Application | undefined;
  private gone = false;
  private readonly host: HTMLElement;
  private palette: Palette | undefined;
  private sans = "sans-serif";
  private titleStyle: TextStyle | undefined;
  private themed: MutationObserver | undefined;
  private back = new Graphics();
  private ring = new Graphics();
  private readonly marks = new Container();
  private readonly texts = new Container();
  private built: { timeline: Timeline; palette: Palette } | undefined;
  private points = new Map<string, Mark>();
  private titles = new Map<string, Text>();
  private origin: Container | undefined;
  private state: Drawn | undefined;
  private frame = 0;
  private size = { width: 0, height: 0 };

  private constructor(host: HTMLElement) {
    this.host = host;
  }

  static start(host: HTMLElement): TimelineStage {
    const stage = new TimelineStage(host);
    stage.init().catch((error: unknown) => console.error("the timeline's canvas did not start", error));
    return stage;
  }

  private async init(): Promise<void> {
    const app = new Application();
    await app.init({
      preference: "webgl",
      antialias: true,
      backgroundAlpha: 0,
      autoDensity: true,
      resolution: Math.min(window.devicePixelRatio || 1, 3),
      width: Math.max(1, this.size.width),
      height: Math.max(1, this.size.height),
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
    app.stage.eventMode = "none";
    app.stage.interactiveChildren = false;
    app.canvas.setAttribute("aria-hidden", "true");
    this.host.prepend(app.canvas);
    app.stage.addChild(this.back, this.marks, this.ring, this.texts);
    this.sans = getComputedStyle(this.host).fontFamily || "sans-serif";
    this.restyle();
    // A theme put on the page changes the tokens under every colour here.
    this.themed = new MutationObserver(() => {
      this.restyle();
      this.ask();
    });
    this.themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    this.ask();
  }

  /** The palette read again from the page, and the type drawn from it. */
  private restyle(): void {
    const p = readPalette(this.host);
    this.palette = p;
    this.titleStyle = new TextStyle({ fontFamily: this.sans, fontSize: TITLE_FONT, fill: hex(p.fg) });
    // Type already drawn keeps the colour it had: it is made again.
    for (const text of this.titles.values()) text.destroy();
    this.titles.clear();
  }

  /** Draws this, now if the canvas is up and on the next frame otherwise. Several asks in a frame are one drawing. */
  set(state: Drawn): void {
    this.state = state;
    this.ask();
  }

  /** Draws again with what it has: a font has come, so the type is cut again. */
  redraw(): void {
    this.ask();
  }

  private ask(): void {
    if (this.gone || this.frame !== 0) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.paint();
    });
  }

  destroy(): void {
    this.gone = true;
    cancelAnimationFrame(this.frame);
    this.themed?.disconnect();
    const app = this.app;
    this.app = undefined;
    app?.destroy({ removeView: true }, { children: true });
  }

  // ---- Drawing ---------------------------------------------------------------------------------------------

  private colour(p: Palette, tone: string | undefined): Rgba {
    return tone === "passed" || tone === "done" ? p.good : tone === "failed" ? p.bad : tone === "pending" ? p.ask : p.muted;
  }

  /** Builds a mark for each point once for a Job and a palette; moving them is all a frame does after. */
  private build(timeline: Timeline, p: Palette): void {
    this.marks.removeChildren().forEach((one) => one.destroy({ children: true }));
    this.points.clear();
    const edge = hex(mix(p.base, p.fg, 0.3));
    for (const line of timeline.lines) {
      for (const [index, point] of line.points.entries()) {
        const box = new Container();
        const tone = point.tone === undefined ? undefined : this.colour(p, point.tone);
        const Glyph = index === 0 ? line.Glyph : point.Glyph;
        if (Glyph === undefined) {
          box.addChild(new Graphics().circle(0, 0, 4).fill({ color: hex(p.muted) }));
        } else {
          box.addChild(new Graphics().circle(0, 0, POINT / 2 - 1).fill({ color: hex(p.base) }).stroke({ width: 2, color: tone === undefined ? edge : hex(tone) }));
          box.addChild(iconGlyph(Glyph, css(tone ?? p.muted), 12));
        }
        this.marks.addChild(box);
        this.points.set(`${line.key}@${index}`, { box, x: 0, y: 0 });
      }
    }
    this.origin?.destroy({ children: true });
    const origin = new Container();
    origin.addChild(new Graphics().circle(0, 0, POINT / 2).fill({ color: hex(p.subtle) }));
    origin.addChild(iconGlyph(SquareTerminal, css(p.base), 12));
    this.marks.addChild(origin);
    this.origin = origin;
    this.built = { timeline, palette: p };
  }

  /** A branch drawn as a curve, in pieces: where it passes behind another line or its title it is faint, so a crossing reads as under. */
  private stem(g: Graphics, row: Row, rows: readonly Row[], from: Row | undefined, x: { first: number; xs: number }, py: number, colour: number, view: View, shift: number): void {
    const y = row.y;
    const at = (t: number) => {
      const u = 1 - t;
      const px = [x.xs, x.xs + STEM * 0.6, x.first - STEM * 0.6, x.first];
      const qy = [py, py, y, y];
      return {
        x: u * u * u * px[0]! + 3 * u * u * t * px[1]! + 3 * u * t * t * px[2]! + t * t * t * px[3]!,
        y: u * u * u * qy[0]! + 3 * u * u * t * qy[1]! + 3 * u * t * t * qy[2]! + t * t * t * qy[3]!,
      };
    };
    const others = rows.filter((one) => one !== row && one !== from);
    const behind = (mx: number, my: number) =>
      others.some((one) => {
        const a = xOf(view, one.line.points[0]!.t) - POINT / 2 - 4;
        const b = xOf(view, one.line.points[one.line.points.length - 1]!.t) + POINT / 2 + 4;
        if (Math.abs(my - one.y) < BEHIND_Y && mx >= a && mx <= b) return true;
        const top = titleY(one) - 3;
        return my >= top && my <= top + TITLE_FONT + 9 && mx >= one.label.x + shift - 4 && mx <= one.label.x + shift + one.label.w + 4;
      });
    let run: { x: number; y: number }[] = [at(0)];
    let faint = false;
    const flush = () => {
      if (run.length < 2) return;
      g.moveTo(run[0]!.x, run[0]!.y);
      for (const one of run.slice(1)) g.lineTo(one.x, one.y);
      g.stroke({ width: 2, color: colour, alpha: faint ? BEHIND : 1, cap: "round", join: "round" });
    };
    for (let i = 1; i <= SAMPLES; i++) {
      const to = at(i / SAMPLES);
      const mid = at((i - 0.5) / SAMPLES);
      const now = behind(mid.x, mid.y);
      if (now !== faint) {
        flush();
        run = [run[run.length - 1]!];
        faint = now;
      }
      run.push(to);
    }
    flush();
  }

  private paint(): void {
    const app = this.app;
    const s = this.state;
    if (this.gone || app === undefined || s === undefined || this.palette === undefined) return;
    const p = this.palette;
    if (s.width !== this.size.width || s.height !== this.size.height) {
      this.size = { width: s.width, height: s.height };
      app.renderer.resize(Math.max(1, s.width), Math.max(1, s.height));
    }
    if (this.built === undefined || this.built.timeline !== s.timeline || this.built.palette !== p) this.build(s.timeline, p);
    const { view, width, tracks } = s;
    const reveal = LEFT + s.reveal * (width - LEFT);
    const rule = mix(p.base, p.fg, 0.3);
    const jobY = tracks.jobY;
    const g = this.back.clear();

    // The grid under each tick of the axis above.
    for (const tick of ticksFor(s.timeline, view, width)) g.moveTo(tick.x, 0).lineTo(tick.x, s.height).stroke({ width: 1, color: hex(mix(p.base, p.fg, 0.07)) });

    // The Job's own line, and what it grew to so far.
    g.moveTo(LEFT - 12, jobY).lineTo(Math.max(LEFT - 12, Math.min(width - RIGHT, reveal)), jobY).stroke({ width: 3, color: hex(p.subtle), cap: "round" });
    this.origin?.position.set(LEFT - 12, jobY);
    if (this.origin !== undefined) this.origin.visible = true;

    // Branches first, so every line is drawn over them.
    const shift = xOf(view, s.timeline.start) - LEFT;
    const rowOf = new Map(tracks.rows.map((one) => [one.line.key, one] as const));
    for (const row of tracks.rows) {
      const first = xOf(view, row.line.points[0]!.t);
      if (first - STEM > reveal || first < LEFT - STEM || first - STEM > width) continue;
      // Level along the parent, then a smooth curve out to its own track, arriving level.
      const parent = row.parent.job === true ? undefined : rowOf.get(row.parent.row);
      const py = parent === undefined ? jobY : parent.y;
      const xs = first - STEM;
      if (parent !== undefined) {
        const born = row.line.points[0]!.t;
        const at = parent.line.points.reduce((found, point, i) => (point.t <= born ? i : found), 0);
        const from = xOf(view, parent.line.points[at]!.t);
        if (from < xs) g.moveTo(from, py).lineTo(xs, py).stroke({ width: 2, color: hex(p.accent), cap: "round" });
      }
      this.stem(g, row, tracks.rows, parent, { first, xs }, py, hex(parent === undefined ? rule : p.accent), view, shift);
    }

    for (const row of tracks.rows) {
      const first = xOf(view, row.line.points[0]!.t);
      const last = xOf(view, row.line.points[row.line.points.length - 1]!.t);
      const y = row.y;
      if (last > first) g.moveTo(Math.max(first, LEFT - 12), y).lineTo(Math.min(last, reveal), y).stroke({ width: 2, color: hex(rule), cap: "round" });
      for (const [index, point] of row.line.points.entries()) {
        const mark = this.points.get(`${row.line.key}@${index}`);
        if (mark === undefined) continue;
        const x = xOf(view, point.t);
        mark.x = x;
        mark.y = y;
        mark.box.position.set(x, y);
        mark.box.visible = x <= reveal && x >= LEFT - 12 && x <= width + 12;
      }
    }

    // The focused point is ringed.
    this.ring.clear();
    const focused = s.focus === undefined ? undefined : this.points.get(s.focus);
    if (focused !== undefined && focused.box.visible) this.ring.circle(focused.x, focused.y, POINT / 2 + 3).stroke({ width: 2, color: hex(p.accent) });

    this.titlesFor(s, reveal);
    app.render();
  }

  /** Each title on its own line's track, beside its first point. Panned past the left edge it stays there while any of it is left to read. */
  private titlesFor(s: Drawn, reveal: number): void {
    const style = this.titleStyle!;
    const seen = new Set<string>();
    // The layout is packed with the plot's left at the Job's start; the view may have moved it.
    const shift = xOf(s.view, s.timeline.start) - LEFT;
    for (const row of s.tracks.rows) {
      seen.add(row.line.key);
      let text = this.titles.get(row.line.key);
      if (text === undefined) {
        text = new Text({ text: "", style });
        this.texts.addChild(text);
        this.titles.set(row.line.key, text);
      }
      const first = xOf(s.view, row.line.points[0]!.t);
      const x0 = row.label.x + shift;
      const x = Math.max(x0, LEFT - 12);
      const left = x0 + row.label.w - x;
      const shown = first - POINT / 2 > reveal || x0 > s.width || left < Math.min(row.label.w, 28) ? "" : fit(this.sans, row.line.title, Math.min(left, MOST_TITLE));
      text.visible = shown !== "";
      if (shown === "") continue;
      text.text = shown;
      text.position.set(x, titleY(row));
    }
    for (const [key, text] of this.titles) {
      if (seen.has(key)) continue;
      text.destroy();
      this.titles.delete(key);
    }
  }
}
