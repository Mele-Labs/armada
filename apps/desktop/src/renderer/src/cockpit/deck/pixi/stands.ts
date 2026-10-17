// What stands upright on the map: a base's hangar and its name, the flag that is main, a Job's pins, the crate of a pull request and the
// stakes of a survey. Each is a picture drawn once around its own origin, which the stage puts at the screen point of its anchor and
// scales by the zoom and the perspective, so it faces the viewer however the plane is tilted. A name is also held to a readable size
// as the field is zoomed out, as the CSS held it. Drones go in the layer above these, in the same depth order.

import { Container, Graphics, Text, TextStyle } from "pixi.js";

import { onPlane } from "../bf-parts";
import { FLAT, type P3 } from "../layout";
import type { PinState } from "../mission";
import { fade, hex, mix, type Palette, type Rgba } from "./palette";
import type { Crate, PixiSceneIn } from "./scene";

/** A standing picture: where it stands, and how to draw it. `sig` is everything that changes the picture, so it is redrawn only when that does. */
export type StandSpec = {
  key: string;
  sig: string;
  at: P3;
  /** Held to a readable size as the field is zoomed out: a name, and not a picture. */
  counter?: boolean;
  /** The plot it lies on: it stands at that plot's height as it is now, not where the scene says it is going. */
  onTile?: string;
  /** A pull request on its way: where it hops from, along an arc, to `at`; landing, it goes once it has arrived. */
  hop?: { id: string; from: P3; delay: number; landing: boolean };
  make: (p: Palette) => Container;
};

const colour = (c: Rgba) => ({ color: hex(c), alpha: c[3] });

/** A name: the smallest type, in mono, with a halo of the ground so it reads over a plot. */
export function name(text: string, p: Palette, fill: Rgba, anchor: { x: number; y: number }): Text {
  const label = new Text({
    text,
    style: new TextStyle({ fontFamily: p.mono, fontSize: p.small, fill: hex(fill), letterSpacing: p.small * 0.04, stroke: { color: hex(p.base), width: 4, join: "round" } }),
    resolution: Math.min(window.devicePixelRatio || 1, 2) * 1.5,
  });
  label.anchor.set(anchor.x, anchor.y);
  label.alpha = fill[3];
  return label;
}

function hangar(p: Palette): Container {
  const g = new Graphics();
  const face = (points: number[], over: number) => g.poly(points).fill(colour(mix(p.base, p.fg, over))).stroke({ ...colour(fade(p.subtle, 0.6)), width: 1, join: "round" });
  face([-26, -8, 0, -18, 26, -8, 0, 2], 0.24);
  face([-26, -8, 0, 2, 0, 16, -26, 6], 0.13);
  face([26, -8, 0, 2, 0, 16, 26, 6], 0.07);
  g.moveTo(18, -12).lineTo(18, -30).stroke({ ...colour(p.subtle), width: 1.2 });
  g.circle(18, -31, 1.8).fill(colour(p.muted));
  g.scale.set(1.3);
  const out = new Container();
  out.addChild(g);
  return out;
}

function flag(red: boolean, hot: boolean, p: Palette): Container {
  const tone = red ? p.bad : p.good;
  const out = new Container();
  const g = new Graphics();
  // A soft light round the head of the pole.
  for (const [r, a] of [[26, 0.05], [18, 0.08], [11, 0.12]] as const) g.circle(0, -73, r).fill(colour(fade(tone, a)));
  g.moveTo(0, 0).lineTo(0, -52).stroke({ ...colour(tone), width: 2, cap: "round" });
  g.poly([0, -52, 26, -45, 0, -37]).fill(colour(fade(tone, hot ? 1 : 0.9)));
  g.circle(0, -54, 2.6).fill(colour(tone));
  g.scale.set(1.35);
  out.addChild(g);
  const label = name("main", p, tone, { x: 0, y: 1 });
  label.position.set(40, -62);
  out.addChild(label);
  return out;
}

const PIN_TONE: Record<PinState, (p: Palette) => Rgba> = { done: (p) => p.good, live: (p) => p.hot, next: (p) => p.dormant, failed: (p) => p.bad, ask: (p) => p.ask };

function pin(state: PinState, label: string | undefined, bright: boolean, pinned: boolean, hot: boolean, p: Palette): Container {
  const tone = PIN_TONE[state](p);
  const body = state === "next" ? mix(tone, p.base, 0.45) : tone;
  const out = new Container();
  const g = new Graphics();
  if (state === "failed") g.moveTo(-9, -44).lineTo(-13, -55).moveTo(0, -47).lineTo(0, -61).moveTo(9, -44).lineTo(13, -55).stroke({ ...colour(p.bad), width: 1.7, cap: "round" });
  g.moveTo(0, 0).bezierCurveTo(-3, -8, -12, -15, -12, -25).arc(0, -25, 12, Math.PI, 0).bezierCurveTo(12, -15, 3, -8, 0, 0).closePath();
  g.fill(colour(body)).stroke({ ...colour(pinned || hot ? p.fg : p.base), width: 1.6, join: "round" });
  const ink = { ...colour(p.base), width: 2.6, cap: "round" as const, join: "round" as const };
  if (state === "done") g.moveTo(-5, -26).lineTo(-1.5, -22.5).lineTo(5, -30).stroke(ink);
  else if (state === "failed") g.moveTo(-4.5, -29.5).lineTo(4.5, -20.5).moveTo(4.5, -29.5).lineTo(-4.5, -20.5).stroke(ink);
  else if (state === "ask") g.moveTo(0, -31).lineTo(0, -25).moveTo(0, -21.4).lineTo(0, -21.2).stroke(ink);
  else if (state === "live") g.circle(0, -25.5, 4).fill(colour(p.base));
  else g.circle(0, -25.5, 3.6).stroke(ink);
  g.scale.set(1.15);
  out.addChild(g);
  if (label !== undefined) {
    const text = name(label.length > 22 ? `${label.slice(0, 21)}…` : label, p, bright ? p.fg : p.muted, { x: 0.5, y: 1 });
    text.position.set(0, 44);
    out.addChild(text);
  }
  return out;
}

function crate(stage: Crate["pr"]["stage"], dim: boolean, hot: boolean, p: Palette): Container {
  const solid = stage === "ready" || stage === "queued" || stage === "landed";
  const red = stage === "red";
  const fill = red ? mix(p.base, p.bad, 0.28) : solid ? p.fg : p.base;
  const edge = hot ? p.fg : red ? p.bad : solid ? p.fg : p.muted;
  const dot = red ? p.bad : solid ? p.base : p.muted;
  const g = new Graphics();
  g.poly([0, -6.5, 7.5, 0, 0, 6.5, -7.5, 0]).fill(colour(fill)).stroke({ ...colour(edge), width: 1.5, join: "round" });
  g.circle(0, 0, 1.8).fill(colour(dot));
  g.scale.set(1.5);
  g.alpha = dim ? 0.15 : 1;
  const out = new Container();
  out.addChild(g);
  return out;
}

function marker(kind: "walk" | "asked", p: Palette): Container {
  const g = new Graphics();
  g.moveTo(0, 0).lineTo(0, -16).stroke({ ...colour(p.muted), width: 1.3, cap: "round" });
  if (kind === "walk") g.rect(0, -16, 9, 6).fill(colour(fade(p.accent, 0.85)));
  else g.circle(4, -14, 3.6).fill(colour(fade(p.accent, 0.85)));
  const out = new Container();
  out.addChild(g);
  return out;
}

export function standsOf(s: PixiSceneIn): StandSpec[] {
  const out: StandSpec[] = [];
  for (const one of s.layout.territories) {
    const { base } = one;
    const hangarAt = onPlane({ x: base.x - base.w / 2 + 36, y: base.y - 2 });
    const centre = onPlane({ x: base.x, y: base.y + 4 });
    out.push({ key: `hangar:${one.root}`, sig: "hangar", at: { ...hangarAt, z: 24 }, make: hangar });
    out.push({ key: `name:${one.root}`, sig: one.name, at: { x: centre.x, y: centre.y + 30 * FLAT, z: 0 }, counter: true, make: (q) => {
      const c = new Container();
      const text = name(one.name, q, q.subtle, { x: 0.5, y: 1 });
      text.position.set(0, -2);
      c.addChild(text);
      return c;
    } });
    const goal = s.goals.get(one.root);
    const hot = s.hotPart === `goal:${one.root}`;
    if (goal !== undefined && one.goal !== undefined) out.push({ key: `flag:${one.root}`, sig: `flag:${goal.red}:${hot}`, at: { ...onPlane(one.goal), z: 1 }, make: (q) => flag(goal.red, hot, q) });
  }
  for (const at of s.route?.pins ?? []) {
    const state = at.state;
    const bright = state === "live" || state === "ask" || state === "failed" || s.pinned === at.id;
    const hot = s.hotPart === `pin:${at.id}`;
    out.push({
      key: `pin:${at.id}`,
      sig: `pin:${state}:${s.readable ? at.label : ""}:${bright}:${s.pinned === at.id}:${hot}`,
      at: { ...onPlane(at.at), z: 0 },
      make: (q) => pin(state, s.readable ? at.label : undefined, bright, s.pinned === at.id, hot, q),
    });
  }
  for (const one of s.crates) {
    const hot = s.hotPart === `crate:${one.pr.key}`;
    out.push({ key: `crate:${one.pr.key}`, sig: `crate:${one.pr.stage}:${one.dim}:${hot}`, at: one.at, ...(one.hop === undefined ? {} : { hop: one.hop }), make: (q) => crate(one.pr.stage, one.dim, hot, q) });
  }
  // A plot's name lies on it, at its near corner, and rises with it.
  for (const one of s.names) {
    const { rect } = one.tile;
    out.push({
      key: `plot:${one.tile.path}`,
      sig: `plot:${one.text}:${one.clash}`,
      at: { x: rect.x + 6, y: rect.y * FLAT + 5, z: s.heights.get(one.tile.path) ?? 0 },
      onTile: one.tile.path,
      make: (q) => {
        const c = new Container();
        const text = name(one.text, q, one.clash ? q.ask : q.muted, { x: 0, y: 1 });
        text.position.set(0, -1);
        c.addChild(text);
        return c;
      },
    });
  }
  for (const placed of s.builds) {
    const anchor = onPlane(placed.anchor);
    placed.build.markers.forEach((kind, i) => {
      out.push({ key: `marker:${placed.build.key}:${i}`, sig: `marker:${kind}`, at: { x: anchor.x - 40 * s.big + i * 15, y: anchor.y + 34 * s.big, z: 0 }, make: (q) => marker(kind, q) });
    });
  }
  return out;
}
