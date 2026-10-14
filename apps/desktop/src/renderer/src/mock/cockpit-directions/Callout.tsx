// The map's hover: a HUD callout. A short dotted leader runs from the thing out to a card beside it, up and to the right
// by default, flipping left or down at the pane's edges so it never clips and never lies over what it is about. It is
// drawn in screen space, outside the tilted scene, and follows its anchor as the camera moves. Mock only.
// proto/cockpit-flight-deck.

import { useLayoutEffect, useRef, type ComponentType, type ReactNode } from "react";

export type Glyph = ComponentType<{ size?: number; strokeWidth?: number; "aria-hidden"?: boolean }>;
/** What colours the card's top edge and its glow. Quiet is gray: a thing with no state to speak of. */
export type TipHue = "calm" | "ask" | "bad" | "quiet" | "good";

export type Tip = {
  /** The kind, in small caps: Job, Session, Main, Bay. */
  label: string;
  title: string;
  hue: TipHue;
  state?: { Glyph: Glyph; said: string; tone: "calm" | "ask" | "bad" | "good" };
  /** What it is doing over its plot, as the tool events say: a glyph, the word, and the plot. */
  doing?: { Glyph: Glyph; word: string; path?: string; via?: string };
  /** The directories it touches, the biggest two. */
  pills?: readonly string[];
  /** The bay it holds. */
  bay?: { slot: number; Glyph: Glyph; word: string };
  /** What it last did, and how long ago. */
  last?: { says: string; ago: string };
  /** The call it holds, in one line. */
  call?: string;
  /** The last hour of its events, oldest first, one bar for each five minutes. */
  ticks?: readonly number[];
  lines?: readonly string[];
  /** A card of its own, drawn under the kind in place of the title and the state: a step's real node card. */
  node?: ReactNode;
};

/** What the callout hangs from: the element hovered or focused, or a point where the pointer was for a thing too large to have a middle. */
export type Anchor = { target: Element; at?: { x: number; y: number } };

const EDGE = 10;
const GAP = 44;

/** The box on the screen the callout keeps clear of: the drone's hit box, the pad's ring, the flag's rim; a point, where the pointer was. */
function boxOf({ target, at }: Anchor): { x: number; y: number; w: number; h: number } | undefined {
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
  const lead = useRef<SVGGElement>(null);
  const line = useRef<SVGLineElement>(null);
  const dot = useRef<SVGCircleElement>(null);

  useLayoutEffect(() => {
    let frame = 0;
    let seen = "";
    const place = () => {
      const base = host.getBoundingClientRect();
      const from = boxOf(anchor);
      const el = card.current;
      if (from !== undefined && el !== null && lead.current !== null && line.current !== null && dot.current !== null && root.current !== null) {
        const [cw, ch] = [el.offsetWidth, el.offsetHeight];
        const a = { x: from.x - base.left, y: from.y - base.top };
        const [hw, hh] = [from.w / 2, from.h / 2];
        // Up and right unless that runs off the pane: then the side that has room.
        const right = a.x + hw + GAP + cw <= base.width - EDGE || (a.x - hw - GAP - cw < EDGE && base.width - a.x >= a.x);
        const up = a.y - hh - GAP - ch >= EDGE || (a.y + hh + GAP + ch > base.height - EDGE && a.y >= base.height - a.y);
        const clamp = (n: number, low: number, high: number) => Math.min(Math.max(n, low), Math.max(low, high));
        const left = clamp(right ? a.x + hw + GAP : a.x - hw - GAP - cw, EDGE, base.width - cw - EDGE);
        const top = clamp(up ? a.y - hh - GAP - ch : a.y + hh + GAP, EDGE, base.height - ch - EDGE);
        // The leader ends at the card's near corner, or on its near edge where the card has been pushed alongside.
        const level = a.y >= top && a.y <= top + ch;
        const end = { x: right ? left : left + cw, y: level ? clamp(a.y, top + 12, top + ch - 12) : up ? top + ch : top };
        const length = Math.hypot(end.x - a.x, end.y - a.y) || 1;
        const lift = Math.min(22, length / 3);
        const start = { x: a.x + ((end.x - a.x) / length) * lift, y: a.y + ((end.y - a.y) / length) * lift };
        const key = [left, top, end.x, end.y, a.x, a.y].map((n) => Math.round(n)).join();
        if (key !== seen) {
          seen = key;
          el.style.transform = `translate(${left.toFixed(1)}px, ${top.toFixed(1)}px)`;
          el.style.setProperty("--ox", `${(a.x - left).toFixed(1)}px`);
          el.style.setProperty("--oy", `${(a.y - top).toFixed(1)}px`);
          line.current.setAttribute("x1", start.x.toFixed(1));
          line.current.setAttribute("y1", start.y.toFixed(1));
          line.current.setAttribute("x2", end.x.toFixed(1));
          line.current.setAttribute("y2", end.y.toFixed(1));
          dot.current.setAttribute("cx", end.x.toFixed(1));
          dot.current.setAttribute("cy", end.y.toFixed(1));
          root.current.dataset.side = `${right ? "right" : "left"} ${up ? "up" : "down"}`;
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
        <g ref={lead}>
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
            <tip.state.Glyph size={16} strokeWidth={2} aria-hidden />
            {tip.state.said}
          </div>
        )}
        {tip.doing === undefined ? null : (
          <div className="bf-callout__doing">
            <tip.doing.Glyph size={16} strokeWidth={2} aria-hidden />
            {tip.doing.word}
            {tip.doing.path === undefined ? null : <span className="bf-callout__path">{tip.doing.path}</span>}
          </div>
        )}
        {tip.lines === undefined ? null : tip.lines.map((one) => <div key={one} className="bf-callout__line">{one}</div>)}
        {tip.pills === undefined && tip.bay === undefined ? null : (
          <ul className="bf-callout__pills">
            {tip.bay === undefined ? null : (
              <li className="bf-callout__pill" data-bay>
                <tip.bay.Glyph size={12} strokeWidth={2} aria-hidden />
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
