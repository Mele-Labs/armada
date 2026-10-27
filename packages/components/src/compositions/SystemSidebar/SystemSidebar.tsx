import { useRef, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SearchField } from "../SearchField/SearchField";
import { columnKeys } from "./columns";

/** One page of System, in the sidebar. `alert` draws the needs-you dot, with this as its tooltip. */
export type SystemEntry = { id: string; label: string; alert?: string };

/** A captioned run of entries. `act` is one press beside the caption, `name` its accessible name, since the caption is hidden from the tree; a search hides it. */
export type SystemGroup = { id: string; label: string; entries: readonly SystemEntry[]; act?: { label: string; name: string; onPress: () => void } };

/**
 * System as one page: a sidebar of every entry under three captions, and the picked entry beside it.
 * The search sits at the top of the sidebar, and the page's keys are `columns.ts`.
 *
 * **What the search keeps is the caller's** (`query` is its own state), so a search survives leaving
 * System and coming back.
 */
export type SystemSidebarProps = {
  groups: readonly SystemGroup[];
  /** The entry drawn, by id. */
  value: string;
  onValue: (id: string) => void;
  query: string;
  onQuery: (query: string) => void;
};

export function SystemSidebar({ groups, value, onValue, query, onQuery }: SystemSidebarProps) {
  const shown = groups.filter((group) => group.entries.length > 0);
  const picked = shown.some((group) => group.entries.some((one) => one.id === value));
  const first = shown[0]?.entries[0]?.id;
  return (
    <nav className="armada-system-side" aria-label="System" data-col="sidebar">
      <CardHeader>
        <CardTitle>System</CardTitle>
      </CardHeader>
      <SearchField label="Search System" value={query} onValue={onQuery} />
      <div className="armada-system-side__scroll">
      {shown.map((group) => (
        <div key={group.id} role="group" aria-label={group.label} className="armada-system-side__group">
          <div className="armada-system-side__head">
            <h3 className="armada-system-side__caption" aria-hidden="true">
              {group.label}
            </h3>
            {/* A search narrows the list to matches, and the act is never one. */}
            {group.act === undefined || query !== "" ? null : (
              <Button variant="ghost" size="sm" aria-label={group.act.name} onClick={group.act.onPress}>
                {group.act.label}
              </Button>
            )}
          </div>
          <ul className="armada-system-side__list">
            {group.entries.map((entry) => {
              const on = entry.id === value;
              return (
                <li key={entry.id}>
                  <button
                    type="button"
                    className="armada-system-side__entry"
                    data-nav-item=""
                    aria-current={on ? "page" : undefined}
                    tabIndex={on || (!picked && entry.id === first) ? 0 : -1}
                    onClick={() => onValue(entry.id)}
                  >
                    <span>{entry.label}</span>
                    {entry.alert === undefined ? null : (
                      <Tooltip label={entry.alert}>
                        <span className="armada-tabs__alert" role="img" aria-label={entry.alert} />
                      </Tooltip>
                    )}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
      </div>
    </nav>
  );
}

/**
 * The sidebar and the page beside it. The root takes the keys, so a row anywhere in it answers them.
 */
export function SystemPage({ side, children }: { side: ReactNode; children: ReactNode }) {
  const root = useRef<HTMLDivElement>(null);
  return (
    <div ref={root} className="armada-system" onKeyDown={(event) => columnKeys(event, root.current)}>
      {side}
      <div className="armada-system__body">{children}</div>
    </div>
  );
}
