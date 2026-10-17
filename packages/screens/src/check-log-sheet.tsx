// One merge line Check's log, in the log panel (owner, 2 Oct 2026: *click on the specific check to
// open a panel and see the live log of the check*). It is followed over `observe_land_check`, which
// serves an ended one too, so it is the one socket either way. It draws as console lines in
// `log-sheet.tsx`'s panel.
//
// **Nothing stands in for lines that have not arrived.** A log still opening, or one its Check has
// printed nothing to yet, is an empty panel; only a failure says anything. **No count over the
// lines either** (`design-system.md`, hard rule 7): each line carries the file's own number.

import { useEffect, useState, type ReactNode } from "react";

import { ConsoleOutput, type ConsoleRow } from "@armada/components";
import type { FollowedLandLog, LandCheckAt } from "@armada/protocol";

import { LogSheet } from "./log-sheet";
import { followFailedNote } from "./outputs";

/** What the panel is titled. */
const TITLE = "Check log";

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
  bands,
  floor,
  onClose,
}: {
  about: string;
  live: boolean;
  rows: ConsoleRow[];
  emptyNote?: string;
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
      bleed
      wrap={{ wrap, onToggle: () => setWrap((was) => !was) }}
      {...(bands === undefined ? {} : { bands })}
      floor={floor}
      onClose={onClose}
    >
      <ConsoleOutput rows={rows} following={live} {...(emptyNote === undefined ? {} : { emptyNote })} wrap={wrap} />
    </LogSheet>
  );
}
