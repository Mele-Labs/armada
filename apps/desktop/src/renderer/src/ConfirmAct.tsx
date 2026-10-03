// The confirmation every destructive act passes through — #1528. Lifted out of
// `App.tsx` whole: the words, the one act that collects a note, and nothing
// else. `App.tsx` keeps which act is being confirmed, because that is window
// state and this is the dialog for it.

import { Dialog, Textarea } from "@armada/components";
import {
  ACT_LABEL,
  CONFIRM,
  KILL_PROCESS,
  KILL_PROCESSES,
  RESTART_NOTE,
  type ConfirmableAct,
} from "@armada/screens";

/**
 * A Job act, or one of Pulse's two kills. **The kills carry what the title
 * names** — the process, or how many — because unlike a Job act their words
 * are not a constant, and #1647 has the dialog name the process.
 */
export type Confirming =
  | { act: ConfirmableAct; jobId: string; droneId?: string }
  | { act: "kill_process"; jobId: string; pid: number; command: string }
  | { act: "kill_processes"; jobId: string; count: number };

/** A Job act to confirm, naming one Drone of several where one was named (23.10). */
export function aJobAct(act: ConfirmableAct, jobId: string, droneId?: string): Confirming {
  return { act, jobId, ...(droneId === undefined ? {} : { droneId }) };
}

export type ConfirmActProps = {
  /** Nothing to confirm draws nothing. */
  confirming: Confirming | null;
  /** The restart note in progress, held by the caller so cancelling clears it. */
  restartNote: string;
  onRestartNote: (said: string) => void;
  onCancel: () => void;
  onConfirm: (confirmed: Confirming) => void;
};

/**
 * **It states what happens and what survives rather than asking "are you
 * sure".** Cancel holds initial focus; the dialog owns that rule and this only
 * supplies the words.
 */
export function ConfirmAct({
  confirming,
  restartNote,
  onRestartNote,
  onCancel,
  onConfirm,
}: ConfirmActProps) {
  if (confirming === null) return null;
  // The kills' title is the whole of what they say: the process that ends, or
  // how many. Nothing survives a killed process worth a sentence here.
  if (confirming.act === "kill_process" || confirming.act === "kill_processes") {
    const killing = confirming;
    return (
      <Dialog
        open
        tone="destructive"
        title={
          killing.act === "kill_process"
            ? KILL_PROCESS.title(killing.command, killing.pid)
            : KILL_PROCESSES.title(killing.count)
        }
        confirmLabel={killing.act === "kill_process" ? KILL_PROCESS.confirm : KILL_PROCESSES.confirm}
        onCancel={onCancel}
        onConfirm={() => onConfirm(killing)}
      >
        {null}
      </Dialog>
    );
  }
  const act = confirming.act;
  return (
    <Dialog
      open
      tone={CONFIRM[act].tone ?? "destructive"}
      title={CONFIRM[act].title}
      confirmLabel={ACT_LABEL[act]}
      onCancel={onCancel}
      onConfirm={() => onConfirm(confirming)}
    >
      {CONFIRM[act].body}
      {/* The one confirmation that collects anything, and what it collects is
          optional — the button is never disabled on it, because leaving the
          field alone is the restart this dialog has always been. No
          `autoFocus`: the dialog puts initial focus on Cancel, and a second
          claim on it here would only lose to it. */}
      {act !== "restart_step" ? null : (
        <>
          <p>{RESTART_NOTE.says}</p>
          <Textarea
            label={RESTART_NOTE.label}
            rows={4}
            value={restartNote}
            onChange={(event) => onRestartNote(event.target.value)}
          />
        </>
      )}
    </Dialog>
  );
}
