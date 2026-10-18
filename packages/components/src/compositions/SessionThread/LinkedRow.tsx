import { ChevronRight, Eye, GitMerge, Link, Power } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

const EVENT: Record<Extract<SessionThreadRow, { kind: "linked" }>["event"], { Glyph: LucideIcon; said: string }> = {
  needs: { Glyph: Eye, said: "Needs you" },
  done: { Glyph: Power, said: "Done" },
  merged: { Glyph: GitMerge, said: "Merged" },
};

/** A Session this one follows needs the person, finished or merged. The agent was told in the same moment, so the row stands in the thread where it happened. */
export function LinkedRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "linked" }>; onOpen: (sessionId: string) => void }) {
  const { Glyph, said } = EVENT[row.event];
  return (
    <li className="armada-session-linked" data-event={row.event}>
      <button type="button" className="armada-session-linked__press" aria-label={`${row.session.title}, ${said}`} onClick={() => onOpen(row.session.id)}>
        <Tooltip label="Linked Job">
          <span className="armada-session-linked__link" role="img" aria-label="Linked Job">
            <Link size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-session-linked__title">{row.session.title}</span>
        <Tooltip label={said}>
          <span className="armada-session-linked__event" role="img" aria-label={said}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-session-linked__at">{row.at}</span>
        <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-linked__go" />
      </button>
    </li>
  );
}
