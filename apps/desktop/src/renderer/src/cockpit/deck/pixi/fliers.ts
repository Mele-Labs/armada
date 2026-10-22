// The drones on the screen: each is drawn when it changes (`drones.ts`) and put where it flies as the camera moves. A launch lifts one from
// its pad along a rising arc to its place; every other drone is simply there, hovering still. The pillar of a call is a pillar of light
// from the plot up past the badge, found across the field.

import { Container, type Sprite } from "pixi.js";

import { alongOf } from "../layout";
import { alphaOf, drawDrone, droneSig, type DroneParts } from "./drones";
import { pillarSprite } from "./glow";
import type { Palette } from "./palette";
import { project, type View } from "./project";
import type { DroneSpec, PixiSceneIn } from "./scene";
import type { Clock } from "./clock";

type Flier = { spec: DroneSpec; sig: string; root: Container; parts: DroneParts; pillar: Sprite | undefined };

const clamp = (n: number) => Math.min(1, Math.max(0, n));

export class Fliers {
  private nodes = new Map<string, Flier>();

  constructor(private readonly layer: Container) {
    layer.sortableChildren = true;
  }

  /** The specs as they stand now, for what draws the ground under them. */
  get specs(): readonly DroneSpec[] {
    return [...this.nodes.values()].map((one) => one.spec);
  }

  sync(scene: PixiSceneIn, palette: Palette, fonts: number): void {
    const next = new Map<string, Flier>();
    for (const spec of scene.drones) {
      const sig = `${droneSig(spec)}:${fonts}:${scene.big}`;
      const kept = this.nodes.get(spec.drone.key);
      if (kept !== undefined && kept.sig === sig) {
        kept.spec = spec;
        next.set(spec.drone.key, kept);
        continue;
      }
      kept?.root.destroy({ children: true });
      const parts = drawDrone(spec, scene.big, palette);
      const root = new Container();
      root.eventMode = "none";
      const tone = spec.call;
      const pillar = tone === undefined ? undefined : pillarSprite(12, 100, tone === "bad" ? palette.bad : palette.ask);
      if (pillar !== undefined) root.addChild(pillar);
      root.addChild(parts.body);
      if (parts.mark !== undefined) root.addChild(parts.mark);
      for (const one of parts.clashes) root.addChild(one);
      this.layer.addChild(root);
      next.set(spec.drone.key, { spec, sig, root, parts, pillar });
    }
    for (const [key, one] of this.nodes) if (!next.has(key)) one.root.destroy({ children: true });
    this.nodes = next;
  }

  /** Where each drone is on the screen at this view. `floorOf` is the height a plot stands at now. */
  place(view: View, floorOf: (tile: string | undefined) => number, clock: Clock): void {
    if (view.width === 0) return;
    for (const { spec, root, parts, pillar } of this.nodes.values()) {
      const at = { x: spec.at.x, y: spec.at.y, z: spec.alt };
      let seen = 1;
      let arrived = 1;
      if (spec.launch !== undefined && clock.pulse > 0) {
        const since = clock.since(spec.launch.id, spec.launch.delay);
        const t = clamp(since / (clock.pulse * 1.2));
        const from = spec.launch.from;
        // From its pad, on an arc that rises 46 above the line between, to its altitude over its plot.
        const lifted = alongOf(from, at, 46, t);
        at.x = lifted.x;
        at.y = lifted.y;
        at.z = lifted.z;
        seen = since < 0 ? 0 : clamp(since / (clock.pulse * 0.35));
        arrived = clamp((since - clock.pulse * 0.7) / (clock.pulse * 0.5));
      }
      const here = project(view, at);
      root.visible = here.scale > 0 && seen > 0;
      root.zIndex = spec.at.y;
      parts.body.position.set(here.x, here.y);
      parts.body.scale.set(view.eye * here.scale);
      parts.body.alpha = alphaOf(spec) * seen;
      if (parts.mark !== undefined) {
        parts.mark.position.set(here.x, here.y - parts.head * view.eye * here.scale);
        parts.mark.scale.set(here.scale);
      }
      parts.clashes.forEach((one, i) => {
        one.position.set(here.x + (32 + 30 * i) * here.scale, here.y - parts.head * view.eye * here.scale);
        one.scale.set(here.scale);
      });
      if (pillar !== undefined) {
        const floor = floorOf(spec.tile);
        const up = Math.max(0, spec.alt - floor) + parts.head;
        const foot = project(view, { x: spec.at.x, y: spec.at.y, z: floor });
        const head = project(view, { x: spec.at.x, y: spec.at.y, z: floor + up + 90 / view.eye });
        pillar.position.set(foot.x, foot.y);
        pillar.rotation = Math.atan2(head.x - foot.x, foot.y - head.y);
        pillar.height = Math.hypot(head.x - foot.x, head.y - foot.y);
        pillar.width = 12 * foot.scale;
        pillar.alpha = (spec.leaving ? 0.35 : 1) * arrived;
      }
    }
  }

  destroy(): void {
    for (const one of this.nodes.values()) one.root.destroy({ children: true });
    this.nodes.clear();
  }
}
