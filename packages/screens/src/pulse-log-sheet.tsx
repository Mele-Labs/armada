// One of a Job's log files, read in a panel over Pulse while it is written.
//
// **Nothing here reads a file.** Both kinds that open already stream to this
// window, opened with the Job: a Drone's transcript is the observe socket's
// rows for that Drone (`Turn.drone_id`, protocol 21.4), drawn by `DroneTurns`,
// the Drones sheet's own pane, which follows the tail itself while `live`; the
// Job's own log is the second socket's notes, drawn by `Log`. A Judge or gaming
// brief has no reader on main, so its row offers the external `Open` alone.
//
// **Live is the reading's `being_written`**, not the socket's: the row and its
// panel carry the one mark, and both stop when `lsof` says nobody holds the
// file any more.

import { useLayoutEffect, useRef } from "react";

import { BeingWritten, DroneBrief, DroneTurns, Sheet, type PulseLogRow } from "@armada/components";
import type { Journalled, Observed } from "@armada/protocol";

import { Log } from "./Log";
import { logOf, turnsOf } from "./mine";
import { notesOf } from "./notes";
import { droneTurnsOf } from "./tab-drones-read";

export type PulseLogSheetProps = {
  /** The row whose panel is open, as the board last drew it; `null` is closed. */
  log: PulseLogRow | null;
  jobId: string;
  observed: Observed;
  journalled: Journalled;
  floor: boolean;
  onClose: () => void;
};

/** What a kind is called in the panel's head. The row's own words. */
const TITLE: Record<string, string> = { job: "Job log", transcript: "Drone transcript" };

export function PulseLogSheet({ log, jobId, observed, journalled, floor, onClose }: PulseLogSheetProps) {
  const body = useRef<HTMLDivElement>(null);
  const notes = log?.kind === "job" ? notesOf(logOf(journalled, jobId)?.notes ?? []) : [];
  const live = log?.writing === true;
  // The Job's log follows its tail while it is written; `DroneTurns` does this
  // for a transcript on its own.
  useLayoutEffect(() => {
    const el = body.current;
    if (live && el !== null) el.scrollTop = el.scrollHeight;
  }, [live, notes.length]);
  if (log === null || log.path === undefined) return null;
  const drone = droneOf(log.path);
  const rows = (turnsOf(observed, jobId)?.rows ?? []).filter((row) => row.drone_id === drone);
  return (
    <Sheet
      open
      contained
      size="wide"
      floor={floor}
      title={TITLE[log.kind] ?? log.kind}
      subtitle={
        <>
          {log.about ?? THE_JOBS_OWN}
          {live ? (
            <>
              {" "}
              <BeingWritten />
            </>
          ) : null}
        </>
      }
      closeLabel="Close"
      closeBinding="Esc"
      bodyRef={body}
      onClose={onClose}
    >
      {log.kind === "transcript" ? (
        <DroneTurns
          turns={droneTurnsOf(rows, (lines) => <DroneBrief lines={lines} flat />)}
          live={live}
          emptyNote={NOTHING_READ}
        />
      ) : (
        <Log rows={notes} emptyNote={NOTHING_READ} region="Job log" />
      )}
    </Sheet>
  );
}

/** A transcript's Drone, off its file name, `<drone-id>.jsonl` — Fleet's own stamp. */
function droneOf(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.endsWith(".jsonl") ? name.slice(0, -".jsonl".length) : name;
}

/** The Job's own log, where the row names no member. */
const THE_JOBS_OWN = "this job";

/** A file whose rows have not reached this window. Not an error: the socket may still be opening. */
const NOTHING_READ = "Nothing from this file has reached Bridge yet.";
