import { Ban } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PLACE_GLYPH, placePhrase } from "./RunsPlaces";
import type { RunsPlace } from "./RunsPlaces";

export * from "./RunsPlaces";

/**
 * How a repository runs, by job: the command Fleet saw pass for it, the port it listened on, and the workspace it ran in. Every line is kept, and wears
 * the glyph of the place it is kept in. **Mock only**, drawn in the layer a repository's row in the picker opens.
 */
export type RunsRow = {
  id: string;
  job: string;
  /** The folder it ran in, as a path from the repository's root. Absent at the root. */
  workspace?: string;
  command?: string;
  port?: number;
};

/** One line of a job: its workspace where the list has any, the job, the command and port, and where it is kept. */
export function RunLine({
  row,
  place,
  workspaces,
  onDrop,
  tabIndex,
}: {
  row: RunsRow;
  place: RunsPlace;
  /** The list names workspaces, so a root line keeps the column empty. */
  workspaces: boolean;
  /** Drops the line, so it is never kept again. Drawn as a button, and `x` on the line does it. */
  onDrop?: (() => void) | undefined;
  tabIndex?: number | undefined;
}) {
  const Glyph = PLACE_GLYPH[place.id];
  const word = `Kept ${placePhrase(place)}`;
  const where = row.workspace === undefined ? "" : `${row.workspace} `;
  return (
    <li className="armada-runs__row" aria-label={`${where}${row.job}`} data-row={row.id} data-workspaces={workspaces || undefined} data-drop={onDrop !== undefined || undefined} tabIndex={tabIndex}>
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
