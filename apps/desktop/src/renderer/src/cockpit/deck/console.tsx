// The Cockpit's flight controls drawn as a console, for the mock walk `cockpit-console`. Three takes, picked by `?take=arc|wrap|dash` (`mock/main.tsx`
// puts it on the root). Nothing sets it in the app, so nothing here draws there. To drop the experiment: delete this file, `console.css` and the
// walk, the `take` lines in `mock/main.tsx`, and the lines in `Deck.tsx` that name `take`, `ConsoleBlock` and `DispatchKey`.

import { useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent, ReactNode, RefObject } from "react";

export type ConsoleTake = "arc" | "wrap" | "dash";

const TAKES: readonly string[] = ["arc", "wrap", "dash"];

/** The take the mock asked for, and nothing in the app. */
export function consoleTake(): ConsoleTake | undefined {
  const asked = document.documentElement.dataset.consoleTake;
  return asked !== undefined && TAKES.includes(asked) ? (asked as ConsoleTake) : undefined;
}

/** The panel's outline in pixels, rounded at its corners, with `top` for its top edge from the one top corner to the other. */
function outline(width: number, height: number, corner: number, top: string): string {
  const r = corner;
  return `M${r},0 ${top} A${r},${r} 0 0 1 ${width},${r} L${width},${height - r} A${r},${r} 0 0 1 ${width - r},${height} L${r},${height} A${r},${r} 0 0 1 0,${height - r} L0,${r} A${r},${r} 0 0 1 ${r},0 Z`;
}

/** The arc's top edge, a parabola from one corner to the other that is `rise` deep at the middle, and its slope. */
function arcAt(x: number, width: number, corner: number, rise: number): { y: number; slope: number } {
  const span = width - 2 * corner;
  const u = x - corner;
  return { y: (4 * rise * u * (span - u)) / (span * span), slope: (4 * rise * (span - 2 * u)) / (span * span) };
}

/**
 * Draws the take on its panel, after each layout: the outline as a rim, the same path as the clip of the panel's glass (`--console-clip`), and each
 * key's seat moved onto the edge (`--lift`) and turned to it (`--tilt`).
 */
function useEdge(take: ConsoleTake, block: RefObject<HTMLDivElement | null>): { width: number; height: number; d: string } | undefined {
  const [edge, setEdge] = useState<{ width: number; height: number; d: string }>();
  useLayoutEffect(() => {
    const here = block.current;
    const panel = here?.parentElement;
    if (here === null || here === undefined || panel === null || panel === undefined || take === "wrap") return;
    const draw = () => {
      const width = panel.clientWidth;
      const height = panel.clientHeight;
      const box = panel.getBoundingClientRect();
      const left = box.left + panel.clientLeft;
      const corner = Math.min(parseFloat(getComputedStyle(panel).borderTopLeftRadius) || 0, height / 2);
      const rise = parseFloat(getComputedStyle(here).getPropertyValue("--console-rise")) || 0;
      const seats = [...here.querySelectorAll<HTMLElement>(".deck-console__seat")];
      let top = "";
      if (take === "arc") {
        top = `Q${width / 2},${2 * rise} ${width - corner},0`;
        for (const seat of seats) {
          const at = seat.getBoundingClientRect();
          const { y, slope } = arcAt((at.left + at.right) / 2 - left, width, corner, rise);
          seat.style.setProperty("--lift", `${y - rise}px`);
          seat.style.setProperty("--tilt", `${Math.atan(slope)}rad`);
        }
      } else {
        const core = here.querySelector<HTMLElement>(".deck-console__core")!.getBoundingClientRect();
        const centre = here.querySelector<HTMLElement>(".deck-console__core > .deck-console__seat")?.getBoundingClientRect() ?? core;
        const [a, b, c, e] = [core.left - left, centre.left - left, centre.right - left, core.right - left];
        top = `L${a},0 C${(a + b) / 2},0 ${(a + b) / 2},${rise} ${b},${rise} L${c},${rise} C${(c + e) / 2},${rise} ${(c + e) / 2},0 ${e},0 L${width - corner},0`;
        for (const seat of seats) seat.style.setProperty("--lift", seat.parentElement?.classList.contains("deck-console__core") ? "0px" : `${-rise}px`);
      }
      const d = outline(width, height, corner, top);
      panel.style.setProperty("--console-clip", `path("${d}")`);
      setEdge((was) => (was?.d === d ? was : { width, height, d }));
    };
    draw();
    const watch = new ResizeObserver(draw);
    watch.observe(panel);
    return () => {
      watch.disconnect();
      panel.style.removeProperty("--console-clip");
    };
  }, [take, block]);
  return take === "wrap" ? undefined : edge;
}

/** Arrows and Home and End move along the keys from the one the key was pressed on, and Enter presses it. Tab already reaches each. */
function along(event: KeyboardEvent<HTMLElement>) {
  const keys = [...event.currentTarget.querySelectorAll<HTMLElement>(".deck-tile")];
  const at = keys.indexOf((event.target as HTMLElement).closest<HTMLElement>(".deck-tile")!);
  if (at < 0) return;
  if (event.key === "Enter") {
    event.preventDefault();
    keys[at]!.click();
    return;
  }
  const to = event.key === "ArrowRight" ? at + 1 : event.key === "ArrowLeft" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? keys.length - 1 : undefined;
  if (to === undefined) return;
  event.preventDefault();
  keys[Math.min(Math.max(to, 0), keys.length - 1)]!.focus();
}

/**
 * The deck's tiles in three groups, Dispatch in the middle one: `keys` split in half either side of it. Each key sits in a seat, which `arc` and
 * `dash` move onto the panel's top edge. The groups are boxes only in `wrap`, where they are the console's sections.
 */
export function ConsoleBlock({ take, keys, centre }: { take: ConsoleTake; keys: ReactNode[]; centre: ReactNode | undefined }) {
  const block = useRef<HTMLDivElement>(null);
  const edge = useEdge(take, block);
  const left = Math.ceil(keys.length / 2);
  const seat = (key: ReactNode, index: number) => (
    <span key={index} className="deck-console__seat">
      {key}
    </span>
  );
  const style = { "--l": `${left}fr`, "--r": `${keys.length - left}fr` } as CSSProperties;
  return (
    <div ref={block} className="deck-controls__block deck-console__block" data-take={take} style={style} onKeyDown={along}>
      {edge === undefined ? null : (
        <svg className="deck-console__edge" viewBox={`0 0 ${edge.width} ${edge.height}`} aria-hidden="true" focusable="false">
          <path className="deck-console__rim" d={edge.d} />
        </svg>
      )}
      <div className="deck-console__wing" data-side="left">
        {keys.slice(0, left).map((key, i) => seat(key, i))}
      </div>
      <div className="deck-console__core">{centre === undefined ? null : seat(centre, left)}</div>
      <div className="deck-console__wing" data-side="right">
        {keys.slice(left).map((key, i) => seat(key, left + 1 + i))}
      </div>
    </div>
  );
}
