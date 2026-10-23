// A registry glyph drawn on the canvas. The icons are lucide's, so the canvas reads each one's own SVG and strokes it, rather than redrawing a
// shape that would drift from the glyph the rest of the app shows. Each element carries its stroke itself, since the canvas does not inherit
// it from the root.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Github, Globe, Plug, Ticket, type LucideIcon } from "lucide-react";
import { Container, Graphics } from "pixi.js";

import type { OutpostGlyph } from "../outposts";
import { hex, type Rgba } from "./palette";

const ICONS = { forge: Github, ticket: Ticket, globe: Globe, plug: Plug } as const;

/** The side of lucide's own drawing board. */
const BOARD = 24;

const markup = new Map<OutpostGlyph, string>();

/** `raw` as SVG markup with its stroke spelled on every element. */
function stroked(raw: string, stroke: string): string {
  // The root says `currentColor`, which the canvas cannot resolve.
  return raw
    .replace(/ stroke="currentColor"/, "")
    .replace(/<(path|circle|rect|line|polyline|polygon|ellipse)\b/g, `<$1 fill="none" stroke="${stroke}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"`);
}

/** `name` as SVG markup with its stroke spelled on every element. */
function svgOf(name: OutpostGlyph, stroke: string): string {
  const raw = markup.get(name) ?? renderToStaticMarkup(createElement(ICONS[name]));
  markup.set(name, raw);
  return stroked(raw, stroke);
}

/** A colour as the `#rrggbb` an SVG attribute takes. */
export const css = (c: Rgba): string => `#${hex(c).toString(16).padStart(6, "0")}`;

/** The glyph centred on the origin, `size` across, stroked in `stroke` (a #rrggbb). */
export function glyph(name: OutpostGlyph, stroke: string, size: number): Container {
  const g = new Graphics().svg(svgOf(name, stroke));
  g.pivot.set(BOARD / 2, BOARD / 2);
  g.scale.set(size / BOARD);
  const out = new Container();
  out.addChild(g);
  return out;
}

const icons = new Map<LucideIcon, string>();

/** Any registry glyph, centred on the origin, `size` across, stroked in `stroke` (a #rrggbb): what the timeline's points wear. */
export function iconGlyph(icon: LucideIcon, stroke: string, size: number): Container {
  const raw = icons.get(icon) ?? renderToStaticMarkup(createElement(icon));
  icons.set(icon, raw);
  const g = new Graphics().svg(stroked(raw, stroke));
  g.pivot.set(BOARD / 2, BOARD / 2);
  g.scale.set(size / BOARD);
  const out = new Container();
  out.addChild(g);
  return out;
}
