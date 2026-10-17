// The map's hover: a HUD callout. A short dotted leader runs from the thing out to a card beside it, up and to the right by default,
// flipping left or down at the pane's edges so it never clips and never lies over what it is about (`callout.ts` places it). It is
// drawn in screen space, outside the tilted scene, and follows its anchor as the camera moves. Ported from
// proto/cockpit-flight-deck. #2301.

import { useLayoutEffect, useRef } from "react";

import { CircleDot, FolderGit2, Flag, GitMerge, KeyRound, Layers, ListChecks, Split, SquareTerminal, TriangleAlert, type LucideIcon } from "lucide-react";
import { InspectCard } from "@armada/components";

import { AskUpdate } from "./AskUpdate";
import { BuildCard } from "./BuildCard";
import { placeCallout, type Anchored } from "./placement";
import type { Tip } from "./tip";

/** The glyph of each kind of thing the map says something about. */
const KIND_GLYPH: Record<string, LucideIcon | undefined> = { Job: SquareTerminal, Main: GitMerge, Bay: KeyRound, Task: ListChecks, Road: Flag, Phase: Layers, Subagent: Split, Repository: FolderGit2, "Write clash": TriangleAlert };

/** The kinds a press on the focused thing opens. */
const OPENS: ReadonlySet<string> = new Set(["Job", "Bay", "Main"]);

/** The status token a state's tone is drawn in. */
const STATE_TONE = { calm: "var(--fg-muted)", ask: "var(--status-awaiting-review)", bad: "var(--status-completed-failed)", good: "var(--status-completed-success)" } as const;

/** What the callout hangs from: the element hovered or focused, or a point where the pointer was for a thing too large to have a middle. */
export type Anchor = { target: Element; at?: { x: number; y: number } };

/** The box on the screen the callout keeps clear of: the drone's hit box, the pad's hit box, the flag's; a point, where the pointer was. */
function boxOf({ target, at }: Anchor): Anchored | undefined {
  if (at !== undefined) return { x: at.x, y: at.y, w: 8, h: 8 };
  if (!target.isConnected) return undefined;
  const r = target.getBoundingClientRect();
  return r.width === 0 && r.height === 0 ? undefined : { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width, h: r.height };
}

/**
 * How long the callout looks for its anchor to move every frame after it last did, and then how often it looks. Reading where the host and the
 * anchor are forces a layout, and a card standing still on a map that is not moving was doing it sixty times a second. Anything that moves the
 * anchor on purpose (`view`, the camera; the host resizing) wakes it at once.
 */
const SETTLED_FRAMES = 30;
const HEARTBEAT_MS = 250;

/** `pinned` is one a press opened: it stays, and what is in it can be pressed. `view` is whatever changes when the camera does. */
export function Callout({ tip, anchor, host, pinned, view }: { tip: Tip; anchor: Anchor; host: HTMLElement; pinned?: boolean; view?: unknown }) {
  const root = useRef<HTMLDivElement>(null);
  const card = useRef<HTMLDivElement>(null);
  const slot = useRef<HTMLDivElement>(null);
  const line = useRef<SVGLineElement>(null);
  const dot = useRef<SVGCircleElement>(null);

  const wake = useRef<() => void>(() => undefined);

  useLayoutEffect(() => {
    let frame = 0;
    let beat = 0;
    let still = 0;
    let seen = "";
    const place = (): boolean => {
      let moved = false;
      const base = host.getBoundingClientRect();
      const from = boxOf(anchor);
      const el = card.current;
      const moves = slot.current;
      if (from !== undefined && el !== null && moves !== null && line.current !== null && dot.current !== null && root.current !== null) {
        const a = { x: from.x - base.left, y: from.y - base.top, w: from.w, h: from.h };
        const put = placeCallout(a, { w: el.offsetWidth, h: el.offsetHeight }, { width: base.width, height: base.height });
        const key = [put.left, put.top, put.end.x, put.end.y, a.x, a.y].map((n) => Math.round(n)).join();
        if (key !== seen) {
          seen = key;
          moved = true;
          // On the wrapper, so the card's own entry animation is the only one on the card.
          moves.style.transform = `translate(${put.left.toFixed(1)}px, ${put.top.toFixed(1)}px)`;
          moves.style.setProperty("--ox", `${(a.x - put.left).toFixed(1)}px`);
          moves.style.setProperty("--oy", `${(a.y - put.top).toFixed(1)}px`);
          line.current.setAttribute("x1", put.start.x.toFixed(1));
          line.current.setAttribute("y1", put.start.y.toFixed(1));
          line.current.setAttribute("x2", put.end.x.toFixed(1));
          line.current.setAttribute("y2", put.end.y.toFixed(1));
          dot.current.setAttribute("cx", put.end.x.toFixed(1));
          dot.current.setAttribute("cy", put.end.y.toFixed(1));
          root.current.dataset.side = `${put.right ? "right" : "left"} ${put.up ? "up" : "down"}`;
        }
        if (root.current.dataset.placed === undefined) root.current.dataset.placed = "";
      }
      return moved;
    };
    const step = () => {
      still = place() ? 0 : still + 1;
      if (still < SETTLED_FRAMES) frame = window.requestAnimationFrame(step);
      else beat = window.setTimeout(step, HEARTBEAT_MS);
    };
    const wakes = () => {
      still = 0;
      window.cancelAnimationFrame(frame);
      window.clearTimeout(beat);
      frame = window.requestAnimationFrame(step);
    };
    wake.current = wakes;
    place();
    frame = window.requestAnimationFrame(step);
    const watch = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(wakes);
    watch?.observe(host);
    return () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(beat);
      watch?.disconnect();
    };
  }, [tip, anchor, host]);

  // The camera moved: the anchor is somewhere else now, and the next frames find where.
  useLayoutEffect(() => wake.current(), [view]);

  const most = Math.max(1, ...(tip.ticks ?? [0]));
  return (
    <div ref={root} className="bf-callout" data-hue={tip.hue} data-pinned={pinned || undefined} data-node={tip.node !== undefined || undefined} role={pinned ? "group" : "tooltip"} aria-label={pinned ? tip.title : undefined}>
      <svg className="bf-callout__lead" aria-hidden="true">
        <g>
          <line ref={line} />
          <circle ref={dot} r={2.5} />
        </g>
      </svg>
      <div ref={slot} className="bf-callout__place">
      <InspectCard
        ref={card}
        className="bf-callout__card"
        kind={{ Glyph: KIND_GLYPH[tip.label] ?? CircleDot, label: tip.label }}
        {...(tip.node !== undefined ? {} : { title: tip.title })}
        {...(tip.node !== undefined || tip.state === undefined ? {} : { state: { Glyph: tip.state.Glyph, said: tip.state.said, tone: STATE_TONE[tip.state.tone] } })}
        tone="var(--tip)"
        keys={OPENS.has(tip.label) ? [{ key: "Enter", verb: "Open" }] : []}
      >
        {pinned && tip.ask !== undefined && tip.build !== undefined ? <AskUpdate subject={tip.ask} /> : null}
        {tip.node !== undefined ? <div className="bf-callout__node">{tip.node}</div> : null}
        {tip.doing === undefined ? null : (
          <div className="bf-callout__doing">
            <tip.doing.Glyph size={16} strokeWidth={2} aria-hidden="true" />
            {tip.doing.word}
            {tip.doing.via === undefined ? null : <span className="bf-callout__path">{tip.doing.via}</span>}
          </div>
        )}
        {tip.build === undefined ? null : <BuildCard build={tip.build} />}
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
      </InspectCard>
      </div>
    </div>
  );
}
