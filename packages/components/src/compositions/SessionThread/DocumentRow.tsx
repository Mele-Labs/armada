import { FileText, Maximize2 } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";
import { ApprovalAnswers } from "./SketchRow";
import type { SketchAnswer } from "./sketch-answer";

/**
 * A page the Job wrote as its document, where in the thread it wrote it: a header band naming the page and its revision,
 * with a press at its end that opens the page over the Job. **Every revision is put to the person**, so the card holds Approve and Send back
 * under the band while the newest waits, and how it ended once answered. An earlier revision nobody answered is dashed
 * and stands back, as a replaced sketch does. It wears the sketch card's frame, so the two read as one kind of thing.
 */
export function DocumentRow({ row, keep, onOpen, onAnswer }: { row: Extract<SessionThreadRow, { kind: "document" }>; keep?: string; onOpen?: (page: string) => void; onAnswer?: (page: string, answer: SketchAnswer) => void }) {
  const settled = row.settled;
  const open = row.asked === true && settled === undefined && row.replaced !== true && onAnswer !== undefined;
  const revision = `Revision ${String(row.revision)}`;
  return (
    <li className="armada-session-sketch armada-session-document" data-replaced={row.replaced || undefined} data-asked>
      <div className="armada-session-sketch__asked" role="article" aria-label={`Document: ${row.title}, ${revision}`} data-settled={settled?.how}>
        <div className="armada-session-sketch__band">
          <FileText size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-sketch__title">{row.title}</span>
          <Tooltip label={revision}>
            <span className="armada-session-sketch__state">r{row.revision}</span>
          </Tooltip>
          <Stamp still at={row.at} className="armada-session-sketch__at" />
          {onOpen === undefined ? null : (
            <Tooltip label="Open">
              <Button className="armada-session-document__open" variant="ghost" size="sm" iconOnly aria-label={`Open ${row.title}, ${revision}`} onClick={() => onOpen(row.page)}>
                <Maximize2 size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </div>
        {!open && settled === undefined ? null : (
          <div className="armada-session-sketch__answers">
            <ApprovalAnswers id={row.page} keep={keep ?? `document:${row.id}`} {...(settled === undefined ? {} : { settled })} open={open} {...(onAnswer === undefined ? {} : { onAnswer })} />
          </div>
        )}
      </div>
    </li>
  );
}
