// The two bulk sweeps over finished Jobs: Clear and Delete records. They sit
// on Cleanup's head and in the command palette — the owner's note of 1 Oct
// 2026 took away the Overview menu that used to carry them.
//
// **The confirmations live here and not on Cleanup**, because the palette can
// ask for one from any surface. Delete cannot be undone, and a person
// reaching for Clear must never end up there by the same press.

import { Button, Dialog, JOB_LIFECYCLE } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { useEffect, useRef } from "react";

/** Which bulk sweep: Clear gives the disk back, Forget deletes the records. */
export type Sweep = "clear" | "forget";

/** Which finished Jobs each bulk act would reach. */
export function terminalOf(jobs: readonly JobSummary[]): {
  reclaimable: string[];
  forgettable: string[];
} {
  const terminal = jobs.filter((job) => JOB_LIFECYCLE[job.status]?.terminal === true);
  return {
    // `Clear` only reaches a Job that has not already given its disk back, so
    // a second press is never a press that does nothing.
    reclaimable: terminal.filter((job) => job.reclaimed_at === undefined).map((job) => job.id),
    // `Delete record` reaches every terminal Job, reclaimed or not: deleting
    // the record a reclaim kept is the whole of what the act is for.
    forgettable: terminal.map((job) => job.id),
  };
}

/**
 * The sweeps there is anything to sweep with, and what each is called. One
 * list, so Cleanup's buttons and the palette's rows cannot word them apart.
 */
export function sweepsOf(jobs: readonly JobSummary[]): { id: Sweep; label: string }[] {
  const { reclaimable, forgettable } = terminalOf(jobs);
  return [
    ...(reclaimable.length === 0
      ? []
      : [{ id: "clear" as const, label: `Clear ${reclaimable.length} finished ${reclaimable.length === 1 ? "job" : "jobs"}` }]),
    ...(forgettable.length === 0
      ? []
      : [{ id: "forget" as const, label: `Delete ${forgettable.length} ${forgettable.length === 1 ? "job's" : "jobs'"} records` }]),
  ];
}

/** Cleanup's head: one button per sweep there is anything to sweep with. */
export function SweepButtons({
  jobs,
  live,
  sweeping,
  onAsk,
}: {
  jobs: readonly JobSummary[];
  live: boolean;
  /** Which bulk sweep Fleet has not answered yet. #1117. */
  sweeping: Sweep | null;
  onAsk: (sweep: Sweep) => void;
}) {
  return sweepsOf(jobs).map((sweep) => (
    // Off while nothing is connected to ask, and while a sweep is already out.
    <Button key={sweep.id} size="sm" disabled={!live || sweeping !== null} onClick={() => onAsk(sweep.id)}>
      {sweep.label}
    </Button>
  ));
}

/** The confirmation each sweep asks for, wherever it was asked from. */
export function SweepDialogs({
  jobs,
  asking,
  sweeping,
  onDone,
  onClearTerminal,
  onForgetTerminal,
}: {
  /** Every Job Bridge holds, not just the rows drawn: clearing is Fleet's board. */
  jobs: readonly JobSummary[];
  /** The sweep a person asked for and has not confirmed or cancelled. */
  asking: Sweep | null;
  /** Which bulk sweep Fleet has not answered yet. #1117. */
  sweeping: Sweep | null;
  /** Cancelled, or answered. */
  onDone: () => void;
  onClearTerminal: (jobIds: readonly string[]) => void;
  onForgetTerminal: (jobIds: readonly string[]) => void;
}) {
  // The dialog that asked stays up while the sweep it confirmed is out, and
  // carries the wait. Closes once `sweeping` answers, whichever way. #1117.
  //
  // **It closes on the answer, not on the absence of a sweep.** The condition
  // was `sweeping === null`, which is true before anything is sent: opening the
  // dialog scheduled the effect that shut it, so neither bulk act could be
  // confirmed at all. Found on 28 Sep 2026 by pressing it, which no test did.
  const out = useRef(false);
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    if (sweeping !== null) {
      out.current = true;
      return;
    }
    if (!out.current) return;
    out.current = false;
    done.current();
  }, [sweeping]);
  const { reclaimable, forgettable } = terminalOf(jobs);
  const clearNoun = reclaimable.length === 1 ? "job" : "jobs";
  const forgetNoun = forgettable.length === 1 ? "job's" : "jobs'";

  return (
    <>
      <Dialog
        open={asking === "clear"}
        tone="destructive"
        title={`Clear ${reclaimable.length} finished ${clearNoun}?`}
        confirmLabel={sweeping === "clear" ? "Clearing…" : "Clear"}
        confirmDisabled={sweeping !== null}
        // Refuses a second press the way a pending Button does: the control
        // stays put, the wait is still on it, and Cancel does nothing rather
        // than abandoning a sweep already sent. #1117.
        onCancel={sweeping === null ? onDone : undefined}
        onConfirm={() => onClearTerminal(reclaimable)}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded, ${reclaimable.length} right now, has `}
          its worktree and branch given back. The job and everything it recorded, its log, its
          checks and its judgments, stay on the board under Cleared.
        </p>
        <p>
          A branch holding commits the base cannot reach is left standing rather than deleted,
          and you are told which ones.
        </p>
      </Dialog>
      <Dialog
        open={asking === "forget"}
        tone="destructive"
        title={`Delete ${forgettable.length} finished ${forgetNoun} records?`}
        confirmLabel={sweeping === "forget" ? "Deleting…" : "Delete"}
        confirmDisabled={sweeping !== null}
        onCancel={sweeping === null ? onDone : undefined}
        onConfirm={() => onForgetTerminal(forgettable)}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded, ${forgettable.length} right now, is `}
          removed from the board along with its whole record. There is no undo, and a deleted job
          cannot be opened again.
        </p>
        <p>Its worktree and branch are left as its drone left them, or as a reclaim already left them.</p>
      </Dialog>
    </>
  );
}
