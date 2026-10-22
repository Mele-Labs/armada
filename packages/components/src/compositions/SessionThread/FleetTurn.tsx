import { useId, useState } from "react";
import { BellRing, ChevronRight } from "lucide-react";

import { Prose } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

type Message = Extract<SessionThreadRow, { kind: "message" }>;
type Notice = Extract<Message, { from: "session" }>;
type Tool = Extract<SessionThreadRow, { kind: "tool" }>;
export type TurnReply = Extract<Message, { from: "you" | "agent" }> | Tool;

/** What the notice is called on the folded row, from what it says. */
function nameOf(text: string): string {
  if (/commits? behind/i.test(text)) return "Merge main notice";
  if (/\bslot\b|\bslot-\d/i.test(text)) return "Slot notice";
  return "Fleet notice";
}

/**
 * A turn Fleet started: its notice, the agent's reply and the tool runs between. One dim row; a press opens the notice and the
 * reply under it, quieter than chat, and the same press folds it again.
 */
export function FleetTurn({ notice, reply }: { notice: Notice; reply: readonly TurnReply[] }) {
  const [open, setOpen] = useState(false);
  const body = useId();
  return (
    <li className="armada-fleet-turn" role="region" aria-label="Fleet turn" data-open={open}>
      <button type="button" className="armada-fleet-turn__row" aria-expanded={open} aria-controls={body} onClick={() => setOpen((now) => !now)}>
        <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-fleet-turn__chevron" />
        <Tooltip label="Fleet notice">
          <span role="img" aria-label="Fleet notice" className="armada-fleet-turn__mark">
            <BellRing size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <Tooltip label={notice.text}>
          <span className="armada-fleet-turn__label">{nameOf(notice.text)} · handled</span>
        </Tooltip>
      </button>
      {open ? (
        <div id={body} className="armada-fleet-turn__body">
          <p className="armada-fleet-turn__notice">{notice.text}</p>
          {reply.map((one) =>
            one.kind === "tool" ? (
              <p key={one.id} className="armada-fleet-turn__tool">{one.text}</p>
            ) : (
              <div key={one.id} className="armada-fleet-turn__reply">
                <Prose text={one.text} />
              </div>
            ),
          )}
        </div>
      ) : null}
    </li>
  );
}
