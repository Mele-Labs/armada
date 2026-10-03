import type { ReactNode } from "react";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Sheet, type SheetBack } from "../../primitives/Sheet/Sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";
import { DroneTurns, type DroneTurn } from "../DroneTurns/DroneTurns";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

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

/**
 * A Drone's state as the mark its step would carry for the same claim — the
 * Workflow step panel's own borrowing. **A mark and never a word** (the owner,
 * 2 Oct 2026: *"I hate text over icons"*), named on hover and to a screen
 * reader; the running one pulses, and stops under reduced motion. The sheet's
 * subtitle takes the same mark.
 */
export const DRONE_ACTIVITY: Record<JobDroneState, StepActivity> = {
  running: "running",
  done: "advanced",
  failed: "failed",
  killed: "killed",
};

export type JobDronesRow = {
  /** What a selection names. The Drone's id. */
  id: string;
  /** Which Drone, as the rest of job detail names it — `Drone on T5`. */
  drone: ReactNode;
  /** The step and task it worked. */
  where: ReactNode;
  state: JobDroneState;
  /** The state, spelled — the mark's tooltip and accessible name. */
  stateSays: string;
  /**
   * A `running` Drone Fleet holds at rest — at the gate, or for a person — and
   * not at work. **Its mark holds still**: what pulses is what is working.
   */
  resting?: boolean;
  /** The model it ran on. Absent draws nothing: a Drone Fleet kept none for. */
  model?: ReactNode;
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
  /** What the transcript says with no rows. Absent draws nothing. */
  emptyNote?: string;
  /** The redirect box, where this Drone can be reached. */
  footer?: ReactNode;
  /** In the head beside Close: the kill, where this Drone can be ended. */
  controls?: ReactNode;
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
  /** What the table says with no rows. Absent draws nothing: an empty slot stays empty. */
  emptyNote?: ReactNode;
  /** The window is at `--window-floor`, where the sheet goes flush. */
  floor?: boolean;
  /** The way back, where a press elsewhere opened the sheet. `Sheet`'s slot. */
  back?: SheetBack | undefined;
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
  emptyNote,
  floor = false,
  back,
}: JobDronesProps) {
  const chosen = filters.find((one) => one.id === filter);
  return (
    <>
      <section className="armada-drones armada-glass" aria-label="Drones on this Job">
        <div className="armada-drones__head">
          <DropdownMenu
            align="start"
            // No count on the trigger: the rows it would count are drawn beneath it.
            triggerLabel={chosen?.label ?? filter}
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
          emptyNote === undefined ? null : (
            <p className="armada-drones__note" role="note">
              {emptyNote}
            </p>
          )
        ) : (
          <div className="armada-drones__scroll">
            <Table className="armada-drones__table">
              <TableHead>
                <TableRow>
                  <TableHeaderCell>Drone</TableHeaderCell>
                  <TableHeaderCell>Where</TableHeaderCell>
                  <TableHeaderCell>State</TableHeaderCell>
                  <TableHeaderCell>Model</TableHeaderCell>
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
                      <StepActivityMark
                        activity={DRONE_ACTIVITY[row.state]}
                        label={row.stateSays}
                        says={row.stateSays}
                        pulsing={row.state === "running" && row.resting !== true}
                      />
                    </TableCell>
                    <TableCell variant="metadata" className="armada-drones__model">
                      {row.model}
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
        kind="drone"
        open={reading !== undefined}
        floating
        floor={floor}
        size="wide"
        title={reading?.title ?? "Drone"}
        back={back}
        {...(reading === undefined ? {} : { subtitle: reading.subtitle })}
        {...(reading?.controls === undefined ? {} : { controls: reading.controls })}
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
            {...(reading.emptyNote === undefined ? {} : { emptyNote: reading.emptyNote })}
            live={reading.live}
            steps={false}
          />
        )}
      </Sheet>
    </>
  );
}
