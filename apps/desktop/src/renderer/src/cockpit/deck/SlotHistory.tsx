// A bay's history: who held it recently, newest first, on a rail that joins the rows in time, under a strip of the last day in
// blocks so that quiet and busy stretches read at a glance. **Only what Fleet's slot events say**: who took the slot and when it
// was given back, so a row carries a holder and a span and nothing about why it ended or which branch it was on, and a bay with
// no slot events has no history and draws none. Pressing a row opens the Job or Session when the ship still holds it. Facts
// only; nothing is counted. Ported from proto/cockpit-flight-deck. #2301.

import type { KeyboardEvent } from "react";
import { Tooltip } from "@armada/components";

import { kindGlyph } from "./BayRows";
import { clockOf, shortOf, spanOf, type Holder, type Ledger, type Lease } from "./bays";

const DAY = 24 * 60 * 60_000;

const whoOf = (holder: Holder) => holder.title ?? shortOf(holder.id);
const spanText = (one: Lease) => (one.from === undefined ? undefined : `${clockOf(one.from)} to ${one.to === undefined ? "now" : clockOf(one.to)}`);

export function SlotHistory({ ledger, clock, known, onOpen }: { ledger: Ledger; clock: number; known: ReadonlySet<string>; onOpen: (holder: Holder) => void }) {
  const rows = ledger.leases;
  const start = clock - DAY;
  const at = (ms: number) => `${(Math.min(1, Math.max(0, (ms - start) / DAY)) * 100).toFixed(2)}%`;
  const move = (event: KeyboardEvent<HTMLOListElement>) => {
    const next = event.key === "j" || event.key === "ArrowDown" ? 1 : event.key === "k" || event.key === "ArrowUp" ? -1 : 0;
    if (next === 0 || event.metaKey || event.ctrlKey || event.altKey) return;
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>(".deck-history__row")];
    const here = all.findIndex((one) => one === document.activeElement || one.contains(document.activeElement));
    const to = all[Math.min(all.length - 1, Math.max(0, here < 0 ? (next > 0 ? 0 : all.length - 1) : here + next))];
    if (to === undefined) return;
    event.preventDefault();
    to.focus();
  };
  if (rows.length === 0) return null;
  return (
    <section className="deck-history" aria-label="History">
      <h3 className="deck-history__head">History</h3>
      <div className="deck-history__strip" role="img" aria-label="The last 24 hours, a block for each holder">
        {rows.flatMap((one, index) => {
          if (one.from === undefined) return [];
          const to = one.to ?? clock;
          return [
            <Tooltip asChild key={`${one.holder.id}:${one.from}:${index}`} label={`${whoOf(one.holder)}, ${clockOf(one.from)} to ${one.to === undefined ? "now" : clockOf(one.to)}`}>
              <span className="deck-history__block" data-kind={one.holder.kind} data-ended={one.to === undefined ? "held" : undefined} style={{ left: at(one.from), width: `max(var(--space-1), calc(${at(to)} - ${at(one.from)}))` }} />
            </Tooltip>,
          ];
        })}
      </div>
      <ol className="deck-history__rows" onKeyDown={move}>
        {rows.map((one, index) => {
          const live = known.has(`${one.holder.kind}:${one.holder.id}`);
          const kind = one.holder.kind === "job" ? "Job" : "Session";
          const span = spanText(one);
          return (
            <li key={`${one.holder.id}:${one.from ?? "unknown"}:${index}`} className="deck-history__item" data-ended={one.to === undefined ? "held" : undefined}>
              <button type="button" className="deck-history__row" data-gone={!live || undefined} tabIndex={index === 0 ? 0 : -1} aria-label={`${kind} ${whoOf(one.holder)}`} aria-disabled={!live || undefined} onClick={() => live && onOpen(one.holder)}>
                <span className="deck-history__who">
                  <Tooltip label={kind}>
                    <span className="deck-history__kind" role="img" aria-label={kind}>
                      {kindGlyph(one.holder.kind, 14)}
                    </span>
                  </Tooltip>
                  <Tooltip label={one.holder.title ?? one.holder.id}>
                    <span className="deck-history__title">{whoOf(one.holder)}</span>
                  </Tooltip>
                </span>
                {span === undefined || one.from === undefined ? null : (
                  <span className="deck-history__facts">
                    <span className="deck-history__span">{span}</span>
                    <span className="deck-history__length">{spanOf((one.to ?? clock) - one.from)}</span>
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
