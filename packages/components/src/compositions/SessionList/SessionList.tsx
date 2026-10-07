import { Search } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Chip, type OwnerChipRef } from "../OwnerChip/OwnerChip";
import { SessionMark, hueOf, type SessionState } from "../SessionFrame/SessionFrame";

/**
 * Sessions, searchable. **Search finds by PR number, branch, Job id, slot or
 * title**, and the host does the finding: the list draws what it was given and
 * what in each row matched, as the chip it is.
 *
 * **The count is not drawn.** Rows are the list; a number beside them says what
 * the eye already reads.
 */
export type SessionRowView = {
  id: string;
  title?: string;
  state: SessionState;
  said: string;
  /** What the search matched in this Session, where it matched an attachment. */
  matched?: OwnerChipRef;
  lastTurn?: string;
};

export type SessionListProps = {
  rows: readonly SessionRowView[];
  query: string;
  onQuery: (query: string) => void;
  onOpen: (id: string) => void;
  onStart: () => void;
};

export function SessionList({ rows, query, onQuery, onOpen, onStart }: SessionListProps) {
  return (
    <section className="armada-session-list" aria-label="Sessions">
      <div className="armada-session-list__head">
        <h2 className="armada-session-list__eyebrow">Sessions</h2>
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
      <ul className="armada-session-list__rows">
        {rows.map((row) => (
          <li key={row.id} className="armada-session-list__row" data-hue={hueOf(row.state)} aria-label={row.title ?? row.id}>
            <button type="button" className="armada-session-list__open" onClick={() => onOpen(row.id)}>
              <SessionMark state={row.state} said={row.said} />
              <span className="armada-session-list__title">
                {row.title ?? <span className="armada-session-list__id">{row.id}</span>}
              </span>
            </button>
            <span className="armada-session-list__chips">
              {row.matched === undefined ? null : <Chip chip={row.matched} matched />}
              {row.lastTurn === undefined ? null : (
                <Tooltip label="Last turn">
                  <span className="armada-session-list__last" aria-label={`Last turn ${row.lastTurn}`}>
                    {row.lastTurn}
                  </span>
                </Tooltip>
              )}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
