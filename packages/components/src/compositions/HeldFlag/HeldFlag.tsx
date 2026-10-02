import type { ReactNode } from "react";
import { useState } from "react";

import { Button, STILL_WAITING, useStillWaiting } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Textarea } from "../../primitives/Textarea/Textarea";
import type { GamingFlagAt } from "../GamingFlags/GamingFlags";
import { UnifiedDiff, type DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * One flag holding a step, as a person has to read it to decide. #1079.
 *
 * **The headline is the registry's**, handed down by the caller. It is copy
 * about an enum, and written here it would be a second vocabulary that says
 * one pattern two ways.
 */
export type HeldFinding = {
  /** The wire spelling. Drawn in mono only where no headline arrived. */
  pattern: string;
  /** `An assertion now asserts less` — the pattern's registry verb. */
  headline?: string;
  /**
   * The flagged lines, as the hunk of the patch that holds them. **Located by
   * the caller from the patch, never from an invented line** — a flag on a
   * removed line has no post-image number, and is found by what it quotes.
   * Absent draws the citation and the location instead.
   */
  hunk?: DiffFile;
  /** What the check quoted, drawn where no hunk was located. */
  cited?: string;
  /** Where it is, drawn where no hunk was located. */
  at?: GamingFlagAt;
  /** The question the check answered. Absent on the three the diff decides. */
  asked?: string;
  /** The brief the question was asked in, by its path. */
  brief?: string;
};

/** What one answer does, and why it is off where it is. */
export type HeldAnswer = {
  /** What pressing it does, in a sentence, under the answer. */
  consequence: ReactNode;
  /** Why this answer cannot be taken here. Present turns its control off. */
  withheld?: ReactNode;
};

export type HeldFlagProps = {
  /** Every flag holding the step, in the order the check answered. Empty draws nothing. */
  findings: HeldFinding[];
  onOpenBrief?: (brief: string) => void;
  /** *No, the work is fine*. The reason is optional: a note, or nothing. */
  carryOn: HeldAnswer & { onCarryOn: (reason: string) => void };
  /**
   * *Yes, the flag is right*. The note is optional and goes to the Drone.
   * Whether that is a redirect or a restart is the caller's, and `consequence`
   * says which.
   */
  sendBack: HeldAnswer & { onSendBack: (note?: string) => void };
  /** An act on this Job is already out, or what is shown is not live. */
  disabled?: boolean;
  /** Why both answers are off, where they are. */
  disabledNote?: ReactNode;
  /**
   * The answer that was pressed and Fleet has not answered. That control
   * waits and the other is off; absent is nothing out. #1117.
   */
  pending?: HeldFlagAnswer;
};

/** Which of the two answers `HeldFlag` draws. */
export type HeldFlagAnswer = "carryOn" | "sendBack";

const IT_ASKED = "It asked";
const OPEN_THE_BRIEF = "Open the brief";
const IS_IT_RIGHT = "Is the flag right?";
const NO_IT_IS_FINE = "No, the work is fine";
const CARRY_ON = "Carry on";
const YES_IT_WAS = "Yes, the flag is right";
const SEND_IT_BACK = "Send it back";
const NOTE = "Note (optional)";
const NOTE_FOR_THE_DRONE = "Note for the drone (optional)";

/**
 * The card a gaming flag holds a step with: what it means, the lines it is
 * about, what it asked, and the two answers a person can give.
 *
 * **Both answers, with what each does, and neither needs typing.** A reason
 * is asked for and never required, so *Carry on* and *Send it back* are each
 * one press — the definition of done in #1079.
 *
 * **Secondary, both.** Neither is an approval: carrying on says the check was
 * wrong, and sending it back says the Drone was. A fill on either would say
 * which the screen expects.
 */
export function HeldFlag({
  findings,
  onOpenBrief,
  carryOn,
  sendBack,
  disabled = false,
  disabledNote,
  pending,
}: HeldFlagProps) {
  const [reasonNote, setReasonNote] = useState("");
  const [droneNote, setDroneNote] = useState("");
  const stillWaiting = useStillWaiting(pending !== undefined);
  if (findings.length === 0) return null;

  function carry() {
    // The note, or nothing — Fleet takes a blank reason on a gaming flag, and
    // the words it does get are the person's own.
    carryOn.onCarryOn(reasonNote.trim());
  }

  function sendIt() {
    const said = droneNote.trim();
    sendBack.onSendBack(said === "" ? undefined : said);
  }

  return (
    <div className="armada-held-flag">
      {findings.map((finding, at) => (
        <article
          className="armada-held-flag__finding"
          key={`finding-${at}`}
          aria-label={finding.headline ?? finding.pattern}
        >
          {finding.headline === undefined ? (
            <span className="armada-held-flag__pattern">{finding.pattern}</span>
          ) : (
            <p className="armada-held-flag__headline">{finding.headline}</p>
          )}
          {finding.hunk !== undefined ? (
            <UnifiedDiff files={[finding.hunk]} emptyNote="" />
          ) : (
            <>
              {finding.cited === undefined || finding.cited === "" ? null : (
                <div className="armada-held-flag__cited">
                  <Prose text={finding.cited} />
                </div>
              )}
              {finding.at === undefined ? null : (
                <span className="armada-held-flag__at">
                  {finding.at.line === undefined ? finding.at.file : `${finding.at.file}:${finding.at.line}`}
                </span>
              )}
            </>
          )}
          {finding.asked === undefined ? null : (
            <div className="armada-held-flag__asked">
              <span className="armada-held-flag__label">{IT_ASKED}</span>
              <p className="armada-held-flag__question">{finding.asked}</p>
              {finding.brief === undefined || onOpenBrief === undefined ? null : (
                <Button variant="ghost" size="sm" onClick={() => onOpenBrief(finding.brief as string)}>
                  {OPEN_THE_BRIEF}
                </Button>
              )}
            </div>
          )}
        </article>
      ))}

      <div className="armada-held-flag__decide" role="group" aria-label={IS_IT_RIGHT}>
        <p className="armada-held-flag__headline">{IS_IT_RIGHT}</p>

        <div className="armada-held-flag__answer">
          <span className="armada-held-flag__answer-label">{NO_IT_IS_FINE}</span>
          <p className="armada-held-flag__means">{carryOn.consequence}</p>
          <Textarea
            label={NOTE}
            rows={2}
            value={reasonNote}
            disabled={disabled || carryOn.withheld !== undefined}
            onChange={(event) => setReasonNote(event.target.value)}
          />
          <div className="armada-held-flag__press">
            <Button
              variant="secondary"
              pending={pending === "carryOn"}
              disabled={disabled || carryOn.withheld !== undefined}
              onClick={carry}
            >
              {pending === "carryOn" ? "Carrying on…" : CARRY_ON}
            </Button>
          </div>
          {carryOn.withheld === undefined ? null : (
            <p className="armada-held-flag__withheld" role="note">
              {carryOn.withheld}
            </p>
          )}
        </div>

        <div className="armada-held-flag__answer">
          <span className="armada-held-flag__answer-label">{YES_IT_WAS}</span>
          <p className="armada-held-flag__means">{sendBack.consequence}</p>
          <Textarea
            label={NOTE_FOR_THE_DRONE}
            rows={2}
            value={droneNote}
            disabled={disabled || sendBack.withheld !== undefined}
            onChange={(event) => setDroneNote(event.target.value)}
          />
          <div className="armada-held-flag__press">
            <Button
              variant="secondary"
              pending={pending === "sendBack"}
              disabled={disabled || sendBack.withheld !== undefined}
              onClick={sendIt}
            >
              {pending === "sendBack" ? "Sending it back…" : SEND_IT_BACK}
            </Button>
          </div>
          {sendBack.withheld === undefined ? null : (
            <p className="armada-held-flag__withheld" role="note">
              {sendBack.withheld}
            </p>
          )}
        </div>

        {stillWaiting ? (
          <p className="armada-held-flag__withheld" role="status">
            {STILL_WAITING}
          </p>
        ) : disabled && pending === undefined && disabledNote !== undefined ? (
          <p className="armada-held-flag__withheld" role="note">
            {disabledNote}
          </p>
        ) : null}
      </div>
    </div>
  );
}
