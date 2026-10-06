// One Check's log, in the log panel: pressed from a boundary's Checks strip wherever it is drawn,
// a plan group's card, a task's Record row or a merge line's row (owner, 2 Oct 2026: *click on
// the specific check to open a panel and see the live log of the check*).
//
// **Two sources and one panel.** A Job's Check is read where its gate keeps it: followed over
// `observe_check_output` while the gate is writing it, fetched whole off `get_check_output` once it
// has ruled. A merge line's Check is followed over `observe_land_check`, which serves an ended one
// too, so it is the one socket either way. Both draw as console lines in `log-sheet.tsx`'s panel.
//
// **Nothing stands in for lines that have not arrived.** A log still opening, or one its Check has
// printed nothing to yet, is an empty panel; only a failure says anything. **No count over the
// lines either** (`design-system.md`, hard rule 7): each line carries the file's own number.

import { useEffect, useState, type ReactNode } from "react";

import { ConsoleOutput, type ConsoleRow } from "@armada/components";
import type { FollowedLandLog, LandCheckAt } from "@armada/protocol";

import { LogSheet } from "./log-sheet";
import { followFailedNote, liveRowsOf, rowsOf, type Following, type Outputs } from "./outputs";

/**
 * Where one of a Job's Checks keeps its log: the file's own name, which both reads take, and
 * whether the gate is still writing it. Fixed at the press, so a Check that ends while its panel
 * is open keeps the lines it was followed to.
 */
export type JobCheckLog = {
  name: string;
  kept: string;
  live: boolean;
  /**
   * The step attempt of the run whose file this is, where it is a kept run, so the sheet can go on
   * to that run's own Record row. Absent on a live log: no row is written until the gate rules.
   */
  stepAttempt?: number;
  /**
   * The group whose run it is, where Fleet names one (#1652): two groups gated on one run of a
   * step each have a row for the same Check, and this decides which the Record opens.
   */
  group?: string;
};

/** What the way on to a Check's own Record row says, as its tooltip and its name. */
const TO_THE_RECORD = "Open in the Record";

/** What both kinds of panel are titled. */
const TITLE = "Check log";

export function JobCheckLogSheet({
  log,
  outputs,
  following,
  floor,
  onOpenRecord,
  bands,
  onClose,
}: {
  log: JobCheckLog;
  outputs: Outputs;
  following: Following;
  floor: boolean;
  /** Go to this Check's own Record row. Absent where the Record holds none: no control is drawn. */
  onOpenRecord?: () => void;
  /** Facts of the Check, held under the head while the log scrolls. */
  bands?: ReactNode;
  onClose: () => void;
}) {
  const { follow, reading } = following;
  const { fetch } = outputs;
  // Opening the panel is the ask, once; closing it lets a followed one go.
  useEffect(() => {
    if (!log.live) {
      fetch(log.kept);
      return;
    }
    follow(log.kept);
    return () => follow(null);
  }, [log.live, log.kept, follow, fetch]);

  if (!log.live) {
    const held = outputs.of(log.kept);
    return (
      <CheckLogPanel
        about={log.name}
        live={false}
        rows={held?.state === "got" ? rowsOf(held.output) : []}
        {...(held?.state === "absent" ? { emptyNote: held.note } : {})}
        {...(onOpenRecord === undefined ? {} : { onOpenRecord })}
        {...(bands === undefined ? {} : { bands })}
        floor={floor}
        onClose={onClose}
      />
    );
  }

  const ours = reading.state !== "none" && reading.kept === log.kept ? reading : undefined;
  const ended = ours?.state === "following" ? ours.ended : undefined;
  const note = ours === undefined ? undefined : followFailedNote(ours.state === "failed", ended);
  return (
    <CheckLogPanel
      about={log.name}
      live={ours?.state === "following" && ended === undefined}
      rows={liveRowsOf(reading, log.kept)}
      {...(note === undefined ? {} : { emptyNote: note })}
      {...(onOpenRecord === undefined ? {} : { onOpenRecord })}
      {...(bands === undefined ? {} : { bands })}
      floor={floor}
      onClose={onClose}
    />
  );
}

/**
 * One merge line Check's log, followed while the runner writes it and read whole once it has ended,
 * on the one socket. Named by the line's three words and never a path.
 */
export function LandCheckLogSheet({
  at,
  followed,
  onFollow,
  bands,
  floor,
  onClose,
}: {
  at: LandCheckAt;
  followed: FollowedLandLog;
  onFollow: (at: LandCheckAt | null) => void;
  /** Facts of the Check, held under the head while the log scrolls. */
  bands?: ReactNode;
  floor: boolean;
  onClose: () => void;
}) {
  const { root, branch, check } = at;
  useEffect(() => {
    onFollow({ root, branch, check });
    return () => onFollow(null);
  }, [root, branch, check, onFollow]);

  const ours =
    followed.state !== "none" && followed.root === root && followed.branch === branch && followed.check === check
      ? followed
      : undefined;
  const following = ours?.state === "following" ? ours : undefined;
  const rows: ConsoleRow[] =
    following === undefined
      ? []
      : following.lines.map((text, n) => ({ row: "line" as const, at: following.fromLine + n, text }));
  const note = ours === undefined ? undefined : followFailedNote(ours.state === "failed", following?.ended);
  return (
    <CheckLogPanel
      about={`${check} · ${branch}`}
      live={following !== undefined && following.ended === undefined}
      rows={rows}
      {...(note === undefined ? {} : { emptyNote: note })}
      {...(bands === undefined ? {} : { bands })}
      floor={floor}
      onClose={onClose}
    />
  );
}

/** The panel and its console, wrapped where the reader asks. */
function CheckLogPanel({
  about,
  live,
  rows,
  emptyNote,
  onOpenRecord,
  bands,
  floor,
  onClose,
}: {
  about: string;
  live: boolean;
  rows: ConsoleRow[];
  emptyNote?: string;
  /** A merge line Check has no Record row, so it never passes one. */
  onOpenRecord?: () => void;
  bands?: ReactNode;
  floor: boolean;
  onClose: () => void;
}) {
  const [wrap, setWrap] = useState(false);
  return (
    <LogSheet
      kind="check-log"
      placement="floating"
      title={TITLE}
      about={<span className="mono">{about}</span>}
      live={live}
      grows={rows.length}
      wrap={{ wrap, onToggle: () => setWrap((was) => !was) }}
      {...(onOpenRecord === undefined ? {} : { goes: { label: TO_THE_RECORD, onGo: onOpenRecord } })}
      {...(bands === undefined ? {} : { bands })}
      floor={floor}
      onClose={onClose}
    >
      <ConsoleOutput rows={rows} following={live} {...(emptyNote === undefined ? {} : { emptyNote })} wrap={wrap} />
    </LogSheet>
  );
}
