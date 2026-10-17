// A bay's history: every lease and every branch checkout, newest first, on a rail that joins the rows in time, under a strip of
// the last day in blocks so that quiet and busy stretches read at a glance. A lease is one row: who took it, when, for how long
// and on which branch; a checkout is a row of its own, from one branch to the next at a commit. Days are headed on the rail. A
// history Fleet cut has a rail that fades out below its last row. Pressing a lease opens the Session when the ship still
// holds it. Facts only; nothing is counted. Ported from proto/cockpit-flight-deck. #2301.

import { Fragment, type KeyboardEvent } from "react";
import { GitBranch, SquareTerminal } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { BayHistory, BayHolder } from "@armada/protocol";

import { rowsOf, type Stay } from "./bay-history";
import { clockOf, shortOf, spanOf } from "./bays";

const DAY = 24 * 60 * 60_000;

const whoOf = (holder: BayHolder) => holder.title ?? shortOf(holder.id);
const KIND = { session: "Job", agent: "Agent" } as const;
const spanText = (one: Stay) =>
  one.from === undefined ? (one.to === undefined ? undefined : `to ${clockOf(one.to)}`) : `${clockOf(one.from)} to ${one.to === undefined ? "now" : clockOf(one.to)}`;

/** The day a row is under: Today, Yesterday, or the date. */
function dayOf(at: number, clock: number): string {
  const day = (ms: number) => new Date(ms).toDateString();
  if (day(at) === day(clock)) return "Today";
  if (day(at) === day(clock - DAY)) return "Yesterday";
  return new Date(at).toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short" });
}

export function SlotHistory({ history, clock, known, onOpen }: { history: BayHistory; clock: number; known: ReadonlySet<string>; onOpen: (holder: { kind: "session"; id: string }) => void }) {
  const rows = rowsOf(history);
  const stays = rows.flatMap((one) => (one.kind === "stay" ? [one.stay] : []));
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
  let first = true;
  let heading = "";
  return (
    <section className="deck-history" aria-label="History">
      <h3 className="deck-history__head">History</h3>
      <div className="deck-history__strip" role="img" aria-label="The last 24 hours, a block for each holder">
        {stays.flatMap((one, index) => {
          if (one.from === undefined || (one.to ?? clock) < start) return [];
          const to = one.to ?? clock;
          return [
            <Tooltip asChild key={`${one.holder.id}:${one.from}:${index}`} label={`${whoOf(one.holder)}, ${clockOf(one.from)} to ${one.to === undefined ? "now" : clockOf(one.to)}`}>
              <span className="deck-history__block" data-kind={one.holder.kind} data-ended={one.to === undefined ? "held" : undefined} style={{ left: at(one.from), width: `max(var(--space-1), calc(${at(to)} - ${at(one.from)}))` }} />
            </Tooltip>,
          ];
        })}
      </div>
      <ol className="deck-history__rows" data-cut={history.cut || undefined} onKeyDown={move}>
        {rows.map((row, index) => {
          const day = dayOf(row.at, clock);
          const head = day === heading ? null : <li className="deck-history__day">{day}</li>;
          heading = day;
          if (row.kind === "checkout") {
            return (
              <Fragment key={`checkout:${row.sha}:${row.at}:${index}`}>
                {head}
                <li className="deck-history__item" data-row="checkout">
                  <div className="deck-history__checkout">
                    <Tooltip label="Checkout">
                      <span className="deck-history__kind" role="img" aria-label="Checkout">
                        <GitBranch size={14} aria-hidden />
                      </span>
                    </Tooltip>
                    <span className="deck-history__refs">
                      {row.from === undefined ? null : (
                        <>
                          <span className="deck-history__ref">{row.from}</span>
                          <span className="deck-history__to">to</span>
                        </>
                      )}
                      <span className="deck-history__ref">{row.to}</span>
                    </span>
                    <Tooltip label="Commit">
                      <span className="deck-history__sha">{row.sha.slice(0, 7)}</span>
                    </Tooltip>
                    <span className="deck-history__span">{clockOf(row.at)}</span>
                  </div>
                </li>
              </Fragment>
            );
          }
          const one = row.stay;
          const live = one.holder.kind !== "agent" && known.has(`${one.holder.kind}:${one.holder.id}`);
          const kind = KIND[one.holder.kind];
          const span = spanText(one);
          const tab = first ? 0 : -1;
          first = false;
          return (
            <Fragment key={`${one.holder.id}:${one.from ?? "cut"}:${index}`}>
              {head}
              <li className="deck-history__item" data-ended={one.to === undefined ? "held" : undefined}>
                <button
                  type="button"
                  className="deck-history__row"
                  data-gone={!live || undefined}
                  tabIndex={tab}
                  aria-label={`${kind} ${whoOf(one.holder)}`}
                  aria-disabled={!live || undefined}
                  onClick={() => live && one.holder.kind === "session" && onOpen({ kind: "session", id: one.holder.id })}
                >
                  <span className="deck-history__who">
                    <Tooltip label={kind}>
                      <span className="deck-history__kind" role="img" aria-label={kind}>
                        <SquareTerminal size={14} aria-hidden />
                      </span>
                    </Tooltip>
                    <Tooltip label={one.holder.title ?? one.holder.id}>
                      <span className="deck-history__title">{whoOf(one.holder)}</span>
                    </Tooltip>
                  </span>
                  {span === undefined && one.branch === undefined ? null : (
                    <span className="deck-history__facts">
                      {one.branch === undefined ? null : (
                        <Tooltip label={one.branch}>
                          <span className="deck-history__branch">{one.branch}</span>
                        </Tooltip>
                      )}
                      {span === undefined ? null : <span className="deck-history__span">{span}</span>}
                      {one.from === undefined ? null : (
                        <Tooltip label="Held for">
                          <span className="deck-history__length">{spanOf((one.to ?? clock) - one.from)}</span>
                        </Tooltip>
                      )}
                    </span>
                  )}
                </button>
              </li>
            </Fragment>
          );
        })}
      </ol>
    </section>
  );
}
