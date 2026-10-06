import { ChevronDown, ChevronRight, MoveRight } from "lucide-react";
import { useState } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { LandsMark, landsWord, type Lands } from "../LandsMark/LandsMark";
import { WhoMark, whoWord, type Who } from "../WhoMark/WhoMark";

/** One row of the Job's record an item cites, resolved from its `cite`. */
export type RetroCite = {
  /** The record's own `cite`, unique within the record. */
  id: string;
  /** What the row is about: a tool, a Check, a criterion, a move. */
  name: string;
  /** A declared name — a tool, a Check, a criterion — is drawn in the mono face. */
  mono?: boolean;
  /** What it came to, in the record's words. */
  detail?: string;
  /** When, where the row says. */
  when?: string;
};

/** What one retro item says, whichever surface draws it. */
export type LessonCardItem = {
  who: Who;
  /** Where its fix lands. Absent on an item stored before that was written. */
  landsIn?: Lands;
  /** The one sentence an item written before the headline had. Drawn as the body where `title` is absent. */
  statement: string;
  title?: string;
  what?: string;
  fix?: string;
  /** The rows the record holds for it. Absent where the surface has not read the record. */
  cites?: readonly RetroCite[];
};

/** The two answers, and what each does for this item's place. */
export type LessonAnswers = {
  agreeLabel: string;
  agreeTip: string;
  disagreeTip: string;
  onAgree: () => void;
  onDisagree: () => void;
  /** The answer sent and not yet answered: its button sweeps and the other is held. */
  pressing?: "agree" | "disagree";
  /** Fleet's words for an answer it refused. */
  refusal?: string;
};

/** What an answered item reads as, while it is still on screen. */
export type LessonSettled = {
  /** `Agreed`, or `Updated Kit`. */
  said: string;
  /** The command Kit was updated with, drawn in monospace beside `said`. */
  command?: string;
  /** The Job the item proposed, where the surface can open one. */
  job?: { label: string; onOpen: () => void };
};

/**
 * An item whose rows are not read yet: the list holds only the ids they cite,
 * so the Evidence control asks for them. Absent once they are read, or where
 * the surface read them with the item.
 */
export type LessonEvidence = {
  /** Ask for the rows. Pressed again after a failed read, it asks again. */
  onAsk: () => void;
  /** The read is out. */
  pending?: boolean;
  /** Fleet's words for a read that failed. */
  failure?: string;
};

export type LessonCardProps = {
  item: LessonCardItem;
  evidence?: LessonEvidence;
  answers?: LessonAnswers;
  settled?: LessonSettled;
  /** The Job the item came from, on the Lessons list, where a press opens its retro. */
  from?: { label: string; exact?: string; onOpen: () => void };
};

/**
 * One retro item, top to bottom: whose way it got in and where its fix lands as
 * words, the headline, what happened, the fix, and the two answers. The record
 * rows it cites are behind a small control.
 *
 * **The Lessons list and a Job's retro sheet draw this one component**, so an
 * item reads the same wherever it is reached. An item written before the
 * headline carries its `statement` alone, drawn as the body with the same
 * answers under it.
 *
 * **The words come first and the marks beside them stay**: a mark carries the
 * tooltip it always did, and never stands in for the word.
 */
export function LessonCard({ item, evidence, answers, settled, from }: LessonCardProps) {
  const [showing, setShowing] = useState(false);
  const cites = item.cites ?? [];
  const asking = evidence !== undefined;
  const headed = item.title !== undefined && item.title !== "";
  const body = headed ? item.what : item.statement;
  return (
    // Named by its headline, so one item is found among many, by a person using a
    // reader and by a walk alike.
    <li className="armada-lesson" aria-label={headed ? item.title : item.statement}>
      <div className="armada-lesson__labels">
        {/* One arrow, from whose way it got in to where the fix lands. Each end is its own hue. */}
        <span className="armada-lesson__route">
          <span className="armada-lesson__label" data-hue={item.who === "owner" ? "you" : item.who}>
            <WhoMark who={item.who} />
            <span className="armada-lesson__word">{whoWord(item.who)}</span>
          </span>
          {item.landsIn === undefined ? null : (
            <>
              <MoveRight className="armada-lesson__arrow" size={14} strokeWidth={2} aria-hidden />
              <span className="armada-lesson__label" data-hue={item.landsIn}>
                <LandsMark lands={item.landsIn} />
                <span className="armada-lesson__word">{landsWord(item.landsIn)}</span>
              </span>
            </>
          )}
        </span>
        {from === undefined ? null : (
          <Tooltip label={from.exact ?? from.label}>
            <button type="button" className="armada-lesson__from" onClick={from.onOpen}>
              {from.label}
            </button>
          </Tooltip>
        )}
      </div>
      {headed ? <h3 className="armada-lesson__headline">{item.title}</h3> : null}
      {body === undefined || body === "" ? null : (
        <p className="armada-lesson__what" data-headed={headed || undefined}>
          {body}
        </p>
      )}
      {item.fix === undefined || item.fix === "" ? null : (
        <div className="armada-lesson__fix">
          <span className="armada-lesson__fix-label">What would change</span>
          <p className="armada-lesson__fix-said">{item.fix}</p>
        </div>
      )}
      {answers?.refusal === undefined ? null : (
        <Alert tone="escalated" title="The answer was not taken">
          {answers.refusal}
        </Alert>
      )}
      {evidence?.failure === undefined ? null : (
        <Alert
          tone="escalated"
          title="Retros could not be read"
          action={
            <Button size="sm" onClick={evidence.onAsk}>
              Retry
            </Button>
          }
        >
          {evidence.failure}
        </Alert>
      )}
      {answers === undefined && settled === undefined && cites.length === 0 && !asking ? null : (
        <div className="armada-lesson__acts">
          {settled !== undefined ? (
            <span className="armada-lesson__settled">
              <span className="armada-lesson__said">{settled.said}</span>
              {settled.command === undefined ? null : (
                <span className="armada-lesson__command mono">{settled.command}</span>
              )}
              {settled.job === undefined ? null : (
                <button type="button" className="armada-lesson__job" onClick={settled.job.onOpen}>
                  {settled.job.label}
                </button>
              )}
            </span>
          ) : answers === undefined ? null : (
            <>
              <Tooltip label={answers.agreeTip}>
                <Button
                  size="sm"
                  pending={answers.pressing === "agree"}
                  disabled={answers.pressing === "disagree"}
                  onClick={answers.onAgree}
                >
                  {answers.agreeLabel}
                </Button>
              </Tooltip>
              <Tooltip label={answers.disagreeTip}>
                <Button
                  size="sm"
                  pending={answers.pressing === "disagree"}
                  disabled={answers.pressing === "agree"}
                  onClick={answers.onDisagree}
                >
                  Reject change
                </Button>
              </Tooltip>
            </>
          )}
          {cites.length === 0 && !asking ? null : (
            <span className="armada-lesson__evidence-toggle">
              <Button
                variant="ghost"
                size="sm"
                aria-expanded={showing && !asking}
                pending={evidence?.pending === true}
                onClick={() => {
                  if (evidence !== undefined) {
                    setShowing(true);
                    evidence.onAsk();
                  } else setShowing((was) => !was);
                }}
              >
                {showing && !asking ? (
                  <ChevronDown size={14} strokeWidth={2} aria-hidden />
                ) : (
                  <ChevronRight size={14} strokeWidth={2} aria-hidden />
                )}
                Evidence
              </Button>
            </span>
          )}
        </div>
      )}
      {!showing || cites.length === 0 ? null : (
        <ul className="armada-retro__cites">
          {cites.map((cite) => (
            <li key={cite.id} className="armada-retro__cite">
              <span className="armada-retro__name" data-mono={cite.mono ? "true" : undefined}>
                {cite.name}
              </span>
              {cite.detail === undefined ? null : <span className="armada-retro__detail">{cite.detail}</span>}
              {cite.when === undefined ? null : <span className="armada-retro__when">{cite.when}</span>}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}
