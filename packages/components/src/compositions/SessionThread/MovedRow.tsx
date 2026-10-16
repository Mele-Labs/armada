import { ChevronRight, FolderGit2 } from "lucide-react";

import type { SessionThreadRow } from "./SessionThread";

/** Where a moved Session went, at the foot of its thread, or where it came from, at the head of the new one: a press opens the other. */
export function MovedRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "moved" }>; onOpen: (sessionId: string) => void }) {
  const said = `Moved ${row.way} ${row.repository}`;
  return (
    <li className="armada-session-moved">
      <button type="button" className="armada-session-moved__press" aria-label={said} onClick={() => onOpen(row.session)}>
        <FolderGit2 size={12} strokeWidth={2} aria-hidden />
        <span>{`Moved ${row.way}`}</span>
        <span className="armada-session-moved__repository">{row.repository}</span>
        <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-moved__go" />
      </button>
    </li>
  );
}
