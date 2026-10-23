// The gaps between the deck's panels as handles. A gap in a column resizes the two panels either side of it, and the middle's stage
// against its foot; the gap between columns is the column handle (`columns.ts`). The sizes are kept in this browser for each layout
// (nothing selected, a Session open), as a share of the column for each panel by name, so a window of another height keeps the
// proportions. A column nobody has dragged is left to the stylesheet. A panel folded to its band, or drawn as a band alone, stands
// outside the shares and has no handle beside it. The panels are found in the page as drawn, because a Session's panels are
// portalled into the column by their own hosts and `display: contents` wrappers hold some.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode, type RefObject } from "react";
import { clampToRange, tokenPx } from "@armada/components";

import "./splits.css";

type Weights = Record<string, number>;
type Shares = { size: Weights; natural: Weights };
type Kept = Record<string, Shares>;
type Gap = { id: string; column: string; above: string; below: string; label: string; x: number; y: number; w: number; h: number };

const KEY = "armada.bridge.deck-splits";
// The stage is a column inside the middle one: the Job over the Cockpit's docked shell (`ShellPanel`), with the merge queue under them.
const COLUMNS = [["left", ".deck-left"], ["centre", ".deck-centre"], ["right", ".deck-right"], ["stage", ".deck-centre > .deck-stage"]] as const;
const PANELS = ".deck-panel, .deck-pane, .sv-panel, .deck-stage, .deck-foot";
const GENERIC = new Set(["deck-panel", "deck-pane", "sv-panel"]);
/** One arrow press, as the column handle moves. */
const STEP = 16;
/** Without a minimum of its own a panel keeps room for its band and a row. */
const FLOOR_UNITS = 2;
const NAMES: Record<string, string> = { "deck-stage": "Thread", "deck-foot": "Foot" };

const keyOf = (panel: HTMLElement) => panel.dataset.area ?? [...panel.classList].find((one) => !GENERIC.has(one)) ?? "panel";
const nameOf = (panel: HTMLElement) => panel.getAttribute("aria-label") ?? NAMES[keyOf(panel)] ?? keyOf(panel);
const fixed = (panel: HTMLElement) => panel.hasAttribute("data-collapsed") || panel.hasAttribute("data-band-only");

function panelsOf(column: HTMLElement): HTMLElement[] {
  return [...column.querySelectorAll<HTMLElement>(PANELS)]
    .filter((one) => [null, undefined, column].includes(one.parentElement?.closest<HTMLElement>(PANELS)) && one.offsetHeight > 0)
    .sort((a, b) => a.getBoundingClientRect().top - b.getBoundingClientRect().top);
}

function minOf(panel: HTMLElement): number {
  const floor = FLOOR_UNITS * tokenPx("--space-12");
  const own = parseFloat(getComputedStyle(panel).minHeight);
  return Math.max(Number.isFinite(own) ? own : 0, Number.isFinite(floor) ? floor : 0);
}

const finite = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value) && value > 0;

function weights(raw: unknown): Weights {
  const out: Weights = {};
  if (typeof raw === "object" && raw !== null) for (const [name, value] of Object.entries(raw)) if (finite(value)) out[name] = value;
  return out;
}

function load(layout: string): Kept {
  try {
    const parsed: unknown = JSON.parse(window.localStorage.getItem(`${KEY}.${layout}`) ?? "{}");
    const kept: Kept = {};
    if (typeof parsed === "object" && parsed !== null) {
      for (const [column, shares] of Object.entries(parsed)) {
        const { size, natural } = (shares ?? {}) as Record<string, unknown>;
        kept[column] = { size: weights(size), natural: weights(natural) };
      }
    }
    return kept;
  } catch {
    return {};
  }
}

function save(layout: string, kept: Kept): void {
  try {
    window.localStorage.setItem(`${KEY}.${layout}`, JSON.stringify(kept));
  } catch {
    // Not kept past this window.
  }
}

function writeShare(panel: HTMLElement, share: number | undefined): void {
  if (share === undefined) {
    if (panel.dataset.sized !== undefined) delete panel.dataset.sized;
    panel.style.removeProperty("--w");
    return;
  }
  panel.dataset.sized = "";
  panel.style.setProperty("--w", String(Math.round(Math.max(share, 0.001) * 100000) / 100));
}

/** Each column's panels drawn at their kept shares, and the handles between them. Render what it returns inside the grid. */
export function useSplits(grid: RefObject<HTMLElement | null>, layout: string): ReactNode {
  const [gaps, setGaps] = useState<Gap[]>([]);
  const [held, setHeld] = useState<string | undefined>(undefined);
  const kept = useRef<Kept>({});
  const shown = useRef("");
  const until = useRef(0);
  const frame = useRef(0);
  const watched = useRef(new Set<Element>());
  const watcher = useRef<ResizeObserver | null>(null);
  const dragging = useRef<{ pointerId: number; startY: number; column: string; above: HTMLElement; below: HTMLElement; ha: number; hb: number; total: number } | null>(null);

  const apply = useCallback((name: string, panels: HTMLElement[]) => {
    const size = kept.current[name]?.size ?? {};
    const flexible = panels.filter((one) => !fixed(one));
    const known = flexible.map((one) => size[keyOf(one)]).filter((one): one is number => one !== undefined);
    if (known.length === 0) return panels.forEach((one) => writeShare(one, undefined));
    const mean = known.reduce((sum, one) => sum + one, 0) / known.length;
    for (const one of panels) writeShare(one, fixed(one) ? undefined : (size[keyOf(one)] ?? mean));
  }, []);

  const measure = useCallback(() => {
    const element = grid.current;
    if (element === null) return;
    const origin = element.getBoundingClientRect();
    const next: Gap[] = [];
    for (const [name, selector] of COLUMNS) {
      const column = element.querySelector<HTMLElement>(`:scope > ${selector}`);
      if (column === null) continue;
      const panels = panelsOf(column);
      apply(name, panels);
      for (const one of panels) {
        if (!watched.current.has(one)) {
          watched.current.add(one);
          watcher.current?.observe(one);
        }
      }
      const box = column.getBoundingClientRect();
      for (let at = 0; at + 1 < panels.length; at += 1) {
        const above = panels[at]!;
        const below = panels[at + 1]!;
        if (fixed(above) || fixed(below)) continue;
        const top = above.getBoundingClientRect().bottom;
        const space = below.getBoundingClientRect().top - top;
        if (space < 2) continue;
        next.push({ id: `${name}:${keyOf(above)}|${keyOf(below)}`, column: name, above: keyOf(above), below: keyOf(below), label: `${nameOf(above)} and ${nameOf(below)}`, x: Math.round(box.left - origin.left), y: Math.round(top - origin.top), w: Math.round(box.width), h: Math.round(space) });
      }
    }
    const said = JSON.stringify(next);
    if (said === shown.current) return;
    shown.current = said;
    setGaps(next);
  }, [apply, grid]);

  // A size can move for a good half second after what moved it (a fold, a card arriving), so the handles follow it a frame at a time.
  const kick = useCallback(() => {
    until.current = performance.now() + 700;
    if (frame.current !== 0) return;
    const tick = () => {
      frame.current = 0;
      measure();
      if (performance.now() < until.current) frame.current = requestAnimationFrame(tick);
    };
    frame.current = requestAnimationFrame(tick);
  }, [measure]);

  useLayoutEffect(() => {
    const element = grid.current;
    if (element === null) return;
    const watching = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(kick);
    watching?.observe(element);
    watcher.current = watching;
    const changes = new MutationObserver((records) => {
      const relevant = records.some((one) => one.type === "attributes" || [...one.addedNodes, ...one.removedNodes].some((node) => node instanceof Element && (node.matches(PANELS) || node.querySelector(PANELS) !== null)));
      if (relevant) kick();
    });
    changes.observe(element, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-collapsed"] });
    kick();
    return () => {
      watching?.disconnect();
      changes.disconnect();
      watched.current.clear();
      watcher.current = null;
      cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, [grid, kick]);

  useLayoutEffect(() => {
    kept.current = load(layout);
    kick();
  }, [layout, kick]);

  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  /** The column's panels at the shares they are drawn at, which become the kept ones if none are yet. */
  const seed = (name: string) => {
    const selector = COLUMNS.find(([one]) => one === name)![1];
    const column = grid.current?.querySelector<HTMLElement>(`:scope > ${selector}`);
    if (column === null || column === undefined) return null;
    const panels = panelsOf(column).filter((one) => !fixed(one));
    const heights = panels.map((one) => one.getBoundingClientRect().height);
    const total = heights.reduce((sum, one) => sum + one, 0);
    if (total <= 0) return null;
    const shares = (kept.current[name] ??= { size: {}, natural: {} });
    const fresh = Object.keys(shares.size).length === 0;
    panels.forEach((one, at) => {
      shares.size[keyOf(one)] = heights[at]! / total;
      if (fresh || shares.natural[keyOf(one)] === undefined) shares.natural[keyOf(one)] = heights[at]! / total;
    });
    return { panels, heights, total, shares };
  };

  const begin = (gap: Gap, event: PointerEvent<HTMLElement>) => {
    const seeded = seed(gap.column);
    const above = seeded?.panels.findIndex((one) => keyOf(one) === gap.above) ?? -1;
    const below = seeded?.panels.findIndex((one) => keyOf(one) === gap.below) ?? -1;
    if (seeded === null || above < 0 || below < 0) return;
    dragging.current = { pointerId: event.pointerId, startY: event.clientY, column: gap.column, above: seeded.panels[above]!, below: seeded.panels[below]!, ha: seeded.heights[above]!, hb: seeded.heights[below]!, total: seeded.total };
    grid.current?.setAttribute("data-splitting", "");
    setHeld(gap.id);
  };

  const move = (by: number) => {
    const now = dragging.current;
    if (now === null) return;
    const share = kept.current[now.column];
    if (share === undefined) return;
    const by_ = clampToRange(by, { min: minOf(now.above) - now.ha, max: now.hb - minOf(now.below) });
    share.size[keyOf(now.above)] = (now.ha + by_) / now.total;
    share.size[keyOf(now.below)] = (now.hb - by_) / now.total;
    measure();
  };

  const end = () => {
    dragging.current = null;
    grid.current?.removeAttribute("data-splitting");
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    setHeld(undefined);
    save(layout, kept.current);
    kick();
  };

  const reset = (gap: Gap) => {
    const seeded = seed(gap.column);
    if (seeded === null) return;
    const { size, natural } = seeded.shares;
    const share = (size[gap.above] ?? 0) + (size[gap.below] ?? 0);
    const wanted = natural[gap.above] ?? 1;
    const other = natural[gap.below] ?? 1;
    size[gap.above] = (share * wanted) / (wanted + other);
    size[gap.below] = share - size[gap.above]!;
    save(layout, kept.current);
    kick();
  };

  const key = (gap: Gap, event: KeyboardEvent<HTMLElement>) => {
    const by = event.key === "ArrowUp" ? -STEP : event.key === "ArrowDown" ? STEP : undefined;
    if (event.key === "Home") reset(gap);
    else if (by === undefined) return;
    else {
      begin(gap, { pointerId: 0, clientY: 0 } as PointerEvent<HTMLElement>);
      move(by);
      end();
    }
    event.preventDefault();
  };

  return gaps.map((gap) => (
    <div
      key={gap.id}
      className="deck-split"
      role="separator"
      aria-orientation="horizontal"
      aria-label={`Resize ${gap.label}`}
      tabIndex={0}
      data-held={held === gap.id || undefined}
      style={{ left: gap.x, top: gap.y, width: gap.w, height: gap.h }}
      onDoubleClick={() => reset(gap)}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        try {
          event.currentTarget.setPointerCapture(event.pointerId);
        } catch {
          // A pointer sent by a script has nothing to capture; its moves are sent here as well.
        }
        document.body.style.cursor = "row-resize";
        document.body.style.userSelect = "none";
        event.currentTarget.focus();
        event.preventDefault();
        begin(gap, event);
      }}
      onPointerMove={(event) => {
        if (dragging.current?.pointerId === event.pointerId) move(event.clientY - dragging.current.startY);
      }}
      onPointerUp={(event) => {
        if (dragging.current?.pointerId === event.pointerId) end();
      }}
      onPointerCancel={(event) => {
        if (dragging.current?.pointerId === event.pointerId) end();
      }}
      onKeyDown={(event) => key(gap, event)}
    />
  ));
}
