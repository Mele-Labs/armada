import type { ReactNode } from "react";

import type { JudgeAnswer } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { JudgeQuestion } from "../JudgeQuestion/JudgeQuestion";

export type MemberDecisionProps = {
  /** Which member is asking — `Member 2`, the same words its row carries. */
  who: string;
  /** Which criterion refused, in the wire's own spelling. */
  criterion: string;
  question: string;
  expected: string;
  produced: string;
  consequence: string;
  /** How long it has been asking, already written — `4 minutes`. */
  asked?: string;
  /** What this Job is at while it asks, in the registry's own verb. */
  state?: string;
  /**
   * What the answer moves besides this member — the pull requests behind it,
   * named. **Spelled out rather than implied**: the same verdict is written
   * either way, and what is only true here is that others wait on the press.
   */
  moves: ReactNode;
  onAnswer: (answer: JudgeAnswer, note?: string) => void;
  /** An answer already in flight, or nothing live to send it over. */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: string;
  /** The answer that was pressed and Fleet has not answered. */
  pending?: boolean;
  /** Open the member's own Job, where the same verdict can be written. */
  onOpen?: () => void;
};

/**
 * One member's refusal, in a column of its own beside the order it belongs to.
 *
 * **Review and reply are one loop** — `docs/practices/bridge.md`. The member's
 * row says a question is open; this is where it is read and answered, without
 * leaving the screen the order is on.
 */
export function MemberDecision({
  who,
  criterion,
  question,
  expected,
  produced,
  consequence,
  asked,
  state,
  moves,
  onAnswer,
  disabled,
  disabledNote,
  pending,
  onOpen,
}: MemberDecisionProps) {
  return (
    <section className="armada-decision" aria-label={`${who} · asked of you`}>
      <header className="armada-decision__head">
        <p className="armada-decision__who">{`${who} · asked of you`}</p>
        <h3 className="armada-decision__asked">{question}</h3>
        <p className="armada-decision__meta">
          {state === undefined ? null : <span className="armada-decision__state">{state}</span>}
          {asked === undefined ? null : <span className="armada-decision__age">{asked}</span>}
        </p>
      </header>

      <p className="armada-decision__criterion">
        Refused
        <span className="armada-decision__criterion-name">{criterion}</span>
      </p>

      <JudgeQuestion
        stacked
        question={question}
        expected={expected}
        produced={produced}
        consequence={consequence}
        onAnswer={onAnswer}
        {...(disabled === undefined ? {} : { disabled })}
        {...(disabledNote === undefined ? {} : { disabledNote })}
        {...(pending === undefined ? {} : { pending })}
      />

      <div className="armada-decision__moves">
        <p className="armada-decision__moves-label">What your answer moves</p>
        <p className="armada-decision__moves-said">{moves}</p>
      </div>

      <span className="armada-decision__spacer" />

      {onOpen === undefined ? null : (
        <div className="armada-decision__act">
          <Button variant="secondary" ground="sunken" onClick={onOpen}>
            {`Open ${who.toLowerCase()}`}
          </Button>
          <p className="armada-decision__act-said">
            Answering here writes the same verdict as answering on the member itself.
          </p>
        </div>
      )}
    </section>
  );
}
