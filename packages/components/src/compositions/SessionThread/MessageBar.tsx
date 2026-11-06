import { Reply } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { CopyAct } from "../../primitives/Prose/RunnableBlock";
import type { QuickResponse } from "../../primitives/Prose/responses";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Under an agent's message in the spacious take, shown while the pointer is over it or a key has reached it: Copy the
 * message, Reply with it quoted, the quick responses, and when it was said.
 */
export function MessageBar({
  text,
  onReply,
  responses,
  onPick,
  marked,
  at,
}: {
  /** The message's own words, as the agent wrote them. */
  text: string;
  onReply?: () => void;
  responses: readonly QuickResponse[];
  onPick?: (response: QuickResponse) => void;
  /** The emoji he already sent to this message. */
  marked?: string;
  at: ReactNode;
}) {
  return (
    <div className="armada-message-bar" role="toolbar" aria-label="Message">
      <CopyAct text={text} label="Copy message" />
      {onReply === undefined ? null : (
        <Tooltip label="Reply">
          <Button variant="ghost" size="sm" iconOnly aria-label="Reply" onClick={onReply}>
            <Reply size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {onPick === undefined || responses.length === 0 ? null : (
        <span className="armada-message-bar__responses">
          {responses.map((response) => (
            <Tooltip key={response.id} label={response.name}>
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={response.name}
                aria-pressed={marked === response.emoji}
                onClick={() => onPick(response)}
              >
                <span className="armada-message-bar__emoji" aria-hidden>
                  {response.emoji}
                </span>
              </Button>
            </Tooltip>
          ))}
        </span>
      )}
      {at}
    </div>
  );
}
