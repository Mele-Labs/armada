// The map's hover: a HUD callout. A short dotted leader runs from the thing out to a card beside it, up and to the right by default,
// flipping left or down at the pane's edges so it never clips and never lies over what it is about (`callout.ts` places it). It is
// drawn in screen space, outside the tilted scene, and follows its anchor as the camera moves. Ported from
// proto/cockpit-flight-deck. #2301.

import { useLayoutEffect, useRef } from "react";

import { placeCallout, type Anchored } from "./callout";
import type { Tip } from "./tip";

/** What the callout hangs from: the element hovered or focused, or a point where the pointer was for a thing too large to have a middle. */
export type Anchor = { target: Element; at?: { x: number; y: number } };

/** The box on the screen the callout keeps clear of: the drone's hit box, the pad's ring, the flag's rim; a point, where the pointer was. */
function boxOf({ target, at }: Anchor): Anchored | undefined {
  if (at !== undefined) return { x: at.x, y: at.y, w: 8, h: 8 };
  if (!target.isConnected) return undefined;
  const one = target.querySelector(".bf-hit") ?? target;
  const r = one.getBoundingClientRect();
  return r.width === 0 && r.height === 0 ? undefined : { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, h: r.height };
}

/** `pinned` is one a press opened: it stays, and what is in it can be pressed. */
export function Callout({ tip, anchor, host, pinned }: { tip: Tip; anchor: Anchor; host: HTMLElement; pinned?: boolean }) {
  const root = useRef<HTMLDivElement>(null);
  const card = useRef<HTMLDivElement>(null);
  const line = useRef<SVGLineElement>(null);
  const dot = useRef<SVGCircleElement>(null);

  useLayoutEffect(() => {
    let frame = 0;
    let seen = "";
    const place = () => {
      const base = host.getBoundingClientRect();
      const from = boxOf(anchor);
      const el = card.current;
      if (from !== undefined && el !== null && line.current !== null && dot.current !== null && root.current !== null) {
        const a = { x: from.x - base.left, y: from.y - base.top, w: from.w, h: from.h };
        const put = placeCallout(a, { w: el.offsetWidth, h: el.offsetHeight }, { width: base.width, height: base.height });
        const key = [put.left, put.top, put.end.x, put.end.y, a.x, a.y].map((n) => Math.round(n)).join();
        if (key !== seen) {
          seen = key;
          el.style.transform = `translate(${put.left.toFixed(1)}px, ${put.top.toFixed(1)}px)`;
          el.style.setProperty("--ox", `${(a.x - put.left).toFixed(1)}px`);
          el.style.setProperty("--oy", `${(a.y - put.top).toFixed(1)}px`);
          line.current.setAttribute("x1", put.start.x.toFixed(1));
          line.current.setAttribute("y1", put.start.y.toFixed(1));
          line.current.setAttribute("x2", put.end.x.toFixed(1));
          line.current.setAttribute("y2", put.end.y.toFixed(1));
          dot.current.setAttribute("cx", put.end.x.toFixed(1));
          dot.current.setAttribute("cy", put.end.y.toFixed(1));
          root.current.dataset.side = `${put.right ? "right" : "left"} ${put.up ? "up" : "down"}`;
        }
        root.current.dataset.placed = "";
      }
      frame = window.requestAnimationFrame(place);
    };
    place();
    return () => window.cancelAnimationFrame(frame);
  }, [tip, anchor, host]);

  const most = Math.max(1, ...(tip.ticks ?? [0]));
  return (
    <div ref={root} className="bf-callout" data-hue={tip.hue} data-pinned={pinned || undefined} data-node={tip.node !== undefined || undefined} role={pinned ? "group" : "tooltip"} aria-label={pinned ? tip.title : undefined}>
      <svg className="bf-callout__lead" aria-hidden="true">
        <g>
          <line ref={line} />
          <circle ref={dot} r={2.5} />
        </g>
      </svg>
      <div ref={card} className="bf-callout__card">
        <i className="bf-callout__glow" aria-hidden="true" />
        <div className="bf-callout__kind">{tip.label}</div>
        {tip.node !== undefined ? <div className="bf-callout__node">{tip.node}</div> : <div className="bf-callout__title">{tip.title}</div>}
        {tip.node !== undefined || tip.state === undefined ? null : (
          <div className="bf-callout__state" data-tone={tip.state.tone}>
            <tip.state.Glyph size={16} strokeWidth={2} aria-hidden="true" />
            {tip.state.said}
          </div>
        )}
        {tip.doing === undefined ? null : (
          <div className="bf-callout__doing">
            <tip.doing.Glyph size={16} strokeWidth={2} aria-hidden="true" />
            {tip.doing.word}
            {tip.doing.via === undefined ? null : <span className="bf-callout__path">{tip.doing.via}</span>}
          </div>
        )}
        {tip.lines === undefined ? null : tip.lines.map((one) => <div key={one} className="bf-callout__line">{one}</div>)}
        {tip.pills === undefined && tip.bay === undefined ? null : (
          <ul className="bf-callout__pills">
            {tip.bay === undefined ? null : (
              <li className="bf-callout__pill" data-bay>
                <tip.bay.Glyph size={12} strokeWidth={2} aria-hidden="true" />
                slot-{tip.bay.slot}
              </li>
            )}
            {(tip.pills ?? []).map((one) => (
              <li key={one} className="bf-callout__pill">
                {one}
              </li>
            ))}
          </ul>
        )}
        {tip.last === undefined ? null : (
          <div className="bf-callout__last">
            <span>{tip.last.says}</span>
            <time>{tip.last.ago}</time>
          </div>
        )}
        {tip.call === undefined ? null : (
          <div className="bf-callout__call">
            <i aria-hidden="true" />
            {tip.call}
          </div>
        )}
        {tip.ticks === undefined ? null : (
          <svg className="bf-callout__ticks" viewBox={`0 0 ${tip.ticks.length * 6} 14`} aria-hidden="true" preserveAspectRatio="none">
            {tip.ticks.map((n, i) => (
              <rect key={i} x={i * 6} y={14 - (n === 0 ? 2 : 4 + (n / most) * 10)} width={4} height={n === 0 ? 2 : 4 + (n / most) * 10} rx={1} data-empty={n === 0 || undefined} />
            ))}
          </svg>
        )}
      </div>
    </div>
  );
}
