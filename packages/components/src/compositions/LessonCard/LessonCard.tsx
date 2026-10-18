import { ChevronDown, ChevronRight, MoveRight } from "lucide-react";
import { useState } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { LandsMark, landsWord, type Lands } from "../LandsMark/LandsMark";
import { WhoMark, whoWord, type Who } from "../WhoMark/WhoMark";

/** One row of the Session's record an item cites, resolved from its `cite`. */
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
  /** Accept, or Update Kit: only a Kit item has one. Any other item is agreed by starting a Session. */
  agree?: { label: string; tip: string; onAgree: () => void };
  disagreeTip: string;
  onDisagree: () => void;
  /**
   * `Start Session`, the way to agree with an Armada or Manifest item. Absent on a Kit item, which
   * has no repository to start one on. **A press begins work at once**: Fleet sends the request as
   * the Session's first message, and the host opens the Session running.
   */
  startSession?: { tip: string; onStart: () => void };
  /** The answer sent and not yet answered: its button sweeps and the others are held. */
  pressing?: "agree" | "disagree" | "session";
  /** Fleet's words for an answer it refused. */
  refusal?: string;
};

/** What an answered item reads as, while it is still on screen. */
export type LessonSettled = {
  /** `Agreed`, `Session started` or `Updated Kit`. */
  said: string;
  /** The command Kit was updated with, drawn in monospace beside `said`. */
  command?: string;
  /** The Session the item started, where the surface can open one. */
  session?: { label: string; onOpen: () => void };
  /** Fleet's words where the Session started and its first message did not go. */
  note?: string;
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
  /** The Session the item came from, on the Lessons list, where a press opens its retro. */
  from?: { label: string; exact?: string; onOpen: () => void };
};

/**
 * Whose way it got in, an arrow, and where the fix lands: each end a word with its mark and its
 * own hue. **One arrow, one component**, so the Lessons list, a retro sheet and the retro a
 * Session is writing in its thread draw the same pair. An end that is absent is not drawn.
 */
export function LessonRoute({ who, landsIn }: { who?: Who; landsIn?: Lands }) {
  return (
    <span className="armada-lesson__route">
      {who === undefined ? null : (
        <span className="armada-lesson__label" data-hue={who === "owner" ? "you" : who}>
          <WhoMark who={who} />
          <span className="armada-lesson__word">{whoWord(who)}</span>
        </span>
      )}
      {landsIn === undefined ? null : (
        <>
          {who === undefined ? null : <MoveRight className="armada-lesson__arrow" size={14} strokeWidth={2} aria-hidden />}
          <span className="armada-lesson__label" data-hue={landsIn}>
            <LandsMark lands={landsIn} />
            <span className="armada-lesson__word">{landsWord(landsIn)}</span>
          </span>
        </>
      )}
    </span>
  );
}

/**
 * One retro item, top to bottom: whose way it got in and where its fix lands as
 * words, the headline, what happened, the fix, and the two answers. The record
 * rows it cites are behind a small control.
 *
 * **The Lessons list and a retro sheet draw this one component**, so an
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
        <LessonRoute who={item.who} landsIn={item.landsIn} />
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
      {settled?.note === undefined ? null : (
        <Alert tone="escalated" title="The Job started, but the request was not sent">
          {settled.note}
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
              {settled.session === undefined ? null : (
                <button type="button" className="armada-lesson__job" onClick={settled.session.onOpen}>
                  {settled.session.label}
                </button>
              )}
            </span>
          ) : answers === undefined ? null : (
            <>
              {answers.agree === undefined ? null : (
                <Tooltip label={answers.agree.tip}>
                  <Button
                    size="sm"
                    pending={answers.pressing === "agree"}
                    disabled={answers.pressing !== undefined && answers.pressing !== "agree"}
                    onClick={answers.agree.onAgree}
                  >
                    {answers.agree.label}
                  </Button>
                </Tooltip>
              )}
              {answers.startSession === undefined ? null : (
                <Tooltip label={answers.startSession.tip}>
                  <Button
                    size="sm"
                    variant="secondary"
                    pending={answers.pressing === "session"}
                    disabled={answers.pressing !== undefined && answers.pressing !== "session"}
                    onClick={answers.startSession.onStart}
                  >
                    Start Job
                  </Button>
                </Tooltip>
              )}
              <Tooltip label={answers.disagreeTip}>
                <Button
                  size="sm"
                  pending={answers.pressing === "disagree"}
                  disabled={answers.pressing !== undefined && answers.pressing !== "disagree"}
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
