import type { ReactNode } from "react";
import { useState } from "react";
import { ThumbsDown, ThumbsUp } from "lucide-react";

import { Button, STILL_WAITING, useStillWaiting } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { GamingFlagAt } from "../GamingFlags/GamingFlags";
import { UnifiedDiff, type DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * One flag holding a step, as a person has to read it to decide. #1079.
 *
 * **The words are the registry's**, handed down by the caller. They are copy
 * about an enum, and written here they would be a second vocabulary that says
 * one pattern two ways.
 */
export type HeldFinding = {
  /**
   * What happened, as the registry's headline says it. Names the finding to a
   * reader who cannot see it, always.
   */
  label: string;
  /**
   * Drawn over the finding where nothing above the card already says it. Under
   * Overview's lead, which says it, the card leaves it out rather than say one
   * sentence twice on one screen.
   */
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
  /** The question the gaming check answered. Absent on the three the diff decides. */
  asked?: string;
  /** The brief the question was asked in, by its path. */
  brief?: string;
};

/** What one answer does, and why it is off where it is. */
export type HeldAnswer = {
  /** What pressing it does, in a sentence, on its tooltip. */
  consequence: string;
  /** Why this answer cannot be taken here. Present turns its control off. */
  withheld?: ReactNode;
};

export type HeldFlagProps = {
  /** Every flag holding the step, in the order the check answered. Empty draws nothing. */
  findings: HeldFinding[];
  onOpenBrief?: (brief: string) => void;
  /** *The work is fine*: the override, with the note as its reason, or none. */
  carryOn: HeldAnswer & { onCarryOn: (reason: string) => void };
  /**
   * *The flag is right*: the note goes to the Drone, or none. Whether that is a
   * redirect or a restart is the caller's, and `consequence` says which.
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

/** Who asked. The registry's name for the machine, never a pronoun (owner, 2 Oct 2026). */
const THE_CHECK_ASKED = "The gaming check asked:";
const OPEN_THE_BRIEF = "Open the brief";
const ANSWER = "Answer the flag";
const CARRY_ON = "Carry on";
const SEND_IT_BACK = "Send it back";
const NOTE = "Note (optional)";

/**
 * The card a gaming flag holds a step with: what it is about, the lines, what
 * the gaming check asked, and the two answers.
 *
 * **Two buttons and one note, not two forms** (owner, 2 Oct 2026). The note
 * goes with whichever is pressed — as the override's reason on Carry on, to
 * the Drone on Send it back — and is never required, so each is one press.
 * What each does is on its tooltip, and a note sent is spent: the field clears
 * on the press.
 *
 * **Carry on is the primary and Send it back the destructive outline**, each
 * with its thumb: one lets the Job go forward and the other sends the work
 * back, read so before their words are. Both exceptions (Kill-only red,
 * label-only buttons) are in `design-system.md` and `iconography.md`.
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
  const [note, setNote] = useState("");
  const stillWaiting = useStillWaiting(pending !== undefined);
  if (findings.length === 0) return null;
  const said = note.trim();

  return (
    <div className="armada-held-flag">
      {findings.map((finding, at) => (
        <article className="armada-held-flag__finding" key={`finding-${at}`} aria-label={finding.label}>
          {finding.headline === undefined ? null : (
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
              <p className="armada-held-flag__question">
                <span className="armada-held-flag__asker">{THE_CHECK_ASKED}</span> {finding.asked}
              </p>
              {finding.brief === undefined || onOpenBrief === undefined ? null : (
                <Button variant="secondary" size="sm" onClick={() => onOpenBrief(finding.brief as string)}>
                  {OPEN_THE_BRIEF}
                </Button>
              )}
            </div>
          )}
        </article>
      ))}

      <div className="armada-held-flag__decide" role="group" aria-label={ANSWER}>
        <Textarea
          label={NOTE}
          rows={2}
          value={note}
          disabled={disabled}
          onChange={(event) => setNote(event.target.value)}
        />
        <div className="armada-held-flag__press">
          <Tooltip label={carryOn.consequence}>
            <Button
              variant="primary"
              pending={pending === "carryOn"}
              disabled={disabled || carryOn.withheld !== undefined}
              // The note, or nothing — Fleet takes a blank reason on a gaming
              // flag, and the words it does get are the person's own.
              onClick={() => {
                setNote("");
                carryOn.onCarryOn(said);
              }}
            >
              <ThumbsUp aria-hidden="true" size={16} />
              {pending === "carryOn" ? "Carrying on…" : CARRY_ON}
            </Button>
          </Tooltip>
          <Tooltip label={sendBack.consequence}>
            <Button
              variant="destructive"
              pending={pending === "sendBack"}
              disabled={disabled || sendBack.withheld !== undefined}
              onClick={() => {
                setNote("");
                sendBack.onSendBack(said === "" ? undefined : said);
              }}
            >
              <ThumbsDown aria-hidden="true" size={16} />
              {pending === "sendBack" ? "Sending it back…" : SEND_IT_BACK}
            </Button>
          </Tooltip>
        </div>
        {[carryOn.withheld, sendBack.withheld].map((why, at) =>
          why === undefined ? null : (
            <p className="armada-held-flag__withheld" role="note" key={`withheld-${at}`}>
              {why}
            </p>
          ),
        )}
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
