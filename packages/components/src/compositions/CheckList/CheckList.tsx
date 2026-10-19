import { useRef, type ReactNode } from "react";

import { useTileGrid } from "../../tile-grid";
import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "../../primitives/Table/Table";
import { Stamp, StampSurface } from "../../primitives/Stamp/Stamp";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * Check list — every Check a Manifest has had requested or run, one row each,
 * and the details of the one that is open.
 *
 * **A mark and never a word for the state** (owner, 2 Oct 2026), named by its
 * tooltip. Running is the step mark's pulsing circle-dot, and it stops under
 * reduced motion with the rest. No count is drawn over the rows: they are the
 * count.
 *
 * **Drawn as the cockpit's glass**: a sunken glass under a raised band holding
 * the host's filters, each Check a tile framed in its run token's hue, and a
 * failed one lit. The rows are one Tab stop, walked by `useTileGrid`.
 */

/** Where a Check is: asked for and not started, out now, or ended. */
export type CheckListStatus = "waiting" | "running" | "passed" | "failed" | "stopped";

/** The step mark each status borrows, for the claim it makes. */
const ACTIVITY: Record<CheckListStatus, StepActivity> = {
  waiting: "not_started",
  running: "running",
  passed: "advanced",
  failed: "failed",
  stopped: "stopped",
};

export type CheckListRow = {
  /** What a selection names. */
  id: string;
  /** The Check, as the Manifest names it. */
  name: ReactNode;
  status: CheckListStatus;
  /** The state, spelled: the mark's tooltip and accessible name. */
  says: string;
  /** Who asked for it, in a line. */
  by?: ReactNode;
  /** When it started, an RFC 3339 instant. Absent on a Check still waiting. */
  startedAt?: string;
  /** How long it ran. Absent while it is out or waiting. */
  duration?: string;
};

export type CheckListProps = {
  rows: readonly CheckListRow[];
  openRow?: string | null;
  onOpenRow?: (id: string) => void;
  /** What the band over the glass holds: the host's filters. With it, the band and glass are drawn though no row is. */
  controls?: ReactNode;
};

export function CheckList({ rows, openRow = null, onOpenRow, controls }: CheckListProps) {
  const list = useRef<HTMLElement>(null);
  // One Tab stop for the rows; the arrows and j/k choose one, and Enter or o opens it.
  useTileGrid(list, { tile: ".armada-check-list__row", cursor: ".armada-check-list__open" });
  if (rows.length === 0 && controls === undefined) return null;
  return (
    <section className="armada-check-list" aria-label="Checks" ref={list}>
      <StampSurface>
        {controls === undefined ? null : <div className="armada-check-list__band">{controls}</div>}
        <div className="armada-check-list__scroll">
          {rows.length === 0 ? null : (
          <Table className="armada-check-list__table">
            <TableHead>
              <TableRow>
                <TableHeaderCell>State</TableHeaderCell>
                <TableHeaderCell>Check</TableHeaderCell>
                <TableHeaderCell>Requested by</TableHeaderCell>
                <TableHeaderCell>Started</TableHeaderCell>
                <TableHeaderCell>Took</TableHeaderCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.map((row) => (
                <TableRow
                  key={row.id}
                  className="armada-check-list__row"
                  selected={row.id === openRow}
                  aria-current={row.id === openRow || undefined}
                  data-status={row.status}
                  data-row-id={row.id}
                  onClick={onOpenRow === undefined ? undefined : () => onOpenRow(row.id)}
                >
                  <TableCell className="armada-check-list__state">
                    <StepActivityMark
                      activity={ACTIVITY[row.status]}
                      label={row.says}
                      says={row.says}
                      pulsing={row.status === "running"}
                    />
                  </TableCell>
                  <TableCell variant="mono">
                    {onOpenRow === undefined ? (
                      row.name
                    ) : (
                      // The keyboard's path to the row's act; the press stops
                      // here or the row answers it twice.
                      <button
                        type="button"
                        className="armada-check-list__open"
                        aria-expanded={row.id === openRow}
                        onClick={(event) => {
                          event.stopPropagation();
                          onOpenRow(row.id);
                        }}
                      >
                        {row.name}
                      </button>
                    )}
                  </TableCell>
                  <TableCell variant="secondary">{row.by}</TableCell>
                  <TableCell variant="metadata">{row.startedAt === undefined ? null : <Stamp at={row.startedAt} />}</TableCell>
                  <TableCell variant="metadata">{row.duration}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          )}
        </div>
      </StampSurface>
    </section>
  );
}

/** One fact about a Check, labelled. */
export type CheckDetail = {
  label: string;
  /** A command or a path: machine-derived, so mono. */
  mono?: boolean;
} & (
  | { value: ReactNode }
  /** An RFC 3339 instant, drawn as a stamp in the owner's time format. */
  | { at: string }
);

/**
 * Every fact a Check has, as labelled values. A fact the Check does not have is
 * left out by the caller: an empty slot stays empty.
 */
export function CheckDetails({ details }: { details: readonly CheckDetail[] }) {
  return (
    <StampSurface>
      <dl className="armada-check-details">
        {details.map((one) => (
          <div className="armada-check-details__fact" key={one.label}>
            <dt>{one.label}</dt>
            <dd data-mono={one.mono || undefined}>{"at" in one ? <Stamp at={one.at} /> : one.value}</dd>
          </div>
        ))}
      </dl>
    </StampSurface>
  );
}
