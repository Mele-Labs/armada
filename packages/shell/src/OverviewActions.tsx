// Overview's menu: Refresh, Reported, Held disk, Settings and the two bulk
// sweeps, which have no other entrance in the app.
//
// **It carried Dispatch on its face until 28 Sep 2026, and the title bar
// carries the same act two rows above it** — one `onCompose`, reached twice
// (the owner: *"We dont need this button on the overview because its already
// right above it in the title bar"*). `SplitButton`'s own rule is that a
// surface carries a likely act or a disclosure and never both, so with the act
// gone the disclosure is all that is left and it is a plain menu. The 11 Sep
// ruling that a head carries one action and a menu still holds; the action
// moved rather than the ruling.
//
// **The two bulk acts still confirm on their own.** Delete cannot be undone,
// and a person reaching for Clear must never end up there by the same press.

import { Dialog, DropdownMenu, JOB_LIFECYCLE, type DropdownMenuEntry } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { useEffect, useRef, useState } from "react";

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

export function OverviewActions({
  jobs,
  live,
  refreshing,
  onRefresh,
  onReadReports,
  onReadWorktrees,
  onOpenSettings,
  onClearTerminal,
  onForgetTerminal,
  sweeping,
}: {
  /** Every Job Bridge holds, not just the rows drawn: clearing is Fleet's board. */
  jobs: readonly JobSummary[];
  live: boolean;
  refreshing: boolean;
  onRefresh: () => void;
  onReadReports: () => void;
  onReadWorktrees: () => void;
  /** Settings. Here, and not among the acts on a Job — Fleet's four limits
   *  answer for every Job Fleet will ever start, not for one row on the Board. */
  onOpenSettings: () => void;
  onClearTerminal: (jobIds: readonly string[]) => void;
  onForgetTerminal: (jobIds: readonly string[]) => void;
  /** Which bulk sweep Fleet has not answered yet. #1117. */
  sweeping: "clear" | "forget" | null;
}) {
  const [asking, setAsking] = useState<"clear" | "forget" | null>(null);
  // The dialog that asked stays up while the sweep it confirmed is out — its
  // confirm is the one control still on screen once the menu that opened it
  // has already closed, so it is what carries the wait. Closes once `sweeping`
  // answers, whichever way. #1117.
  //
  // **It closes on the answer, not on the absence of a sweep.** The condition
  // was `sweeping === null`, which is true before anything is sent: opening the
  // dialog scheduled the effect that shut it, so neither bulk act could be
  // confirmed at all. Found on 28 Sep 2026 by pressing it, which no test did.
  const out = useRef(false);
  useEffect(() => {
    if (sweeping !== null) {
      out.current = true;
      return;
    }
    if (!out.current) return;
    out.current = false;
    setAsking(null);
  }, [sweeping]);
  const { reclaimable, forgettable } = terminalOf(jobs);
  const clearNoun = reclaimable.length === 1 ? "job" : "jobs";
  const forgetNoun = forgettable.length === 1 ? "job's" : "jobs'";

  const entries: DropdownMenuEntry[] = [
    // Re-reads over the connection Bridge already holds. It does not
    // reconnect: the runtime-file path already retries on its own.
    { kind: "item", id: "refresh", label: refreshing ? "Refreshing" : "Refresh" },
    { kind: "item", id: "reports", label: "Reported" },
    { kind: "item", id: "worktrees", label: "Held disk" },
    { kind: "item", id: "settings", label: "Settings" },
    ...(reclaimable.length === 0
      ? []
      : ([
          { kind: "separator", id: "before-sweeps" },
          { kind: "item", id: "clear", label: `Clear ${reclaimable.length} finished ${clearNoun}` },
        ] satisfies DropdownMenuEntry[])),
    ...(forgettable.length === 0
      ? []
      : ([
          ...(reclaimable.length === 0 ? [{ kind: "separator", id: "before-delete" } as const] : []),
          {
            kind: "item",
            id: "forget",
            label: `Delete ${forgettable.length} ${forgetNoun} records`,
            danger: true,
          },
        ] satisfies DropdownMenuEntry[])),
  ];

  const chose: Record<string, () => void> = {
    refresh: onRefresh,
    reports: onReadReports,
    worktrees: onReadWorktrees,
    settings: onOpenSettings,
    clear: () => setAsking("clear"),
    forget: () => setAsking("forget"),
  };

  return (
    <>
      {/* The same reach the caret had: off while nothing is connected to ask,
          and off while a sweep is already out. */}
      <DropdownMenu
        triggerLabel="Everything else"
        entries={entries}
        disabled={!live || sweeping !== null}
        onSelect={(id) => chose[id]?.()}
      />
      <Dialog
        open={asking === "clear"}
        tone="destructive"
        title={`Clear ${reclaimable.length} finished ${clearNoun}?`}
        confirmLabel={sweeping === "clear" ? "Clearing…" : "Clear"}
        confirmDisabled={sweeping !== null}
        // Refuses a second press the way a pending Button does: the control
        // stays put, the wait is still on it, and Cancel does nothing rather
        // than abandoning a sweep already sent. #1117.
        onCancel={sweeping === null ? () => setAsking(null) : undefined}
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
        onCancel={sweeping === null ? () => setAsking(null) : undefined}
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
