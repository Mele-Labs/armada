import { useMemo, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { RunLine } from "./RepositoryRuns";
import type { RunsRow } from "./RepositoryRuns";
import { PLACE_GLYPH, placePhrase } from "./RunsPlaces";
import type { RunsPlace } from "./RunsPlaces";

/** The root of the repository has no folder to name it by. */
const ROOT = "Root";

type Workspace = { key: string; name: string; rows: readonly RunsRow[]; kinds: string };

/** The workspaces the commands ran in, in the order they were first seen, each with the kinds of command it has. */
function workspacesOf(rows: readonly RunsRow[]): Workspace[] {
  const seen = new Map<string, RunsRow[]>();
  for (const one of rows) seen.set(one.workspace ?? "", [...(seen.get(one.workspace ?? "") ?? []), one]);
  return [...seen].map(([key, held]) => ({ key, name: key === "" ? ROOT : key, rows: held, kinds: [...new Set(held.map((one) => one.job))].join(", ") }));
}

const has = (said: string, filter: string) => said.toLowerCase().includes(filter.trim().toLowerCase());

/**
 * All of how a repository runs, as a layer over Dispatch: the workspaces, each with the kinds of command it has, and a workspace's commands
 * one press in. J and K move, Enter opens, Esc goes back a level and then closes. Where the runs are kept sits in the header. **Mock only**.
 */
export function RunsAll({
  name,
  source,
  rows,
  place,
  onChange,
  changing,
  chooser,
  onClose,
}: {
  name: string;
  source: string;
  rows: readonly RunsRow[];
  place?: RunsPlace | undefined;
  /** Opens where the runs are kept, to change it. Drawn once there is a place. */
  onChange?: (() => void) | undefined;
  changing?: boolean | undefined;
  /** The places to choose from, under the header. */
  chooser?: ReactNode;
  onClose: () => void;
}) {
  const workspaces = useMemo(() => workspacesOf(rows), [rows]);
  // A repository with a single workspace opens at its commands, and Esc closes the layer from there.
  const [open, setOpen] = useState<string | null>(() => (workspaces.length === 1 ? workspaces[0]!.key : null));
  const [filter, setFilter] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLInputElement>(null);

  const one = open === null ? undefined : workspaces.find((it) => it.key === open);
  const listed = workspaces.filter((it) => has(`${it.name} ${it.kinds}`, filter));
  const commands = (one?.rows ?? []).filter((it) => has(`${it.job} ${it.command ?? ""}`, filter));
  const keep = place === undefined ? undefined : PLACE_GLYPH[place.id];
  const back = workspaces.length > 1;

  const items = () => [...(root.current?.querySelectorAll<HTMLElement>("button[data-item], li[data-row]") ?? [])];
  const go = (key: string | null) => {
    setOpen(key);
    setFilter("");
    requestAnimationFrame(() => field.current?.focus());
  };

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const typing = event.target instanceof HTMLInputElement;
    const filtering = event.target === field.current;
    // The bucket address in the places is a field of its own.
    if (typing && !filtering) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (open !== null && back) return go(null);
      return onClose();
    }
    const down = event.key === "ArrowDown" || (!typing && event.key === "j");
    const up = event.key === "ArrowUp" || (!typing && event.key === "k");
    if (down || up) {
      event.preventDefault();
      event.stopPropagation();
      const all = items();
      const at = all.indexOf(document.activeElement as HTMLElement);
      all[at < 0 ? (down ? 0 : all.length - 1) : (at + (down ? 1 : all.length - 1)) % all.length]?.focus();
      return;
    }
    if (event.key === "/" && !typing) {
      event.preventDefault();
      event.stopPropagation();
      field.current?.focus();
      return;
    }
    if (event.key === "Enter" && filtering && open === null && listed[0] !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      go(listed[0].key);
    }
  };

  const title = `All of how ${name} runs`;
  return (
    <div ref={root} className="armada-runs-all" role="dialog" aria-label={title} onKeyDown={keys}>
      <header className="armada-runs__head">
        <span className="armada-runs__title">{title}</span>
        {one === undefined || !back ? null : (
          <Button variant="ghost" size="sm" onClick={() => go(null)}>
            Workspaces
          </Button>
        )}
        <span className="armada-runs__source">{source}</span>
        {keep === undefined || place === undefined || onChange === undefined ? null : (
          <Tooltip label={`Kept ${placePhrase(place)}`}>
            <button type="button" className="armada-runs__keep" aria-label={`Kept ${placePhrase(place)}`} aria-expanded={changing === true} onClick={onChange}>
              {(() => {
                const Glyph = keep;
                return <Glyph size={12} strokeWidth={2} aria-hidden />;
              })()}
            </button>
          </Tooltip>
        )}
      </header>
      {chooser}
      <Input
        ref={field}
        aria-label={one === undefined ? "Filter workspaces" : `Filter ${one.name}`}
        mono
        autoFocus
        placeholder={one === undefined ? "Workspace or kind" : "Kind or command"}
        value={filter}
        onChange={(event) => setFilter(event.target.value)}
        trailing={<Kbd aria-hidden>/</Kbd>}
      />
      {one === undefined ? (
        <ul className="armada-runs__rows armada-runs-all__list" aria-label="Workspaces">
          {listed.map((it) => (
            <li key={it.key} className="armada-runs-all__item">
              <button type="button" className="armada-runs-all__workspace" data-item={it.key} onClick={() => go(it.key)}>
                <code className="armada-runs-all__name">{it.name}</code>
                <span className="armada-runs-all__kinds">{it.kinds}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <ul className="armada-runs__rows" aria-label={`Commands in ${one.name}`}>
          {commands.map((it, at) => (
            <RunLine key={it.id} row={it} place={place} workspaces={false} tabIndex={at === 0 ? 0 : -1} />
          ))}
        </ul>
      )}
    </div>
  );
}
