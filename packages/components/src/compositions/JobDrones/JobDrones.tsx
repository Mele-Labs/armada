import type { ReactNode } from "react";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Sheet } from "../../primitives/Sheet/Sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";
import { DroneTurns, type DroneTurn } from "../DroneTurns/DroneTurns";

/**
 * Job drones — every Drone a Job has used, and one of them read whole.
 *
 * **A list of Drones, not of tasks.** One task can have had two — the first
 * ended by hand, the second finishing it — and a row per task would hide the
 * one somebody is looking for.
 *
 * **Job ledger's panel, and its sheet.** A bordered panel with its filter menu
 * in the head, a table under it, and a pressed row read in the floating sheet
 * the owner approved on the Record (29 Sep 2026). The sheet holds the Drone's
 * whole transcript, following the tail while it runs, and the redirect box at
 * its foot.
 */

/** Where a Drone is. `failed` stopped on its own; `killed` was ended by hand. */
export type JobDroneState = "running" | "done" | "failed" | "killed";

export type JobDronesRow = {
  /** What a selection names. The Drone's id. */
  id: string;
  /** Which Drone, as the rest of job detail names it — `Drone on T5`. */
  drone: ReactNode;
  /** The step and task it worked. */
  where: ReactNode;
  state: JobDroneState;
  /** The state, spelled. */
  stateSays: string;
  /** Turns, and cost once it stopped. Absent draws nothing. */
  spent?: ReactNode;
  /** How long it has run — to now while it runs, to its end once stopped. */
  ranFor?: ReactNode;
  /** When it was spawned, the full instant, for the pointer. */
  sinceExact?: string;
};

export type JobDronesFilter = { id: string; label: string; count: number };

/** One Drone, read whole in the sheet. */
export type JobDroneReading = {
  title: string;
  /** The line under the title: where, state, and what it spent. */
  subtitle: ReactNode;
  turns: DroneTurn[];
  /** The Drone is still writing, so the transcript opens at its tail. */
  live: boolean;
  /** What the transcript says with no rows. */
  emptyNote: string;
  /** The redirect box, where this Drone can be reached. */
  footer?: ReactNode;
};

export type JobDronesProps = {
  rows: readonly JobDronesRow[];
  filters: readonly JobDronesFilter[];
  filter: string;
  onFilter: (id: string) => void;
  /** Controls beside the filter menu — the panel's own, not filters. */
  controls?: ReactNode;
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
  /** The open Drone. Absent while none is. */
  reading?: JobDroneReading;
  emptyNote?: ReactNode;
  /** The window is at `--window-floor`, where the sheet goes flush. */
  floor?: boolean;
};

export function JobDrones({
  rows,
  filters,
  filter,
  onFilter,
  controls,
  openRow = null,
  onOpenRow,
  reading,
  emptyNote = "No Drone under this filter",
  floor = false,
}: JobDronesProps) {
  const chosen = filters.find((one) => one.id === filter);
  return (
    <>
      <section className="armada-drones armada-glass" aria-label="Drones on this Job">
        <div className="armada-drones__head">
          <DropdownMenu
            align="start"
            triggerLabel={chosen?.label ?? filter}
            {...(chosen === undefined ? {} : { triggerCount: chosen.count })}
            entries={filters.map((one) => ({
              kind: "item",
              id: one.id,
              label: one.label,
              count: one.count,
              selected: one.id === filter,
            }))}
            onSelect={onFilter}
          />
          {controls}
        </div>

        {rows.length === 0 ? (
          <p className="armada-drones__note" role="note">
            {emptyNote}
          </p>
        ) : (
          <div className="armada-drones__scroll">
            <Table className="armada-drones__table">
              <TableHead>
                <TableRow>
                  <TableHeaderCell>Drone</TableHeaderCell>
                  <TableHeaderCell>Where</TableHeaderCell>
                  <TableHeaderCell>State</TableHeaderCell>
                  <TableHeaderCell>Spent</TableHeaderCell>
                  <TableHeaderCell>Run time</TableHeaderCell>
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
                    <TableCell className="armada-drones__drone">
                      {onOpenRow === undefined ? (
                        row.drone
                      ) : (
                        // The keyboard's path to the row's act. The press stops
                        // here, or the row answers it a second time.
                        <button
                          type="button"
                          className="armada-drones__open"
                          aria-expanded={row.id === openRow}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenRow(row.id);
                          }}
                        >
                          {row.drone}
                        </button>
                      )}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-drones__where">
                      {row.where}
                    </TableCell>
                    <TableCell className="armada-drones__state">
                      <span className="armada-drones__said" data-state={row.state}>
                        <span className="armada-drones__dot" aria-hidden />
                        {row.stateSays}
                      </span>
                    </TableCell>
                    <TableCell variant="metadata" className="armada-drones__spent">
                      {row.spent}
                    </TableCell>
                    <TableCell variant="metadata" className="armada-drones__ran" title={row.sinceExact}>
                      {row.ranFor}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </section>

      {/* Floating over the whole work area, the Record's own sheet. */}
      <Sheet
        open={reading !== undefined}
        floating
        floor={floor}
        size="wide"
        title={reading?.title ?? "Drone"}
        {...(reading === undefined ? {} : { subtitle: reading.subtitle })}
        closeLabel="Close"
        closeBinding="Esc"
        {...(reading?.footer === undefined
          ? {}
          : { footer: <div className="armada-drones__foot">{reading.footer}</div> })}
        onClose={() => onOpenRow?.(null)}
      >
        {reading === undefined ? null : (
          <DroneTurns
            key={reading.title}
            turns={reading.turns}
            emptyNote={reading.emptyNote}
            live={reading.live}
          />
        )}
      </Sheet>
    </>
  );
}
