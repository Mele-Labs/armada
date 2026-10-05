import { LessonCard, type LessonAnswers, type LessonSettled, type RetroCite } from "../LessonCard/LessonCard";
import type { Lands } from "../LandsMark/LandsMark";
import type { Who } from "../WhoMark/WhoMark";

/** One item of one Job's retro, as the Lessons page lists it. */
export type LessonRow = {
  /** Unique within the list. */
  id: string;
  jobId: string;
  who: Who;
  /** Where its fix lands. Absent on an item stored before that was written. */
  landsIn?: Lands;
  statement: string;
  title?: string;
  what?: string;
  fix?: string;
  /** The Job, as a person reads it — `Job 3`. Its handle rides the label's tooltip. */
  job: string;
  jobExact?: string;
  /** When its retro was written, as a date. The exact instant rides the label's tooltip. */
  when: string;
  whenExact?: string;
  /** The answers this row offers, absent where it is already answered. */
  answers?: LessonAnswers;
  /** What an answered row reads as while it stays on screen. */
  settled?: LessonSettled;
  cites?: readonly RetroCite[];
};

export type LessonListProps = {
  /** Newest retro first, as Fleet serves them. */
  rows: readonly LessonRow[];
  /** A press on the Job a row came from opens its retro. */
  onOpen: (jobId: string) => void;
};

/**
 * What got in the way across Jobs, newest first — the Retros page's one list,
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
          {...(row.answers === undefined ? {} : { answers: row.answers })}
          {...(row.settled === undefined ? {} : { settled: row.settled })}
          from={{
            label: row.job,
            ...(row.jobExact === undefined ? {} : { exact: row.jobExact }),
            onOpen: () => onOpen(row.jobId),
          }}
        />
      ))}
    </ul>
  );
}
