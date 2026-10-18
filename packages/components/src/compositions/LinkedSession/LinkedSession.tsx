import { Link } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { sessionGlyphOf, type SessionState } from "../SessionFrame/SessionFrame";
import { PullRequestChecksMark, PullRequestStateMark, type LedgerPullRequest } from "../SessionLedger/PullRequestMarks";

/**
 * A Session a parent follows, as one press that opens it, in two lines: the `link` glyph and the title, then in the muted mono sub-line the plan
 * group it was handed from where the row is not already under that group, its state as the one registry mark, and the pull request it holds.
 * **Drawn the same wherever the parent keeps its children**, on a Plan group or in the Ledger's Linked section.
 */
export type LinkedSessionProps = {
  title: string;
  /** The plan group's title, when the row does not stand under it. */
  group?: string;
  state: SessionState;
  /** The state's name, which the mark's tooltip says. */
  said: string;
  pr?: LedgerPullRequest & { number: number };
  onOpen: () => void;
};

const TONE: Partial<Record<SessionState, "ask" | "bad" | "done">> = { waiting: "ask", replied: "ask", stuck: "bad", failing: "bad", done: "done" };

export function LinkedSession({ title, group, state, said, pr, onOpen }: LinkedSessionProps) {
  const Glyph = sessionGlyphOf(state);
  return (
    <button type="button" className="armada-linked" data-tone={TONE[state]} aria-label={`Open ${title}`} onClick={onOpen}>
      <span className="armada-linked__head">
        <Tooltip label="Linked Job">
          <span className="armada-linked__link" role="img" aria-label="Linked Job">
            <Link size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-linked__title">{title}</span>
      </span>
      <span className="armada-linked__facts">
        {group === undefined ? null : (
          <Tooltip label="Handed from">
            <code className="armada-linked__group" aria-label={`Handed from ${group}`}>
              {group}
            </code>
          </Tooltip>
        )}
        <Tooltip label={said}>
          <span className="armada-session-mark armada-linked__state" role="img" aria-label={said} data-pulsing={state === "working" || undefined}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        {pr === undefined ? null : (
          <span className="armada-linked__pr">
            <code>#{pr.number}</code>
            <PullRequestStateMark merged={pr.merged} />
            <PullRequestChecksMark checks={pr.checks} said={pr.checksSaid} />
          </span>
        )}
      </span>
    </button>
  );
}
