// Which scenario the mock is on, and a way to another scenario or a walk. Dev-only: nothing the
// Electron build bundles imports this file.

import { Fragment, StrictMode, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { createRoot } from "react-dom/client";
import { Button, Input } from "@armada/components";
import { FlaskConical } from "lucide-react";
import type { KeyboardEvent, ReactNode } from "react";

import "./mock.css";
import { SCENARIOS } from "./scenario";
import { EVERY_WALK } from "./walks";

/**
 * The app's own left column — where the owner asked for the picker on
 * 30 Sep 2026: *"I still would like to find a better way to overlay the mock
 * scenarios. What if we put it in the left side panel?"*
 *
 * **In the column, not over it.** #1618 answered the same complaint by making
 * the card draggable, which moved the problem rather than ending it: wherever
 * it was dropped it was still on top of something, and every screenshot taken
 * all night had to hide it first. A child of the column is in the flow with
 * Navigation, Stats and Fleet, so it covers nothing at any width and scrolls
 * with them when the window is too short to hold all four.
 */
const COLUMN = ".armada-shell__left";

/**
 * The column, once the app has drawn it. `main.tsx` mounts the app first, but
 * React renders on its own schedule, so it is not there when this root mounts
 * — and it never arrives inside a `?frame` capture, where the picker is not
 * mounted at all. Watching is what covers both without polling.
 */
function useColumn(): Element | null {
  const [column, setColumn] = useState<Element | null>(() => document.querySelector(COLUMN));
  useEffect(() => {
    if (column !== null) return;
    const watch = new MutationObserver(() => {
      const found = document.querySelector(COLUMN);
      if (found !== null) setColumn(found);
    });
    watch.observe(document.body, { childList: true, subtree: true });
    return () => watch.disconnect();
  }, [column]);
  return column;
}

/**
 * Where each character of `query` sits in `name`, in order, or `null` where
 * they do not all fit. **A subsequence, not a substring**: there are over a
 * hundred scenarios and the useful thing to type is the initials of a long
 * one — `arcex` finds `arc/executing-concurrent`.
 */
function fuzzy(name: string, query: string): number[] | null {
  const at: number[] = [];
  let from = 0;
  for (const letter of query) {
    const found = name.indexOf(letter, from);
    if (found === -1) return null;
    at.push(found);
    from = found + 1;
  }
  return at;
}

/**
 * How well `at` reads as a hit, lower first: a run of adjacent characters
 * beats a scattering of them, and an early hit beats a late one. **No other
 * term** — a score with more in it is one nobody can predict from the query
 * they typed, and this list is walked by eye as often as it is searched.
 */
function score(at: number[]): number {
  const gaps = at.reduce((held, one, i) => (i === 0 ? 0 : held + (one - at[i - 1]! - 1)), 0);
  return gaps * 100 + (at[0] ?? 0);
}

/** A scenario or a walk that matched, and where — the marks the row draws it with. */
type Hit = { name: string; says: string; at: number[]; walk: boolean };

/**
 * Every walk, as a row: **the list is how one is played from a window that
 * cannot type an address**, which is Bridge's own window on a Job's mock —
 * `docs/practices/capture-window.md` gives it no address field.
 */
const WALK_ROWS = [...EVERY_WALK].map(([name, one]) => ({
  name,
  says: `A walk of ${one.steps.length} steps over ${one.scenario}`,
  walk: true,
}));
const SCENARIO_ROWS = SCENARIOS.map((one) => ({ name: one.name, says: one.says, walk: false }));

/** Each group the query reaches, best first within it. The empty query is all of them, in roster order. */
function hits(query: string, rows: readonly Omit<Hit, "at">[]): Hit[] {
  const wanted = query.trim().toLowerCase();
  if (wanted === "") return rows.map((one) => ({ ...one, at: [] }));
  return rows
    .flatMap((one) => {
      const at = fuzzy(one.name.toLowerCase(), wanted);
      return at === null ? [] : [{ ...one, at }];
    })
    .sort((a, b) => score(a.at) - score(b.at) || a.name.length - b.name.length);
}

/** The name with the matched characters marked, so a fuzzy hit reads as one. */
function marked(name: string, at: number[]): ReactNode {
  if (at.length === 0) return name;
  const held = new Set(at);
  return [...name].map((letter, i) =>
    held.has(i) ? (
      <b className="armada-mock-picker__hit" key={i}>
        {letter}
      </b>
    ) : (
      letter
    ),
  );
}

/**
 * Where a row goes: this page again, on `?scenario=`.
 *
 * **A link, not a handler that navigates.** Choosing reloads, so that a
 * scenario never inherits the last one's window state, and a reload is what an
 * ordinary link already does — with the destination readable before it is
 * pressed, which is also what lets a test read where a row leads without
 * leaving the page.
 */
function to(hit: Pick<Hit, "name" | "walk">): string {
  const url = new URL(window.location.href);
  // A walk names its own scenario, and `?walk` wins over `?scenario` anyway.
  url.searchParams.delete(hit.walk ? "scenario" : "walk");
  url.searchParams.delete("autoplay");
  url.searchParams.set(hit.walk ? "walk" : "scenario", hit.name);
  return url.toString();
}

/**
 * The picker, in the app's own left column.
 *
 * **One control at rest, saying which scenario is on**, and a layer over the
 * content only while it is open — which is the difference the owner asked for:
 * a card parked over his work, however draggable, is a card he has to move.
 *
 * **The list is fuzzy-searched**, because there are over a hundred scenarios
 * and the `<select>` this replaces made finding one a scroll. Typing narrows,
 * the arrows walk what is left, Enter takes the top row, Esc gives up.
 */
export function Picker({ current }: { current: string }) {
  const column = useColumn();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [at, setAt] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const top = useRef<HTMLAnchorElement>(null);

  // Scenarios first and then walks, so Enter on a query both reach is a scenario.
  const found = useMemo(() => [...hits(query, SCENARIO_ROWS), ...hits(query, WALK_ROWS)], [query]);
  // The first row is always the one Enter takes, so a query narrowed to one
  // scenario is a two-key act. Reset with the query rather than clamped: a
  // cursor left at row nine of a list that now has two is nowhere he put it.
  useEffect(() => setAt(0), [query]);
  useEffect(() => {
    if (open) field.current?.focus();
    else setQuery("");
  }, [open]);

  // Esc and a press outside give it up, `Popover`'s own two rules — spelled
  // here because that primitive's panel is `position: absolute` and this one
  // hangs off a column that scrolls, so it has to be `fixed` to escape it.
  useEffect(() => {
    if (!open) return;
    function key(event: globalThis.KeyboardEvent): void {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
    }
    function down(event: MouseEvent): void {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    }
    window.addEventListener("keydown", key, true);
    window.addEventListener("mousedown", down);
    return () => {
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("mousedown", down);
    };
  }, [open]);

  function keyDown(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key === "ArrowDown") setAt((held) => Math.min(held + 1, found.length - 1));
    else if (event.key === "ArrowUp") setAt((held) => Math.max(held - 1, 0));
    // The row's own link, pressed — one way through for the pointer and the
    // keyboard, rather than a second navigation spelled beside it.
    else if (event.key === "Enter") top.current?.click();
    else return;
    event.preventDefault();
  }

  if (column === null) return null;

  return createPortal(
    <div className="armada-mock-picker" ref={root} data-open={open || undefined}>
      <Button
        variant="ghost"
        size="sm"
        data-said
        aria-expanded={open}
        aria-label={`Mock scenario — ${current}`}
        title={`Mock scenario — ${current}`}
        onClick={() => setOpen((held) => !held)}
      >
        <FlaskConical size={16} strokeWidth={2} aria-hidden />
        <span className="armada-mock-picker__current">{current}</span>
      </Button>
      {open ? (
        <div className="armada-mock-picker__layer" role="dialog" aria-label="Mock scenario">
          <Input
            ref={field}
            aria-label="Find a scenario"
            placeholder="Find a scenario"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={keyDown}
          />
          <div className="armada-mock-picker__list">
            {found.map((one, i) => (
              <Fragment key={`${one.walk ? "walk" : "scenario"}:${one.name}`}>
                {one.walk && found[i - 1]?.walk !== true ? (
                  <span className="armada-mock-picker__group">Walks</span>
                ) : null}
                <a
                  ref={i === at ? top : undefined}
                  className="armada-mock-picker__row"
                  href={to(one)}
                  aria-current={!one.walk && one.name === current ? "true" : undefined}
                  data-at={i === at || undefined}
                  data-current={(!one.walk && one.name === current) || undefined}
                  data-walk={one.walk || undefined}
                  title={one.says}
                >
                  {marked(one.name, one.at)}
                </a>
              </Fragment>
            ))}
          </div>
        </div>
      ) : null}
    </div>,
    column,
  );
}

/**
 * The picker on its own root, in `host` — the mock's own mount, so the page and
 * a test put up the same control with the same stylesheet behind it. What it
 * draws goes into the app's left column from there, so `host` stays empty.
 */
export function mountPicker(current: string, host: HTMLElement): () => void {
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Picker current={current} />
    </StrictMode>,
  );
  return () => root.unmount();
}
