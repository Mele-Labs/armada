// The canvas the Pixi take of the Artifacts timeline is drawn on. It is started as the deck's map is (`cockpit/deck/pixi/stage.ts`): no
// loop of its own, WebGL with the generated-code-free shaders, the renderer's resolution the screen's so type is crisp, and the colours read
// from the tokens the page is wearing (`palette.ts`), never written here. **A frame is drawn when something changed**, on the next animation
// frame, and not otherwise. The canvas is hidden from the accessibility tree: what a person presses, tabs to and has read to them is the layer
// of buttons over it (`PixiTimeline.tsx`), as the map's is `Hits.tsx`.

import "pixi.js/unsafe-eval";
import { Application, CanvasTextMetrics, Container, Graphics, Text, TextStyle } from "pixi.js";
import { SquareTerminal } from "lucide-react";

import { css, iconGlyph } from "../cockpit/deck/pixi/glyphs";
import { hex, mix, readPalette, type Palette, type Rgba } from "../cockpit/deck/pixi/palette";
import type { Timeline } from "./artifact-timeline";
import { BAND, LEFT, POINT, RIGHT, STEM, roomFor, ticksFor, xOf, type Tracks, type View } from "./pixi-timeline";

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
  clock: (ms: number) => string;
};

const TICK_FONT = 12;
const TITLE_FONT = 13;

type Mark = { box: Container; x: number };

export class TimelineStage {
  private app: Application | undefined;
  private gone = false;
  private readonly host: HTMLElement;
  private palette: Palette | undefined;
  private sans = "sans-serif";
  private tickStyle: TextStyle | undefined;
  private titleStyle: TextStyle | undefined;
  private themed: MutationObserver | undefined;
  private back = new Graphics();
  private ring = new Graphics();
  private readonly marks = new Container();
  private readonly texts = new Container();
  private built: { tracks: Tracks; palette: Palette } | undefined;
  private points = new Map<string, Mark>();
  private titles = new Map<string, Text>();
  private ticks: Text[] = [];
  private origin: Container | undefined;
  private widths = new Map<string, number>();
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
    // A name measured before its font arrived is measured again once it has.
    void document.fonts.ready.then(() => {
      this.widths.clear();
      this.ask();
    });
    this.ask();
  }

  /** The palette read again from the page, and the type drawn from it. */
  private restyle(): void {
    const p = readPalette(this.host);
    this.palette = p;
    this.tickStyle = new TextStyle({ fontFamily: p.mono, fontSize: TICK_FONT, fontWeight: "600", fill: hex(p.muted) });
    this.titleStyle = new TextStyle({ fontFamily: this.sans, fontSize: TITLE_FONT, fill: hex(p.fg) });
    this.widths.clear();
    // Type already drawn keeps the colour it had: it is made again.
    for (const text of [...this.titles.values(), ...this.ticks]) text.destroy();
    this.titles.clear();
    this.ticks = [];
  }

  /** Draws this, now if the canvas is up and on the next frame otherwise. Several asks in a frame are one drawing. */
  set(state: Drawn): void {
    this.state = state;
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

  /** Builds a mark for each point once for a layout and a palette; moving them is all a frame does after. */
  private build(tracks: Tracks, p: Palette): void {
    this.marks.removeChildren().forEach((one) => one.destroy({ children: true }));
    this.points.clear();
    const edge = hex(mix(p.base, p.fg, 0.3));
    for (const row of tracks.rows) {
      for (const [index, point] of row.line.points.entries()) {
        const box = new Container();
        const tone = point.tone === undefined ? undefined : this.colour(p, point.tone);
        const Glyph = index === 0 ? row.line.Glyph : point.Glyph;
        if (Glyph === undefined) {
          box.addChild(new Graphics().circle(0, 0, 4).fill({ color: hex(p.muted) }));
        } else {
          box.addChild(new Graphics().circle(0, 0, POINT / 2 - 1).fill({ color: hex(p.base) }).stroke({ width: 2, color: tone === undefined ? edge : hex(tone) }));
          box.addChild(iconGlyph(Glyph, css(tone ?? p.muted), 12));
        }
        this.marks.addChild(box);
        this.points.set(`${row.line.key}@${index}`, { box, x: 0 });
      }
    }
    this.origin?.destroy({ children: true });
    const origin = new Container();
    origin.addChild(new Graphics().circle(0, 0, POINT / 2).fill({ color: hex(p.subtle) }));
    origin.addChild(iconGlyph(SquareTerminal, css(p.base), 12));
    this.marks.addChild(origin);
    this.origin = origin;
    this.built = { tracks, palette: p };
  }

  private measure(text: string, style: TextStyle): number {
    const key = `${style.fontSize}|${text}`;
    let width = this.widths.get(key);
    if (width === undefined) {
      width = CanvasTextMetrics.measureText(text, style).width;
      this.widths.set(key, width);
    }
    return width;
  }

  /** `title` cut with an ellipsis to the most that fits in `room`, or whole where it does. */
  private fit(title: string, room: number, style: TextStyle): string {
    if (this.measure(title, style) <= room) return title;
    let low = 0;
    let high = title.length;
    while (low < high) {
      const mid = Math.ceil((low + high) / 2);
      if (this.measure(`${title.slice(0, mid)}…`, style) <= room) low = mid;
      else high = mid - 1;
    }
    return low === 0 ? "" : `${title.slice(0, low).trimEnd()}…`;
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
    if (this.built === undefined || this.built.tracks !== s.tracks || this.built.palette !== p) this.build(s.tracks, p);
    const { view, width, tracks } = s;
    const reveal = LEFT + s.reveal * (width - LEFT);
    const rule = mix(p.base, p.fg, 0.3);
    const jobY = BAND + tracks.jobY;
    const g = this.back.clear();

    // The band, and the grid under each tick.
    g.rect(0, 0, width, BAND).fill({ color: hex(mix(p.base, p.fg, 0.05)) });
    g.moveTo(0, BAND).lineTo(width, BAND).stroke({ width: 1, color: hex(mix(p.base, p.fg, 0.14)) });
    const ticks = ticksFor(s.timeline, view, width);
    const tickStyle = this.tickStyle!;
    while (this.ticks.length < ticks.length) {
      const one = new Text({ text: "", style: tickStyle });
      one.anchor.set(0.5);
      this.texts.addChild(one);
      this.ticks.push(one);
    }
    this.ticks.forEach((text, i) => {
      const tick = ticks[i];
      text.visible = tick !== undefined;
      if (tick === undefined) return;
      text.text = s.clock(tick.t);
      text.position.set(tick.x, BAND / 2);
      g.moveTo(tick.x, BAND).lineTo(tick.x, s.height).stroke({ width: 1, color: hex(mix(p.base, p.fg, 0.07)) });
    });

    // The Job's own line, and what it grew to so far.
    g.moveTo(LEFT - 12, jobY).lineTo(Math.max(LEFT - 12, Math.min(width - RIGHT, reveal)), jobY).stroke({ width: 3, color: hex(p.subtle), cap: "round" });
    this.origin?.position.set(LEFT - 12, jobY);
    if (this.origin !== undefined) this.origin.visible = true;

    const rowOf = new Map(tracks.rows.map((one) => [one.line.key, one] as const));
    for (const row of tracks.rows) {
      const first = xOf(view, row.line.points[0]!.t);
      const last = xOf(view, row.line.points[row.line.points.length - 1]!.t);
      const y = BAND + row.y;
      // Its branch: level along the parent, then a smooth curve out to its own track, arriving level.
      if (first - STEM <= reveal && first >= LEFT - STEM && first - STEM <= width) {
        const parent = row.parent.job === true ? undefined : rowOf.get(row.parent.row);
        const py = parent === undefined ? jobY : BAND + parent.y;
        const xs = first - STEM;
        const colour = parent === undefined ? rule : p.accent;
        if (parent !== undefined) {
          const born = row.line.points[0]!.t;
          const at = parent.line.points.reduce((found, point, i) => (point.t <= born ? i : found), 0);
          const from = xOf(view, parent.line.points[at]!.t);
          if (from < xs) g.moveTo(from, py).lineTo(xs, py);
        }
        g.moveTo(xs, py).bezierCurveTo(xs + STEM * 0.6, py, first - STEM * 0.6, y, first, y).stroke({ width: 2, color: hex(colour), cap: "round" });
      }
      if (last > first) g.moveTo(Math.max(first, LEFT - 12), y).lineTo(Math.min(last, reveal), y).stroke({ width: 2, color: hex(rule), cap: "round" });
      for (const [index, point] of row.line.points.entries()) {
        const mark = this.points.get(`${row.line.key}@${index}`);
        if (mark === undefined) continue;
        const x = xOf(view, point.t);
        mark.x = x;
        mark.box.position.set(x, y);
        mark.box.visible = x <= reveal && x >= LEFT - 12 && x <= width + 12;
      }
    }

    // The focused point is ringed.
    this.ring.clear();
    const focused = s.focus === undefined ? undefined : this.points.get(s.focus);
    if (focused !== undefined && focused.box.visible) this.ring.circle(focused.box.x, focused.box.y, POINT / 2 + 3).stroke({ width: 2, color: hex(p.accent) });

    this.titlesFor(s, reveal);
    app.render();
  }

  /** Each title at its line's start, held at the left edge while its line runs on past it, and cut to the room it has. */
  private titlesFor(s: Drawn, reveal: number): void {
    const style = this.titleStyle!;
    const seen = new Set<string>();
    for (const row of s.tracks.rows) {
      seen.add(row.line.key);
      let text = this.titles.get(row.line.key);
      if (text === undefined) {
        text = new Text({ text: "", style });
        this.texts.addChild(text);
        this.titles.set(row.line.key, text);
      }
      const first = xOf(s.view, row.line.points[0]!.t);
      const last = xOf(s.view, row.line.points[row.line.points.length - 1]!.t);
      const x = Math.max(first - POINT / 2, LEFT - 12);
      const room = roomFor(row, s.tracks, s.view, s.width, x);
      // Held at the edge, a title stays only while enough of its line is left to speak for.
      const held = first - POINT / 2 < x;
      const shown = first - POINT / 2 > reveal || (held && last + POINT / 2 - x < 40) || room < 28 || first > s.width ? "" : this.fit(row.line.title, room, style);
      text.visible = shown !== "";
      if (shown === "") continue;
      text.text = shown;
      text.position.set(x, BAND + row.y + (row.side === "up" ? -POINT / 2 - 20 : POINT / 2 + 2));
    }
    for (const [key, text] of this.titles) {
      if (seen.has(key)) continue;
      text.destroy();
      this.titles.delete(key);
    }
  }
}
