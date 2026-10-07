// The Job's own log, opened on the line a Trigger's firing wrote. The panel is `log-sheet.tsx`'s,
// the one a Check's log opens in, so a log reads the same wherever it is opened from.
//
// **The line is found by `log_at` and the Trigger's name.** The log has no line numbers, so
// Fleet stamps a firing's line with the firing's own end, and the `trigger` field names it.

import type { JobTrigger, Journalled, Noted } from "@armada/protocol";

import { Log } from "./Log";
import { LogSheet } from "./log-sheet";
import { logOf } from "./mine";
import { notesOf } from "./notes";

/** The note a firing wrote, where the log holds it. */
export function noteOf(notes: readonly Noted[], trigger: JobTrigger): Noted | undefined {
  return notes.find(
    (one) => one.at === trigger.log_at && (one.fields ?? []).some((field) => field.name === "trigger" && field.value === trigger.name),
  );
}

export function TriggerLogSheet({
  trigger,
  jobId,
  journalled,
  floor,
  onClose,
}: {
  trigger: JobTrigger;
  jobId: string;
  journalled: Journalled;
  floor: boolean;
  onClose: () => void;
}) {
  const notes = logOf(journalled, jobId)?.notes ?? [];
  const at = noteOf(notes, trigger);
  return (
    <LogSheet kind="trigger-log" placement="floating" floor={floor} title="Job log" about={trigger.name} live={false} grows={notes.length} onClose={onClose}>
      <Log rows={notesOf(notes)} region="Job log" {...(at === undefined ? {} : { opens: `note-${at.seq}` })} />
    </LogSheet>
  );
}
