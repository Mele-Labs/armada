import type { KeyboardEvent } from "react";

import { Table, TableBody, TableCell, TableRow } from "../../primitives/Table/Table";
import { LandsMark, type Lands } from "../LandsMark/LandsMark";
import { WhoMark, type Who } from "../WhoMark/WhoMark";

/** One item of one Job's retro, as the Lessons page lists it. */
export type LessonRow = {
  /** Unique within the list. */
  id: string;
  jobId: string;
  who: Who;
  /** Where its fix lands. Absent on an item stored before that was written. */
  landsIn?: Lands;
  statement: string;
  /** The Job, as a person reads it — `Job 3`. Its handle rides the cell's title. */
  job: string;
  jobExact?: string;
  /** When its retro was written, as a date. The exact instant rides the cell's title. */
  when: string;
  whenExact?: string;
};

export type LessonListProps = {
  /** Newest retro first, as Fleet serves them. */
  rows: readonly LessonRow[];
  /** The Job whose retro is open, so its rows read as selected. */
  openJob?: string | null;
  onOpen: (jobId: string) => void;
};

/**
 * What got in the way across Jobs, newest first — the Lessons page's one list.
 *
 * **Nothing here acts.** A row opens its Job's retro, and that is all a press
 * does: the owner reads and decides, and nothing is filed or proposed
 * (`docs/concepts/retro.md`).
 *
 * **No heading row and no count.** Four columns a person reads at a glance —
 * whose way, what, which Job, when — need no labels over them, and a count
 * beside the rows it counts is design-system hard rule 7.
 *
 * **Not virtualized, and bounded.** `list_lessons` answers 200 at most where
 * `most` is not sent, and Bridge sends none.
 */
export function LessonList({ rows, openJob = null, onOpen }: LessonListProps) {
  if (rows.length === 0) return null;
  return (
    <section className="armada-lessons armada-glass" aria-label="Lessons">
      <Table>
        <TableBody>
          {rows.map((row) => (
            <TableRow
              key={row.id}
              className="armada-lessons__row"
              selected={row.jobId === openJob}
              tabIndex={0}
              onClick={() => onOpen(row.jobId)}
              onKeyDown={(event: KeyboardEvent) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                onOpen(row.jobId);
              }}
            >
              <TableCell className="armada-lessons__who">
                <span className="armada-lessons__marks">
                  <WhoMark who={row.who} />
                  {row.landsIn === undefined ? null : <LandsMark lands={row.landsIn} />}
                </span>
              </TableCell>
              <TableCell variant="primary" className="armada-lessons__statement">
                {row.statement}
              </TableCell>
              <TableCell variant="mono" className="armada-lessons__job" title={row.jobExact}>
                {row.job}
              </TableCell>
              <TableCell variant="metadata" className="armada-lessons__when" title={row.whenExact}>
                {row.when}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </section>
  );
}
