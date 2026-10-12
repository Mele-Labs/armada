import { useRef, type KeyboardEvent, type ReactNode } from "react";

import { Input } from "../../primitives/Input/Input";

/** One category Settings is split into. `matches` is drawn beside it while a search is on. */
export type SettingsIndexSection = { id: string; label: string; matches?: number };

/**
 * Settings, split into categories a person can find: a search over every setting, the categories
 * down the side, and the one chosen beside them. **One category at a time**, so the screen is the
 * thing being changed and not a scroll past every other — the long list this replaced. While a
 * search is on, the categories that hold a match carry how many, and the body is the matches.
 *
 * The categories are tabs, vertical, because they are one choice of what the panel shows: arrows
 * move between them as they do in any tab row, and Home and End reach the ends.
 */
export type SettingsIndexProps = {
  sections: readonly SettingsIndexSection[];
  /** The category drawn, by id. Ignored while `query` is not empty: the body is the matches then. */
  current: string;
  onSection: (id: string) => void;
  query: string;
  onQuery: (query: string) => void;
  /** The body: the chosen category, or every match while a search is on. */
  children: ReactNode;
};

export function SettingsIndex({ sections, current, onSection, query, onQuery, children }: SettingsIndexProps) {
  const searching = query.trim() !== "";
  const tabs = useRef<HTMLDivElement>(null);
  const shown = searching ? sections.filter((one) => (one.matches ?? 0) > 0) : sections;

  const keyed = (event: KeyboardEvent<HTMLDivElement>) => {
    const at = shown.findIndex((one) => one.id === current);
    const to =
      event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? shown.length - 1 : null;
    if (to === null) return;
    const next = shown[(to + shown.length) % shown.length];
    if (next === undefined) return;
    event.preventDefault();
    onSection(next.id);
    tabs.current?.querySelector<HTMLButtonElement>(`[data-section="${next.id}"]`)?.focus();
  };

  return (
    <div className="armada-settings-index">
      <div className="armada-settings-index__grid">
        <div className="armada-settings-index__side">
          <Input
            type="search"
            aria-label="Search settings"
            placeholder="Search settings"
            value={query}
            onChange={(event) => onQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape" && query !== "") {
                event.preventDefault();
                event.stopPropagation();
                onQuery("");
              }
            }}
          />
          <div ref={tabs} role="tablist" aria-orientation="vertical" aria-label="Settings" className="armada-settings-index__tabs" onKeyDown={keyed}>
            {shown.map((one) => {
              const on = !searching && one.id === current;
              return (
                <button
                  key={one.id}
                  type="button"
                  role="tab"
                  data-section={one.id}
                  aria-selected={on}
                  tabIndex={on || (searching && one === shown[0]) ? 0 : -1}
                  className="armada-settings-index__tab"
                  onClick={() => {
                    onQuery("");
                    onSection(one.id);
                  }}
                >
                  <span className="armada-settings-index__label">{one.label}</span>
                  {searching ? <span className="armada-settings-index__count">{one.matches}</span> : null}
                </button>
              );
            })}
          </div>
          {searching && shown.length === 0 ? <p className="armada-settings-index__none">Nothing matches.</p> : null}
        </div>
        <div role="tabpanel" aria-label={searching ? "Matches" : (sections.find((one) => one.id === current)?.label ?? "Settings")} className="armada-settings-index__body">
          {children}
        </div>
      </div>
    </div>
  );
}
