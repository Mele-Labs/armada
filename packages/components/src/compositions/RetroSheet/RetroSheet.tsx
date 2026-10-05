import type { ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { LessonCard, type LessonAnswers, type LessonCardItem, type LessonSettled } from "../LessonCard/LessonCard";

/**
 * One thing that got in the way. `cites` are the rows the record holds for it:
 * a cite the record does not hold is left out.
 */
export type RetroSheetItem = LessonCardItem & {
  /** The item's own id, which an answer names. */
  id?: string;
  answers?: LessonAnswers;
  settled?: LessonSettled;
};

/** A note the owner left with this Job's detail open. */
export type RetroNote = { id: string; text: string; when?: string };

export type RetroSheetProps = {
  open: boolean;
  /** The Job, as a person reads it. */
  job?: ReactNode;
  /** The read has not answered yet. */
  reading?: boolean;
  /** Why the read failed, in Fleet's words. Draws in place of everything else. */
  failure?: string;
  /** Where the retro stands, where it is not simply written. Absent draws nothing. */
  status?: string;
  items: readonly RetroSheetItem[];
  notes: readonly RetroNote[];
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

/**
 * One Job's retro: what got in the way, whose way, and the record rows that
 * show it — `docs/concepts/retro.md`.
 *
 * **One sheet, opened from two places**: a row on the Lessons page, and the
 * Job's own Record. The same layer in both, so a retro reads the same
 * wherever it is reached.
 *
 * **Each item is a `LessonCard`**, the Lessons list's own, so the owner agrees
 * or disagrees with it here as there.
 *
 * **An empty slot stays empty.** A written retro with no items draws nothing,
 * and a Job with no linked notes draws no notes section.
 */
export function RetroSheet({
  open,
  job,
  reading = false,
  failure,
  status,
  items,
  notes,
  floor = false,
  onClose,
}: RetroSheetProps) {
  return (
    <Sheet
      kind="retro"
      open={open}
      floating
      floor={floor}
      title="Retro"
      subtitle={job === undefined ? undefined : <span className="armada-retro__job">{job}</span>}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <Body reading={reading} failure={failure} status={status} items={items} notes={notes} />
    </Sheet>
  );
}

function Body({
  reading,
  failure,
  status,
  items,
  notes,
}: Pick<RetroSheetProps, "reading" | "failure" | "status" | "items" | "notes">) {
  if (failure !== undefined) {
    return (
      <Alert tone="escalated" title="The retro could not be read">
        {failure}
      </Alert>
    );
  }
  if (reading === true) return <SkeletonText />;
  return (
    <div className="armada-retro">
      {status === undefined ? null : <p className="armada-retro__status">{status}</p>}
      {items.length === 0 ? null : (
        <ol className="armada-retro__items">
          {items.map((item, at) => (
            <LessonCard
              key={item.id ?? at}
              item={item}
              {...(item.answers === undefined ? {} : { answers: item.answers })}
              {...(item.settled === undefined ? {} : { settled: item.settled })}
            />
          ))}
        </ol>
      )}
      {notes.length === 0 ? null : (
        <section className="armada-retro__notes" aria-label="Notes">
          <h3 className="armada-retro__heading">Notes</h3>
          <ul className="armada-retro__cites">
            {notes.map((note) => (
              <li key={note.id} className="armada-retro__note">
                <span className="armada-retro__statement">{note.text}</span>
                {note.when === undefined ? null : <span className="armada-retro__when">{note.when}</span>}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
