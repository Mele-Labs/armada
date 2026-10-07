import { useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Search } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Chip, type OwnerChipRef } from "../OwnerChip/OwnerChip";
import { ModMark, SessionMark, type SessionState } from "../SessionFrame/SessionFrame";

/**
 * Sessions, searchable, under the headings Overview's lists use: **Needs you**
 * and **Running**, a heading drawn only where it has a row. **Search finds by
 * PR number, branch, Job id, slot or title**, and the host does the finding.
 *
 * **A row says what is open in a Session**: its worktree slot and its open
 * pull requests, each with how its Checks stand, on one line that never wraps.
 * Where the width holds fewer than there are, the most recent show and a `…`
 * keeps the rest in its tooltip. A merged pull request is on the Session's own
 * ledger, not on its row, and a row with nothing open has no second line. The
 * time sits at the title line's end. What a search matched is the chip that is
 * ringed. **No count is drawn** beside any of it; the rows are the list.
 */
export type SessionRowView = {
  id: string;
  /** What other sessions call it, drawn where the id would be on a Session with no title yet. */
  address?: string;
  title?: string;
  state: SessionState;
  said: string;
  slots: readonly number[];
  /** A merged one is on the Session's ledger and not on its row. */
  pullRequests: readonly { number: number; checks: "pending" | "passed" | "failed"; said: string; state?: "open" | "draft" | "merged" }[];
  /** What the search matched in this Session, where it matched an attachment. */
  matched?: OwnerChipRef;
  /** The clock time of the last turn, which the tooltip keeps. */
  lastTurn?: string;
  /** When it was, so the row can say how long ago. */
  lastTurnAt?: string;
  /** Its mod is older than the repository's. */
  modOutOfDate?: boolean;
};

export type SessionGroup = { label: string; rows: readonly SessionRowView[] };

export type SessionListProps = {
  groups: readonly SessionGroup[];
  query: string;
  onQuery: (query: string) => void;
  onOpen: (id: string) => void;
  onStart: () => void;
  /** The moment times are counted from; the clock, unless a story fixes it. */
  now?: number;
};

const same = (a: OwnerChipRef | undefined, b: OwnerChipRef): boolean => a !== undefined && JSON.stringify(a) === JSON.stringify(b);

/** Time since `at`, as "4m", "2h" or "3d"; nothing for a stamp that will not parse. */
export function since(at: string, now: number): string | undefined {
  const then = Date.parse(at);
  if (Number.isNaN(then)) return undefined;
  const minutes = Math.max(0, Math.floor((now - then) / 60_000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
}

type Item = { key: string; fixed: boolean; label: string; node: ReactNode };

/** How many of the droppable items, newest first, fit in `width` beside the fixed ones and a `…`. */
function keptOf(widths: readonly number[], items: readonly Item[], width: number, gap: number): Set<string> {
  const all = new Set(items.map((one) => one.key));
  const more = widths[items.length];
  if (width <= 0 || more === undefined) return all;
  const total = items.reduce((sum, _, at) => sum + (widths[at] ?? 0) + (at > 0 ? gap : 0), 0);
  if (total <= width) return all;
  let used = 0;
  const kept = new Set<string>();
  items.forEach((one, at) => {
    if (one.fixed) {
      used += (widths[at] ?? 0) + gap;
      kept.add(one.key);
    }
  });
  used += more + gap;
  for (let at = items.length - 1; at >= 0; at--) {
    const one = items[at]!;
    if (one.fixed) continue;
    const next = used + (widths[at] ?? 0) + gap;
    if (next > width + gap) break;
    used = next;
    kept.add(one.key);
  }
  return kept;
}

/** The open items on one line: what fits, newest kept, and a `…` for the rest. */
function OpenLine({ items }: { items: readonly Item[] }) {
  const line = useRef<HTMLSpanElement>(null);
  const ghost = useRef<HTMLSpanElement>(null);
  const [kept, setKept] = useState<Set<string> | undefined>(undefined);
  const signature = items.map((one) => one.key).join("|");
  useLayoutEffect(() => {
    const measure = () => {
      if (line.current === null || ghost.current === null) return;
      const widths = [...ghost.current.children].map((child) => child.getBoundingClientRect().width);
      const gap = Number.parseFloat(getComputedStyle(line.current).columnGap) || 0;
      const style = getComputedStyle(line.current);
      setKept(keptOf(widths, items, line.current.clientWidth - (Number.parseFloat(style.paddingInlineStart) || 0), gap));
    };
    measure();
    const watch = new ResizeObserver(measure);
    if (line.current !== null) watch.observe(line.current);
    return () => watch.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);
  const shown = items.filter((one) => kept === undefined || kept.has(one.key));
  const rest = items.filter((one) => !shown.includes(one));
  return (
    <span className="armada-session-list__chips" ref={line}>
      {shown.map((one) => (
        <span className="armada-session-list__item" key={one.key}>
          {one.node}
        </span>
      ))}
      {rest.length === 0 ? null : (
        <Tooltip label={rest.map((one) => one.label).join(", ")}>
          <span className="armada-session-list__more" role="img" aria-label={`Also open: ${rest.map((one) => one.label).join(", ")}`}>
            …
          </span>
        </Tooltip>
      )}
      <span className="armada-session-list__ghost" aria-hidden ref={ghost}>
        {items.map((one) => (
          <span className="armada-session-list__item" key={one.key}>
            {one.node}
          </span>
        ))}
        <span className="armada-session-list__more">…</span>
      </span>
    </span>
  );
}

function itemsOf(row: SessionRowView): Item[] {
  const items: Item[] = row.slots.map((slot) => ({
    key: `slot${slot}`,
    fixed: true,
    label: `Slot ${slot}`,
    node: <Chip chip={{ kind: "slot", slot }} matched={same(row.matched, { kind: "slot", slot })} />,
  }));
  for (const pr of row.pullRequests.filter((one) => one.state !== "merged")) {
    items.push({
      key: `pr${pr.number}`,
      fixed: false,
      label: `#${pr.number}`,
      node: (
        <span className="armada-session-list__pr" data-checks={pr.checks}>
          <Chip chip={{ kind: "pull_request", number: pr.number }} matched={same(row.matched, { kind: "pull_request", number: pr.number })} checks={{ state: pr.checks, said: pr.said }} />
        </span>
      ),
    });
  }
  if (row.matched?.kind === "branch" || row.matched?.kind === "job") {
    items.push({ key: "matched", fixed: true, label: "Match", node: <Chip chip={row.matched} matched /> });
  }
  return items;
}

export function SessionList({ groups, query, onQuery, onOpen, onStart, now = Date.now() }: SessionListProps) {
  return (
    <section className="armada-session-list" aria-label="Sessions">
      <div className="armada-session-list__head">
        <h2 className="armada-session-list__title-bar">Sessions</h2>
        <Button size="sm" variant="secondary" onClick={onStart}>
          New Session
        </Button>
      </div>
      <Input
        type="search"
        aria-label="Search Sessions"
        mono
        value={query}
        onChange={(event) => onQuery(event.target.value)}
        trailing={<Search size={12} strokeWidth={2} aria-hidden />}
      />
      {groups.map((group) => (
        <div className="armada-session-list__group" key={group.label}>
          <h3 className="armada-session-list__eyebrow">{group.label}</h3>
          <ul className="armada-session-list__rows">
            {group.rows.map((row) => {
              const items = itemsOf(row);
              return (
                <li key={row.id} className="armada-session-list__row" data-state={row.state} aria-label={row.title ?? row.address ?? row.id}>
                  <div className="armada-session-list__top">
                    <button type="button" className="armada-session-list__open" onClick={() => onOpen(row.id)}>
                      <SessionMark state={row.state} said={row.said} />
                      <span className="armada-session-list__title">
                        {row.title ?? <span className="armada-session-list__id">{row.address ?? row.id}</span>}
                      </span>
                      {row.modOutOfDate === true ? <ModMark size={12} /> : null}
                    </button>
                    {row.lastTurn === undefined ? null : (
                      <Tooltip label={`Last turn ${row.lastTurn}`}>
                        <span className="armada-session-list__last" aria-label={`Last turn ${row.lastTurn}`}>
                          {(row.lastTurnAt === undefined ? undefined : since(row.lastTurnAt, now)) ?? row.lastTurn}
                        </span>
                      </Tooltip>
                    )}
                  </div>
                  {items.length === 0 ? null : <OpenLine items={items} />}
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </section>
  );
}
