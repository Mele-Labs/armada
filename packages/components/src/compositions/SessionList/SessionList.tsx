import { Search, ShieldCheck, ShieldEllipsis, ShieldX } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Chip, type OwnerChipRef } from "../OwnerChip/OwnerChip";
import { SessionMark, type SessionState } from "../SessionFrame/SessionFrame";

/**
 * Sessions, searchable, under the headings Overview's lists use: **Needs you**
 * and **Running**, a heading drawn only where it has a row. **Search finds by
 * PR number, branch, Job id, slot or title**, and the host does the finding.
 *
 * **A row says what a Session holds**: its worktree slot, its pull requests
 * each with how its Checks stand, its Jobs, and one live-or-idle mark. What a
 * search matched is the chip that is ringed. **No count is drawn** beside any
 * of it; the rows are the list.
 */
export type SessionRowView = {
  id: string;
  /** What other sessions call it, drawn where the id would be on a Session with no title yet. */
  address?: string;
  title?: string;
  state: SessionState;
  said: string;
  slots: readonly number[];
  pullRequests: readonly { number: number; checks: "pending" | "passed" | "failed"; said: string }[];
  jobs: readonly { id: string; number: number }[];
  /** What the search matched in this Session, where it matched an attachment. */
  matched?: OwnerChipRef;
  /** The clock time of the last turn, which the tooltip keeps. */
  lastTurn?: string;
  /** When it was, so the row can say how long ago. */
  lastTurnAt?: string;
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

const CHECKS: Record<"pending" | "passed" | "failed", LucideIcon> = { pending: ShieldEllipsis, passed: ShieldCheck, failed: ShieldX };

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
            {group.rows.map((row) => (
              <li key={row.id} className="armada-session-list__row" aria-label={row.title ?? row.address ?? row.id}>
                <button type="button" className="armada-session-list__open" onClick={() => onOpen(row.id)}>
                  <SessionMark state={row.state} said={row.said} />
                  <span className="armada-session-list__title">
                    {row.title ?? <span className="armada-session-list__id">{row.address ?? row.id}</span>}
                  </span>
                </button>
                <span className="armada-session-list__chips">
                  {row.slots.map((slot) => (
                    <Chip key={`slot${slot}`} chip={{ kind: "slot", slot }} matched={same(row.matched, { kind: "slot", slot })} />
                  ))}
                  {row.pullRequests.map((pr) => {
                    const Glyph = CHECKS[pr.checks];
                    return (
                      <span className="armada-session-list__pr" key={`pr${pr.number}`}>
                        <Chip chip={{ kind: "pull_request", number: pr.number }} matched={same(row.matched, { kind: "pull_request", number: pr.number })} />
                        <Tooltip label={pr.said}>
                          <span className="armada-session-list__checks" role="img" aria-label={pr.said}>
                            <Glyph size={12} strokeWidth={2} aria-hidden />
                          </span>
                        </Tooltip>
                      </span>
                    );
                  })}
                  {row.jobs.map((job) => (
                    <Chip key={`job${job.id}`} chip={{ kind: "job", id: job.id, number: job.number }} matched={same(row.matched, { kind: "job", id: job.id, number: job.number })} />
                  ))}
                  {row.matched?.kind === "branch" ? <Chip chip={row.matched} matched /> : null}
                  {row.lastTurn === undefined ? null : (
                    <Tooltip label={`Last turn ${row.lastTurn}`}>
                      <span className="armada-session-list__last" aria-label={`Last turn ${row.lastTurn}`}>
                        {(row.lastTurnAt === undefined ? undefined : since(row.lastTurnAt, now)) ?? row.lastTurn}
                      </span>
                    </Tooltip>
                  )}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
