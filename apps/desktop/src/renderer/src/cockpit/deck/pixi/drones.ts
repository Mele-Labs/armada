// A drone, drawn once around its own origin (its centre, at altitude) and put on the screen by the stage: a Session, the small plus quad. **It hovers still**: the rotors do not turn and it does not bob or circle, whatever it is doing. What it
// is doing is on the ground under it (`ground.ts`). Gray shell, with the nose lens, the LED and a soft glow behind it carrying the hue.
// The marker of a call is a second picture, which the stage holds at one size on the screen however far the camera stands.

import { Container, Graphics, GraphicsPath } from "pixi.js";

import { bracketsOf } from "../geometry";
import type { Hue } from "../layout";
import { glowSprite } from "./glow";
import { fade, hex, mix, type Palette, type Rgba } from "./palette";
import type { DroneSpec } from "./scene";
import { name } from "./stands";

const colour = (c: Rgba) => ({ color: hex(c), alpha: c[3] });

export const hueOf = (hue: Hue, p: Palette): Rgba => (hue === "ask" ? p.ask : hue === "bad" ? p.bad : p.muted);

/** How much of a drone shows: faint when filtered out, faded when it is another Job's, dim when another is picked, muted when he is locked on another. */
export const alphaOf = (s: DroneSpec): number => (s.away ? 0.12 : s.ghost ? GHOST : s.dim ? 0.4 : s.muted ? 0.6 : 1);

/** How large a subagent is drawn beside the drone that sent it. */
export const SMALL = 0.68;

/** How much of another Job's drone shows on a Job's own Field. */
export const GHOST = 0.25;

/** Everything that changes the picture, so a drone is drawn again only when one of these does. */
export function droneSig(s: DroneSpec): string {
  const d = s.drone;
  return [d.kind, d.hue, d.spin, d.ping, d.flare, s.selected, s.hot, s.labelled && d.title, s.muted, s.away, s.dim, s.call, s.reason, s.leaving, s.clashes, s.ghost, s.small].join(":");
}

/** A rotor: a squashed disc so it reads at an angle, its blades standing still inside it. */
function rotor(g: Graphics, x: number, y: number, r: number, spin: boolean, p: Palette): void {
  g.ellipse(x, y, r, r * 0.38).fill(colour(fade(p.fg, spin ? 0.15 : 0.08))).stroke({ ...colour(fade(p.muted, 0.5)), width: 1 });
  const blade = { ...colour(fade(p.fg, spin ? 0.8 : 0.5)), width: 1.6, cap: "round" as const };
  g.moveTo(x - r, y).lineTo(x + r, y).stroke(blade);
  g.moveTo(x, y - r * 0.38).lineTo(x, y + r * 0.38).stroke(blade);
  g.circle(x, y, 2).fill(colour(mix(p.base, p.fg, 0.45))).stroke({ ...colour(p.base), width: 0.8 });
}

/** The quad itself: the small plus. A subagent is drawn as the small one. */
export function body(spin: boolean, hue: Rgba, p: Palette): Graphics {
  const g = new Graphics();
  const arm = { ...colour(p.muted), width: 2, cap: "round" as const };
  const hull = { ...colour(mix(p.muted, hue, 0.7)), width: 1.2 };
  g.moveTo(-13, 0).lineTo(-4, 0).moveTo(13, 0).lineTo(4, 0).moveTo(0, -8).lineTo(0, -3).moveTo(0, 8).lineTo(0, 3).stroke(arm);
  rotor(g, -15, 0, 7, spin, p);
  rotor(g, 15, 0, 7, spin, p);
  rotor(g, 0, -10, 6, spin, p);
  g.circle(0, 0, 6.2).fill(colour(mix(p.base, p.fg, 0.38))).stroke(hull);
  g.circle(0, 0, 3.2).fill(colour(mix(p.base, p.fg, 0.15)));
  g.circle(0, 0, 1.4).fill(colour(hue));
  rotor(g, 0, 10, 7, spin, p);
  return g;
}

/** What the drone shows besides its body: a red flare over a failing Check, the lit mast of a call, the lock-on brackets. */
function marks(s: DroneSpec, wide: number, hue: Rgba, p: Palette): Graphics {
  const g = new Graphics();
  if (s.drone.flare) g.moveTo(-9, -16).lineTo(-13, -27).moveTo(0, -19).lineTo(0, -33).moveTo(9, -16).lineTo(13, -27).stroke({ ...colour(p.bad), width: 1.7, cap: "round" });
  if (s.drone.ping) {
    g.moveTo(0, -7).lineTo(0, -20).stroke({ ...colour(hue), width: 1.5, cap: "round" });
    g.circle(0, -21.5, 2.6).fill(colour(hue));
  }
  if (s.selected || s.hot) {
    const brackets = new GraphicsPath(bracketsOf(wide + 14, -34, 22));
    g.path(brackets).stroke({ ...colour(p.accent), width: 2, cap: "round", join: "round" });
  }
  return g;
}

// Lucide's megaphone and swords, as the page draws them, on a 24 grid.
const MEGAPHONE = [
  "M11 6a13 13 0 0 0 8.4-2.8A1 1 0 0 1 21 4v12a1 1 0 0 1-1.6.8A13 13 0 0 0 11 14H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z",
  "M6 14a12 12 0 0 0 2.4 7.2 2 2 0 0 0 3.2-2.4A8 8 0 0 1 10 14",
  "M8 6v8",
];

/** How many characters of a reason the label over a marker holds. */
const REASON_MOST = 30;

/**
 * The marker of a call he has not opened: a badge over the drone on a stem, amber while it is owed and red while its Check is failing,
 * lit so it is found across the field. Its origin is the foot of the stem. The stage scales it against the zoom, so it is one size. A `reason` is the label over the badge.
 */
export function callMark(tone: "ask" | "bad", p: Palette, reason?: string): Container {
  const c = tone === "bad" ? p.bad : p.ask;
  const out = new Container();
  const lit = glowSprite(46, c, 0.7);
  lit.position.set(0, -16 - 17);
  out.addChild(lit);
  const g = new Graphics();
  g.rect(-1, -16, 2, 16).fill(colour(c));
  g.circle(0, -33, 17).fill(colour(c)).stroke({ ...colour(p.base), width: 2 });
  const icon = new Graphics();
  for (const d of MEGAPHONE) icon.path(new GraphicsPath(d)).stroke({ ...colour(p.base), width: 2.2, cap: "round", join: "round" });
  icon.scale.set(18 / 24);
  icon.position.set(-9, -33 - 9);
  out.addChild(g, icon);
  // Why it calls, over the badge: the smallest type, cut where it would run past the badge's neighbours.
  if (reason !== undefined) {
    const said = reason.length > REASON_MOST ? `${reason.slice(0, REASON_MOST - 1)}…` : reason;
    const label = name(said, p, p.fg, { x: 0.5, y: 1 });
    label.position.set(0, -33 - 17 - 5);
    out.addChild(label);
  }
  return out;
}

const SWORDS = [
  "M14.5 17.5 3 6V3h3l11.5 11.5",
  "M13 19l6-6",
  "M16 16l4 4",
  "M19 21l2-2",
  "M14.5 6.5 18 3h3v3l-3.5 3.5",
  "M5 14l4 4",
  "M7 17l-3 3",
  "M3 19l2 2",
];

/** A write clash: a smaller amber badge beside the drone's head, with no stem and no pillar. It is a mark on the drone, not a call. */
function clashMark(p: Palette): Container {
  const out = new Container();
  const lit = glowSprite(34, p.ask, 0.55);
  lit.position.set(0, -13);
  out.addChild(lit);
  const g = new Graphics();
  g.circle(0, -13, 13).fill(colour(p.ask)).stroke({ ...colour(p.base), width: 2 });
  out.addChild(g);
  const icon = new Graphics();
  for (const d of SWORDS) icon.path(new GraphicsPath(d)).stroke({ ...colour(p.base), width: 2.2, cap: "round", join: "round" });
  icon.scale.set(14 / 24);
  icon.position.set(-7, -13 - 7);
  out.addChild(icon);
  return out;
}

export type DroneParts = {
  /** Its picture, which the stage puts at the screen point of the drone, scaled by the zoom and the perspective. */
  body: Container;
  /** Its call marker, which the stage puts a little over it, or nothing. */
  mark: Container | undefined;
  /** A mark for each write clash it is in, which the stage puts beside the marker, one after another. */
  clashes: Container[];
  /** How far over the drone the marker stands, in the scene's units. */
  head: number;
};

export function drawDrone(s: DroneSpec, big: number, p: Palette): DroneParts {
  const d = s.drone;
  const wide = 20;
  const hue = hueOf(d.hue, p);
  const out = new Container();
  const shell = new Container();
  // A subagent is drawn smaller than the drone that sent it.
  shell.scale.set(big * (s.small ? SMALL : 1));
  // The light a drone gives off: stronger where it is calling or failing, and where it is at work.
  const stands = d.hue !== "calm" || d.spin;
  if ((stands || s.selected) && !s.ghost) shell.addChild(glowSprite(30, s.selected ? p.accent : hue, d.hue !== "calm" ? 0.5 : d.spin ? 0.3 : 0.2));
  shell.addChild(body(d.spin, hue, p), marks(s, wide, hue, p));
  if (d.ping || d.flare) {
    const lamp = glowSprite(10, d.flare ? p.bad : hue, 0.9);
    lamp.position.set(0, d.flare ? -22 : -21.5);
    shell.addChild(lamp);
  }
  out.addChild(shell);
  if (s.labelled && !s.ghost) {
    const text = d.title.length > 22 ? `${d.title.slice(0, 21)}…` : d.title;
    const label = name(text, p, d.hue !== "calm" || s.selected ? p.fg : p.muted, { x: 0.5, y: 1 });
    label.position.set(0, -Math.round(24 * big + (d.flare ? 26 : d.ping ? 14 : 4) + (s.selected ? 14 : 0)));
    out.addChild(label);
  }
  // Muted when he is locked on another drone and this one is calm; dim when another is picked on the timeline; faint when filtered out.
  out.alpha = alphaOf(s);
  const tone = s.call;
  const head = Math.round(40 * big);
  const mark = tone === undefined ? undefined : callMark(tone, p, s.reason);
  if (mark !== undefined && s.leaving) mark.alpha = 0.35;
  return { body: out, mark, clashes: Array.from({ length: s.clashes }, () => clashMark(p)), head };
}
