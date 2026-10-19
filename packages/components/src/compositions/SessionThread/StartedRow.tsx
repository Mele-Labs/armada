import { ChevronRight, CornerUpRight, GitFork } from "lucide-react";

import { Stamp } from "../../primitives/Stamp/Stamp";
import type { SessionThreadRow } from "./SessionThread";

/** The Session this one's agent started or forked, as a press in the thread that opens it. */
export function StartedRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "started" }>; onOpen: (sessionId: string) => void }) {
  const Glyph = row.fork ? GitFork : CornerUpRight;
  return (
    <li className="armada-session-started">
      <button type="button" className="armada-session-started__press" aria-label={row.session.title} onClick={() => onOpen(row.session.id)}>
        <Glyph size={14} strokeWidth={2} aria-hidden />
        <span className="armada-session-started__eyebrow">{row.fork ? "Forked" : "Started"}</span>
        <span className="armada-session-started__title">{row.session.title}</span>
        <Stamp still at={row.at} className="armada-session-started__at" />
        <ChevronRight size={14} strokeWidth={2} aria-hidden className="armada-session-started__go" />
      </button>
    </li>
  );
}
