// What a Job's Field lens puts over the canvas: the card at the top right and a dashed line from it to the slab it shows, and over each
// other lifted file its counts. Placed from the canvas's view each time the camera moves or a slab changes, never on a loop. The card
// never moves the camera: where the card's drone or slab stands under it, the card stands at the top left instead.

import { useCallback, useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

import type { Box } from "./camera";
import type { P3 } from "./layout";
import { cornerOf, slabMarks, unitOf } from "./lens";
import type { View } from "./pixi/project";
import type { LensSpec } from "./pixi/scene";
import type { Stage } from "./pixi/stage";

const base = (path: string): string => path.split("/").pop() || path;

/** Whether any of the points lies inside the box. */
export const covers = (box: Box, points: readonly { x: number; y: number }[]): boolean => points.some((one) => one.x >= box.l && one.x <= box.r && one.y >= box.t && one.y <= box.b);

/**
 * Which top corner the card stands in: the right, unless the right would cover the card's drone or slab and the left would not. Once
 * moved it stays left until the left covers them and the right does not, so a pan does not toss it from side to side.
 */
export function sideOf(was: "right" | "left", right: Box, left: Box, points: readonly { x: number; y: number }[]): "right" | "left" {
  const [r, l] = [covers(right, points), covers(left, points)];
  if (was === "right") return r && !l ? "left" : "right";
  return l && !r ? "right" : "left";
}

export function LensWires({ stage, specs, card, writer }: { stage: Stage | undefined; specs: readonly LensSpec[] | undefined; card: ReactNode | undefined; writer: P3 | undefined }) {
  const box = useRef<HTMLDivElement>(null);
  const held = useRef<HTMLDivElement>(null);
  const line = useRef<SVGLineElement>(null);
  const counts = useRef(new Map<string, HTMLLIElement>());
  const side = useRef<"right" | "left">("right");
  const gaps = useRef({ right: 0, left: 0 });
  const flying = useRef(writer);
  flying.current = writer;
  const shown = specs?.find((one) => one.card);
  const others = (specs ?? []).filter((one) => !one.card);

  const place = useCallback(
    (view: View) => {
      if (stage === undefined || view.width === 0 || box.current === null) return;
      const host = box.current.getBoundingClientRect();
      const wire = line.current;
      const wrap = held.current;
      const front = wrap?.querySelector("[data-lens-front]");
      const unit = unitOf(view.eye);
      if (wrap == null || front == null || shown === undefined) {
        if (wire !== null) wire.style.display = "none";
      } else {
        const rect = wrap.getBoundingClientRect();
        const [w, t, h] = [rect.width, rect.top - host.top, rect.height];
        const style = getComputedStyle(wrap);
        // Each corner's inset as the stylesheet sets it, read while the card stands there.
        if (side.current === "right") gaps.current.right = Number.parseFloat(style.right) || gaps.current.right;
        else gaps.current.left = Number.parseFloat(style.left) || gaps.current.left;
        const right: Box = { l: host.width - gaps.current.right - w, t, r: host.width - gaps.current.right, b: t + h };
        // The left inset is the top's until it has been read: the stylesheet sets both to one space.
        const inset = gaps.current.left || t;
        const left: Box = { l: inset, t, r: inset + w, b: t + h };
        const floor = stage.floorOf(shown.tile);
        const points = [...slabMarks(shown, floor, unit), ...(flying.current === undefined ? [] : [flying.current])].map((one) => stage.project(one));
        const now = sideOf(side.current, right, left, points);
        if (now !== side.current) {
          side.current = now;
          if (now === "left") wrap.dataset.side = "left";
          else delete wrap.dataset.side;
        }
        if (wire !== null) {
          const at = stage.project(cornerOf(shown, floor, unit));
          const face = front.getBoundingClientRect();
          wire.style.display = at.scale <= 0 ? "none" : "";
          wire.setAttribute("x1", at.x.toFixed(1));
          wire.setAttribute("y1", at.y.toFixed(1));
          wire.setAttribute("x2", ((side.current === "left" ? face.right : face.left) - host.left).toFixed(1));
          wire.setAttribute("y2", (face.top - host.top + face.height / 2).toFixed(1));
        }
      }
      for (const spec of others) {
        const el = counts.current.get(spec.path);
        if (el === undefined) continue;
        const corner = cornerOf(spec, stage.floorOf(spec.tile), unit);
        const at = stage.project({ ...spec.at, z: corner.z });
        el.style.display = at.scale <= 0 ? "none" : "";
        el.style.left = `${at.x.toFixed(1)}px`;
        el.style.top = `${(at.y - spec.d * 0.3).toFixed(1)}px`;
      }
    },
    [stage, shown, others.map((one) => one.path).join("|")],
  );

  useEffect(() => stage?.onView(place), [stage, place]);
  useLayoutEffect(() => {
    if (stage !== undefined) place(stage.shown);
  });

  return (
    <div ref={box} className="bf-lens" aria-hidden={specs === undefined || specs.length === 0 ? true : undefined}>
      <svg className="bf-lens__wire" aria-hidden="true">
        <line ref={line} style={{ display: "none" }} />
      </svg>
      {others.length === 0 ? null : (
        <ul className="bf-lens__counts" aria-label="Files being written">
          {others.map((spec) => (
            <li
              key={spec.path}
              ref={(el) => {
                if (el === null) counts.current.delete(spec.path);
                else counts.current.set(spec.path, el);
              }}
              aria-label={`${base(spec.path)} ${spec.counts}`}
            >
              {spec.counts}
            </li>
          ))}
        </ul>
      )}
      {card === undefined ? null : (
        <div ref={held} className="bf-lens__card" data-side={side.current === "left" ? "left" : undefined}>
          {card}
        </div>
      )}
    </div>
  );
}
