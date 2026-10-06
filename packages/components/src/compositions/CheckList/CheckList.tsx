import type { ReactNode } from "react";

import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "../../primitives/Table/Table";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * Check list — every Check a Manifest has had requested or run, one row each,
 * and the details of the one that is open.
 *
 * **A mark and never a word for the state** (owner, 2 Oct 2026), named by its
 * tooltip. Running is the step mark's pulsing circle-dot, and it stops under
 * reduced motion with the rest. No count is drawn over the rows: they are the
 * count.
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
  /** When it started. Absent on a Check still waiting. */
  started?: string;
  /** The full instant, for the pointer. */
  startedExact?: string;
  /** How long it ran. Absent while it is out or waiting. */
  duration?: string;
};

export type CheckListProps = {
  rows: readonly CheckListRow[];
  openRow?: string | null;
  onOpenRow?: (id: string) => void;
};

export function CheckList({ rows, openRow = null, onOpenRow }: CheckListProps) {
  if (rows.length === 0) return null;
  return (
    <section className="armada-check-list armada-glass" aria-label="Checks">
      <div className="armada-check-list__scroll">
        <Table className="armada-check-list__table">
          <TableHead>
            <TableRow>
              <TableHeaderCell>State</TableHeaderCell>
              <TableHeaderCell>Check</TableHeaderCell>
              <TableHeaderCell>Started</TableHeaderCell>
              <TableHeaderCell>Took</TableHeaderCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow
                key={row.id}
                selected={row.id === openRow}
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
                <TableCell variant="metadata" title={row.startedExact}>
                  {row.started}
                </TableCell>
                <TableCell variant="metadata">{row.duration}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}

/** One fact about a Check, labelled. */
export type CheckDetail = {
  label: string;
  value: ReactNode;
  /** A command or a path: machine-derived, so mono. */
  mono?: boolean;
};

/**
 * Every fact a Check has, as labelled values. A fact the Check does not have is
 * left out by the caller: an empty slot stays empty.
 */
export function CheckDetails({ details }: { details: readonly CheckDetail[] }) {
  return (
    <dl className="armada-check-details">
      {details.map((one) => (
        <div className="armada-check-details__fact" key={one.label}>
          <dt>{one.label}</dt>
          <dd data-mono={one.mono || undefined}>{one.value}</dd>
        </div>
      ))}
    </dl>
  );
}
