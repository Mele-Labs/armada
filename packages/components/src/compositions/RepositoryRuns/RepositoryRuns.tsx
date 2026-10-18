import type { KeyboardEvent, ReactNode } from "react";
import { Ban, Check, Terminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PLACE_GLYPH, placePhrase } from "./RunsPlaces";
import type { RunsPlace } from "./RunsPlaces";

export * from "./RunsPlaces";

/**
 * How a repository runs, by job: the command Fleet saw pass for it, the port it listened on, and the workspace it ran in. A state is a glyph with a
 * tooltip, and a remembered one wears the glyph of the place it is kept in. **Mock only**, drawn in the dispatch panel.
 */
export type RunsRow = {
  id: string;
  job: string;
  /** The folder it ran in, as a path from the repository's root. Absent at the root. */
  workspace?: string;
  command?: string;
  port?: number;
  state: "seen" | "remembered";
};

const STATE: Record<RunsRow["state"], { Glyph: LucideIcon; word: string }> = {
  seen: { Glyph: Terminal, word: "Seen in a Job" },
  remembered: { Glyph: Check, word: "Remembered" },
};

/** The glyph and the word a row's state wears: a remembered row wears the place it is kept in. */
export function runsState(row: RunsRow, place: RunsPlace | undefined): { Glyph: LucideIcon; word: string } {
  return row.state === "remembered" && place !== undefined ? { Glyph: PLACE_GLYPH[place.id], word: `Remembered ${placePhrase(place)}` } : STATE[row.state];
}

/** One line of a job: its workspace where the list has any, the job, the command and port, and the state. */
export function RunLine({
  row,
  place,
  workspaces,
  onDrop,
  tabIndex,
}: {
  row: RunsRow;
  place?: RunsPlace | undefined;
  /** The list names workspaces, so a root line keeps the column empty. */
  workspaces: boolean;
  /** Drops the line, so it is not kept and not offered again. Drawn as a button, and `x` on the line does it. */
  onDrop?: (() => void) | undefined;
  tabIndex?: number | undefined;
}) {
  const { Glyph, word } = runsState(row, place);
  const where = row.workspace === undefined ? "" : `${row.workspace} `;
  return (
    <li className="armada-runs__row" aria-label={`${where}${row.job}`} data-state={row.state} data-row={row.id} data-workspaces={workspaces || undefined} data-drop={onDrop !== undefined || undefined} tabIndex={tabIndex}>
      {!workspaces ? null : <code className="armada-runs__workspace">{row.workspace ?? ""}</code>}
      <span className="armada-runs__job">{row.job}</span>
      <span className="armada-runs__how">
        <code className="armada-runs__command">{row.command ?? ""}</code>
        {row.port === undefined ? null : <code className="armada-runs__port">{`:${row.port}`}</code>}
      </span>
      <Tooltip label={word}>
        <span className="armada-runs__state" role="img" aria-label={`${where}${row.job}: ${word}`}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      {onDrop === undefined ? null : (
        <Tooltip label="Do not keep" shortcut="x">
          <button type="button" tabIndex={-1} className="armada-runs__drop" aria-label={`Do not keep ${where}${row.job}`} onClick={onDrop}>
            <Ban size={12} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      )}
    </li>
  );
}

/**
 * What is new in how a repository runs since its last Remember: one line each, in the workspace it ran in. J and K move between the lines and `x`
 * drops the one that has focus. **Mock only**, drawn in the dispatch panel.
 */
export function RepositoryRuns({
  name,
  source,
  rows,
  onDrop,
  onLeave,
  chooser,
  acts,
}: {
  name: string;
  source: string;
  /** The lines that are new. */
  rows: readonly RunsRow[];
  /** Drops a line: it is not kept, and not offered again. */
  onDrop?: ((id: string) => void) | undefined;
  /** Focus has nowhere in the group to go once its last line is dropped. */
  onLeave?: (() => void) | undefined;
  /** The places to choose from, in place under the rows. */
  chooser?: ReactNode;
  acts?: ReactNode;
}) {
  const workspaces = rows.some((one) => one.workspace !== undefined);

  const keys = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const here = (event.target as HTMLElement).closest<HTMLElement>("li[data-row]");
    if (here === null) return;
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>("li[data-row]")];
    const at = all.indexOf(here);
    if (event.key === "j" || event.key === "k" || event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      event.stopPropagation();
      all[(at + (event.key === "j" || event.key === "ArrowDown" ? 1 : all.length - 1)) % all.length]?.focus();
    } else if (event.key === "x" && onDrop !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      const next = all[at + 1] ?? all[at - 1];
      if (next === undefined) onLeave?.();
      else next.focus();
      onDrop(here.dataset.row!);
    }
  };

  return (
    <section className="armada-runs" role="group" aria-label={`How ${name} runs`}>
      <header className="armada-runs__head">
        <span className="armada-runs__title">How {name} runs</span>
        <span className="armada-runs__source">{source}</span>
      </header>
      <ul className="armada-runs__rows" onKeyDown={keys}>
        {rows.map((one, at) => (
          <RunLine key={one.id} row={one} workspaces={workspaces} tabIndex={at === 0 ? 0 : -1} onDrop={onDrop === undefined ? undefined : () => onDrop(one.id)} />
        ))}
      </ul>
      {chooser}
      {acts === undefined ? null : <div className="armada-runs__acts">{acts}</div>}
    </section>
  );
}
