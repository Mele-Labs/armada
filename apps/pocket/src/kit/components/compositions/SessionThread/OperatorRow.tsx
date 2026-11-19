import type { SessionThreadRow } from "./SessionThread";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Bot } from "lucide-react";
import { Stamp } from "../../primitives/Stamp/Stamp";

/** The Operator started this Job, sent it words or answered for the person, where it happened and in those exact words. */
export function OperatorRow({
  row,
}: {
  row: Extract<
    SessionThreadRow,
    {
      kind: "operator";
    }
  >;
}) {
  return (
    <li className="armada-session-operator">
      <Tooltip label="Operator">
        <span className="armada-session-operator__glyph" role="img" aria-label="Operator">
          <Bot size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-session-operator__text">{row.text}</span>
      <Stamp still at={row.at} className="armada-session-operator__at" />
    </li>
  );
}
