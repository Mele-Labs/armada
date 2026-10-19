import { LessonCard, type LessonAnswers, type LessonEvidence, type LessonSettled, type RetroCite } from "../LessonCard/LessonCard";
import type { Lands } from "../LandsMark/LandsMark";
import type { Who } from "../WhoMark/WhoMark";

/** One item of one Session's retro, as the Lessons page lists it. */
export type LessonRow = {
  /** Unique within the list. */
  id: string;
  sessionId: string;
  who: Who;
  /** Where its fix lands. Absent on an item stored before that was written. */
  landsIn?: Lands;
  statement: string;
  title?: string;
  what?: string;
  fix?: string;
  /** The Session, as a person reads it. Its id rides the label's tooltip. */
  session: string;
  sessionExact?: string;
  /** When its retro was written, an RFC 3339 instant. */
  at: string;
  /** The answers this row offers, absent where it is already answered. */
  answers?: LessonAnswers;
  /** What an answered row reads as while it stays on screen. */
  settled?: LessonSettled;
  cites?: readonly RetroCite[];
  /** Set while the rows it cites are not read yet: the control that reads them. */
  evidence?: LessonEvidence;
};

export type LessonListProps = {
  /** Newest retro first, as Fleet serves them. */
  rows: readonly LessonRow[];
  /** A press on the Session a row came from opens its retro. The row's id says which, where a Session has several. */
  onOpen: (sessionId: string, rowId: string) => void;
};

/**
 * What got in the way across Sessions, newest first — the Retros page's one list,
 * each item a `LessonCard` the owner can agree or disagree with.
 *
 * **No heading and no count.** A count beside the rows it counts is
 * design-system hard rule 7, and an empty list draws nothing.
 *
 * **Not virtualized, and bounded.** `list_lessons` answers 200 at most where
 * `most` is not sent, and Bridge sends none.
 */
export function LessonList({ rows, onOpen }: LessonListProps) {
  if (rows.length === 0) return null;
  return (
    <ul className="armada-lessons" aria-label="Retros">
      {rows.map((row) => (
        <LessonCard
          key={row.id}
          item={{
            who: row.who,
            ...(row.landsIn === undefined ? {} : { landsIn: row.landsIn }),
            statement: row.statement,
            ...(row.title === undefined ? {} : { title: row.title }),
            ...(row.what === undefined ? {} : { what: row.what }),
            ...(row.fix === undefined ? {} : { fix: row.fix }),
            ...(row.cites === undefined ? {} : { cites: row.cites }),
          }}
          {...(row.evidence === undefined ? {} : { evidence: row.evidence })}
          {...(row.answers === undefined ? {} : { answers: row.answers })}
          {...(row.settled === undefined ? {} : { settled: row.settled })}
          from={{
            label: row.session,
            ...(row.sessionExact === undefined ? {} : { exact: row.sessionExact }),
            onOpen: () => onOpen(row.sessionId, row.id),
          }}
        />
      ))}
    </ul>
  );
}
