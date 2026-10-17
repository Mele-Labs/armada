// The subagents that circle a Session's ring: each a small drone, the Session's plus quad at half size, flying round the ring at its own
// radius and height while it has a loop to spend, and standing at its place on the ring otherwise. Where one is at a moment is worked out
// here from the clock alone, so the drawing and the hit box over it agree.

import { Container } from "pixi.js";

import type { P3 } from "../layout";
import type { Clock } from "./clock";
import { body } from "./drones";
import type { Palette } from "./palette";
import { project, type View } from "./project";
import type { PixiSceneIn, WorkerSpec } from "./scene";

/** Where a subagent is at `now`: round its ring at the angle its lap has reached, `floor` being the height of the plot under the ring. With no clock it stands at its phase. */
export function workerAt(w: WorkerSpec, floor: number, now: number | undefined): P3 {
  const lapping = w.loop && now !== undefined;
  const lap = lapping ? ((now / w.period + w.phase) % 1 + 1) % 1 : w.phase % 1;
  const turn = (lapping && w.rev ? 1 - lap : lap) * 2 * Math.PI;
  return { x: w.anchor.x + w.radius * Math.cos(turn), y: w.anchor.y + w.radius * Math.sin(turn), z: floor + 1.5 + 26 + 6 * w.i };
}

type Flier = { spec: WorkerSpec; sig: string; root: Container };

export class Workers {
  private nodes = new Map<string, Flier>();

  constructor(private readonly layer: Container) {}

  sync(scene: PixiSceneIn, palette: Palette, fonts: number): void {
    const next = new Map<string, Flier>();
    for (const spec of scene.workers) {
      const sig = `${fonts}:${scene.big}`;
      const kept = this.nodes.get(spec.key);
      if (kept !== undefined && kept.sig === sig) {
        kept.spec = spec;
        next.set(spec.key, kept);
        continue;
      }
      kept?.root.destroy({ children: true });
      const root = new Container();
      root.eventMode = "none";
      const glyph = body("session", false, palette.muted, palette);
      glyph.scale.set(0.5 * scene.big);
      root.addChild(glyph);
      this.layer.addChild(root);
      next.set(spec.key, { spec, sig, root });
    }
    for (const [key, one] of this.nodes) if (!next.has(key)) one.root.destroy({ children: true });
    this.nodes = next;
  }

  /** Puts each at its place on the screen at this view. `floorOf` is the height a plot stands at now. */
  place(view: View, floorOf: (tile: string | undefined) => number, clock: Clock): void {
    if (view.width === 0) return;
    for (const { spec, root } of this.nodes.values()) {
      const at = workerAt(spec, floorOf(spec.tile), clock.pulse > 0 ? clock.now : undefined);
      const here = project(view, at);
      root.visible = here.scale > 0;
      root.zIndex = at.y;
      root.position.set(here.x, here.y);
      root.scale.set(view.eye * here.scale);
    }
  }

  destroy(): void {
    for (const one of this.nodes.values()) one.root.destroy({ children: true });
    this.nodes.clear();
  }
}
