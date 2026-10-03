import type { ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { WhoMark, type Who } from "../WhoMark/WhoMark";

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

/** One thing that got in the way. */
export type RetroSheetItem = {
  who: Who;
  statement: string;
  /** The rows the record holds for it. A cite the record does not hold is left out. */
  cites: readonly RetroCite[];
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
 * **Nothing here acts.** No control files, proposes or writes; the owner
 * reads and decides. The close is the sheet's own.
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
            <li key={at} className="armada-retro__item">
              <WhoMark who={item.who} />
              <div className="armada-retro__said">
                <p className="armada-retro__statement">{item.statement}</p>
                {item.cites.length === 0 ? null : (
                  <ul className="armada-retro__cites">
                    {item.cites.map((cite) => (
                      <li key={cite.id} className="armada-retro__cite">
                        <span className="armada-retro__name" data-mono={cite.mono ? "true" : undefined}>
                          {cite.name}
                        </span>
                        {cite.detail === undefined ? null : (
                          <span className="armada-retro__detail">{cite.detail}</span>
                        )}
                        {cite.when === undefined ? null : (
                          <span className="armada-retro__when">{cite.when}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </li>
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
