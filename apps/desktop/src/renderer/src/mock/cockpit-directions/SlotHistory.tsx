// A bay's history: who held it recently, newest first, on a rail that joins the rows in time, under a strip of the last day in blocks
// so that quiet and busy stretches read at a glance. Each row says who, on what branch, from when to when, and how it ended, as a glyph
// with its tooltip. Pressing a row opens the Job or Session when the ship still holds it. Facts only; nothing is counted. At a replay's
// playhead it is the history as of then. Mock only. proto/cockpit-flight-deck.

import { Anchor, Ghost, Hand, KeyRound, Check } from "lucide-react";
import type { KeyboardEvent, ReactNode } from "react";
import { Tooltip } from "@armada/components";

import { kindGlyph } from "./BayRows";
import { ENDING, clockOf, shortOf, spanOf, type Ending, type Holder, type Lease } from "./deck-bays";

const DAY = 24 * 60 * 60_000;

const ENDED: Record<Ending, ReactNode> = {
  held: <KeyRound size={14} aria-hidden />,
  clean: <Check size={14} aria-hidden />,
  kept: <Anchor size={14} aria-hidden />,
  stranded: <Ghost size={14} aria-hidden />,
  taken: <Hand size={14} aria-hidden />,
};

/** The history as of an instant: leases that began by then, the one still going shown as held. */
export const leasesAt = (ledger: readonly Lease[], clock: number): Lease[] =>
  ledger.filter((one) => one.from <= clock).map((one) => (one.to === undefined || one.to > clock ? { holder: one.holder, branch: one.branch, from: one.from, ended: "held" as const } : one));

const whoOf = (holder: Holder) => holder.title ?? shortOf(holder.id);

export function SlotHistory({ ledger, clock, known, onOpen }: { ledger: readonly Lease[]; clock: number; known: ReadonlySet<string>; onOpen: (holder: Holder) => void }) {
  const rows = leasesAt(ledger, clock);
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
        {rows.map((one, at_) => {
          const to = one.to ?? clock;
          return (
            <Tooltip asChild key={`${one.holder.id}:${one.from}:${at_}`} label={`${whoOf(one.holder)}, ${clockOf(one.from)} to ${one.to === undefined ? "now" : clockOf(one.to)}`}>
              <span className="deck-history__block" data-kind={one.holder.kind} data-ended={one.ended} style={{ left: at(one.from), width: `max(2px, calc(${at(to)} - ${at(one.from)}))` }} />
            </Tooltip>
          );
        })}
      </div>
      <ol className="deck-history__rows" onKeyDown={move}>
        {rows.map((one, i) => {
          const live = known.has(`${one.holder.kind}:${one.holder.id}`);
          return (
            <li key={`${one.holder.id}:${one.from}`} className="deck-history__item" data-ended={one.ended}>
              <button
                type="button"
                className="deck-history__row"
                data-gone={!live || undefined}
                tabIndex={i === 0 ? 0 : -1}
                aria-label={`${one.holder.kind === "job" ? "Job" : "Session"} ${whoOf(one.holder)}, ${ENDING[one.ended]}`}
                aria-disabled={!live || undefined}
                onClick={() => live && onOpen(one.holder)}
              >
                <span className="deck-history__who">
                  <Tooltip label={one.holder.kind === "job" ? "Job" : "Session"}>
                    <span className="deck-history__kind" role="img" aria-label={one.holder.kind === "job" ? "Job" : "Session"}>
                      {kindGlyph(one.holder.kind, 14)}
                    </span>
                  </Tooltip>
                  <Tooltip label={one.holder.title ?? one.holder.id}>
                    <span className="deck-history__title">{whoOf(one.holder)}</span>
                  </Tooltip>
                  <Tooltip label={ENDING[one.ended]}>
                    <span className="deck-history__end" role="img" aria-label={ENDING[one.ended]}>
                      {ENDED[one.ended]}
                    </span>
                  </Tooltip>
                </span>
                <span className="deck-history__facts">
                  <Tooltip label={one.branch}>
                    <span className="deck-history__branch">{one.branch}</span>
                  </Tooltip>
                  <span className="deck-history__span">
                    {clockOf(one.from)} to {one.to === undefined ? "now" : clockOf(one.to)}
                  </span>
                  <span className="deck-history__length">{spanOf((one.to ?? clock) - one.from)}</span>
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
