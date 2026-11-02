import type { ReactNode } from "react";
import { ArrowRight, Ban } from "lucide-react";

import { keyFor } from "../../actions";
import { useKeyBindings } from "../../keymap";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ConcernClaim } from "../SessionThread/ConcernClaim";

import type { Finding } from "./review-state";

export type { Finding } from "./review-state";

/** A Judge's notes on a plan waiting on its card. The host draws the press (`press`, a Judge act), and `onRun` is the key that opens it. */
export type JudgedPlanProps = {
  findings: readonly Finding[];
  onRun: () => void;
  onSend: (id: string) => void;
  onDismiss: (id: string) => void;
  press: ReactNode;
};

function Mark({ said, children }: { said: string; children: ReactNode }) {
  return (
    <Tooltip label={said}>
      <span className="armada-adversarial__mark" role="img" aria-label={said}>
        {children}
      </span>
    </Tooltip>
  );
}

/** One finding: the claim, then its two acts in the card's footer. Focusable; its keys are the registry's (`useFindingKeys`). */
export function FindingRow({
  finding,
  onSend,
  onDismiss,
}: {
  finding: Finding;
  onSend: () => void;
  onDismiss: () => void;
}) {
  useKeyBindings();
  return (
    <li
      onClick={(event) => event.stopPropagation()}
      className="armada-adversarial__finding"
      role="group"
      aria-label={`Finding: ${finding.consequence}`}
      tabIndex={0}
      data-finding={finding.id}
    >
      <div className="armada-adversarial__claim">
        <h4 className="armada-adversarial__heading">You should know</h4>
        <ConcernClaim
          concern={{
            text: finding.consequence,
            consequence: finding.consequence,
            ...(finding.expected === undefined
              ? {}
              : {
                  expected: finding.expected,
                }),
            ...(finding.found === undefined
              ? {}
              : {
                  produced: finding.found,
                }),
            ...(finding.impact === undefined
              ? {}
              : {
                  impact: finding.impact,
                }),
            ...(finding.cites === undefined
              ? {}
              : {
                  cites: finding.cites,
                  citesLabel: "Where",
                }),
          }}
        />
      </div>
      <div className="armada-adversarial__acts" role="group" aria-label="Answers">
        <Tooltip label="Goes to the Job as your message" shortcut={keyFor("judge_note_send")}>
          <Button size="sm" variant="secondary" onClick={onSend}>
            Send to Job
          </Button>
        </Tooltip>
        <Tooltip label="Leaves it unsent" shortcut={keyFor("judge_note_dismiss")}>
          <Button size="sm" variant="ghost" onClick={onDismiss}>
            Dismiss
          </Button>
        </Tooltip>
      </div>
    </li>
  );
}

/** A finding acted on, as one muted line: a mark with its tooltip, then the consequence. */
export function SettledRow({ finding }: { finding: Finding }) {
  const sent = finding.settled === "sent";
  const said = sent ? "Sent" : "Dismissed";
  return (
    <li
      onClick={(event) => event.stopPropagation()}
      className="armada-adversarial__settled"
      role="group"
      aria-label={`Finding ${said.toLowerCase()}: ${finding.consequence}`}
    >
      <Mark said={said}>
        {sent ? (
          <ArrowRight size={12} strokeWidth={2} aria-hidden />
        ) : (
          <Ban size={12} strokeWidth={2} aria-hidden />
        )}
      </Mark>
      <span className="armada-adversarial__settled-text">{finding.consequence}</span>
    </li>
  );
}
