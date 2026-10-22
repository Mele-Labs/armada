// What a Job's Field lens puts over the canvas: the card at the top right and a dashed line from it to the slab it shows, and over each
// other lifted file its counts. Placed from the canvas's view each time the camera moves or a slab changes, never on a loop.

import { useCallback, useEffect, useLayoutEffect, useRef, type ReactNode } from "react";

import { cornerOf, unitOf } from "./pixi/lens";
import type { View } from "./pixi/project";
import type { LensSpec } from "./pixi/scene";
import type { Stage } from "./pixi/stage";

const base = (path: string): string => path.split("/").pop() || path;

export function LensWires({ stage, specs, card }: { stage: Stage | undefined; specs: readonly LensSpec[] | undefined; card: ReactNode | undefined }) {
  const box = useRef<HTMLDivElement>(null);
  const held = useRef<HTMLDivElement>(null);
  const line = useRef<SVGLineElement>(null);
  const counts = useRef(new Map<string, HTMLLIElement>());
  const shown = specs?.find((one) => one.card);
  const others = (specs ?? []).filter((one) => !one.card);

  const place = useCallback(
    (view: View) => {
      if (stage === undefined || view.width === 0 || box.current === null) return;
      const host = box.current.getBoundingClientRect();
      const wire = line.current;
      const front = held.current?.querySelector("[data-lens-front]");
      if (wire !== null) {
        const at = shown === undefined || front == null ? undefined : stage.project(cornerOf(shown, stage.floorOf(shown.tile), unitOf(view.eye)));
        if (at === undefined || at.scale <= 0 || front == null) wire.style.display = "none";
        else {
          const rect = front.getBoundingClientRect();
          wire.style.display = "";
          wire.setAttribute("x1", at.x.toFixed(1));
          wire.setAttribute("y1", at.y.toFixed(1));
          wire.setAttribute("x2", (rect.left - host.left).toFixed(1));
          wire.setAttribute("y2", (rect.top - host.top + rect.height / 2).toFixed(1));
        }
      }
      for (const spec of others) {
        const el = counts.current.get(spec.path);
        if (el === undefined) continue;
        const corner = cornerOf(spec, stage.floorOf(spec.tile), unitOf(view.eye));
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
        <div ref={held} className="bf-lens__card">
          {card}
        </div>
      )}
    </div>
  );
}
