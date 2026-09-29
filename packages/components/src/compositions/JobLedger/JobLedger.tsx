import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";
import { TabsWithCounts } from "../../primitives/TabsWithCounts/TabsWithCounts";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Job ledger — one table of everything that happened to a Job, newest first.
 *
 * **It replaces `JobRecord` rather than nesting inside it.** That one folded
 * five readings behind a strip of its own, so reading why a step was refused
 * meant opening four and lining timestamps up by hand.
 *
 * **No row is counted twice, and the filters need not sum to All** — the board
 * this came from said All 34 while its filters summed to 35. A row that
 * answers to no filter is under All alone, and `note` is where the surface
 * says how many and what they are.
 *
 * **Who is a column**, and telling a Check, a Judge and Fleet apart is most of
 * what this table is for. It read `Who ran it` until the owner cut it to one
 * word, 28 September 2026.
 *
 * **The kind is a mark and not a column** (the owner, same day). A 12px mark
 * leads the row under All alone, named by its tooltip — `kindMarks`. A specific
 * filter has already said the family, and a spelling like `task_files` was
 * costing a sentence's width to say what the row's words say.
 *
 * **The filters sit inside the panel they narrow**, on its head row beside the
 * panel's own controls. Stacked under the destinations as a second strip, the
 * two read as one level (the owner, 28 September 2026).
 *
 * **The open row is a column beside the table, and a sheet below
 * `--layout-breakpoint`.** It was a sheet at every width while Helm's dock took
 * 380px off the content and left the table 250px; the dock is a layer now
 * (#1583), and the board draws the two side by side. The floor is still the
 * width a column cannot pay for.
 */

/** Who ran a row, as the column reads it. */
export type LedgerWho = "you" | "contributor" | "fleet" | "drone" | "judge" | "check";

/**
 * What a row came to, for the hue. **None of these is a Job status** — a failed
 * Check inside a running Job is not a failed Job, and `tokens/status.css` is
 * what they alias.
 */
export type LedgerTone = "passed" | "failed" | "waiting" | "running";

/**
 * The mark leading a row, and the kind it stands for.
 *
 * **`says` is the kind's own name**, which is what the tooltip carries and what
 * the mark alone cannot say. A kind the icon registry has no glyph for carries
 * no mark at all rather than a borrowed one — `packages/icons/icons.toml`,
 * `[conventions.record_kind_mark]`.
 */
export type JobLedgerMark = { glyph: ReactNode; says: string };

export type JobLedgerRow = {
  /** What a selection names. Stable across renders, never the row's position. */
  id: string;
  /** When, already written for a reader: this owns chrome, the surface owns words. */
  when: ReactNode;
  /** The full instant, for the pointer. */
  whenExact?: string;
  /**
   * Where in the Job it happened — the step, its group and its task where it
   * has one. **`Where` and not `Step`**: a fact about the Job's own machine
   * names no step, and a blank cell would read as a gap.
   */
  where: ReactNode;
  who: LedgerWho;
  /** Who, spelled. The caller's, so a Drone's row can name the Drone. */
  whoSays: ReactNode;
  /**
   * The kind's mark. Absent where the registry assigns the kind no glyph, and
   * drawn only while `kindMarks` is on.
   */
  mark?: JobLedgerMark;
  /** What happened. The thing the row is about. */
  what: ReactNode;
  /** What it came to. Absent where the row states no outcome, never a placeholder. */
  outcome?: ReactNode;
  tone?: LedgerTone;
};

/** One filter, with how many rows answer to it. */
export type JobLedgerFilter = {
  id: string;
  /** Sentence case. */
  label: string;
  count: number;
};

export type JobLedgerProps = {
  /** The rows the chosen filter holds, newest first. */
  rows: readonly JobLedgerRow[];
  /** The strip above the table — All first, and the families after it. */
  filters: readonly JobLedgerFilter[];
  filter: string;
  onFilter: (id: string) => void;
  /**
   * One line under the strip, for what the counts cannot say — the rows All
   * holds that no filter does. **Absent draws nothing**, which is the ordinary
   * case: it appears only where the arithmetic would otherwise be a reader's
   * to do.
   */
  note?: ReactNode;
  /** Which row is open, held by the surface so a live redraw does not close it. */
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
  /** The open row, read whole. The body of the sheet a press opens. */
  inspector?: ReactNode;
  /** The folded inspector's own name, which is the open row's. */
  inspectorTitle?: string;
  /** Why no row is open, where none is. */
  inspectorAbsent?: string;
  /** What the table says where the chosen filter holds nothing. */
  emptyNote?: ReactNode;
  /**
   * How many rows are drawn. A Record grows for as long as the Job lives, and
   * what is left out is counted under the table rather than truncated quietly.
   */
  bound?: number;
  /**
   * Lead each row with its kind's mark. **On All alone**: under a filter that
   * names one family, a column of one repeated glyph says only what the strip
   * above it already said.
   */
  kindMarks?: boolean;
  /** The window is at `--window-floor`, where the folded inspector goes flush. */
  floor?: boolean;
  /**
   * The window is at or below `--layout-breakpoint`. The open row folds into a
   * sheet over the table rather than taking a column beside it.
   */
  narrow?: boolean;
  /** Controls at the trailing end of the filter row — the panel's own, not filters. */
  controls?: ReactNode;
};

/** A whole Job's worth of reading, without the list becoming the cost. */
const BOUND = 200;

export function JobLedger({
  rows,
  filters,
  filter,
  onFilter,
  note,
  openRow = null,
  onOpenRow,
  inspector,
  inspectorTitle,
  inspectorAbsent = "Press a row to read it whole",
  emptyNote = "Nothing under this filter yet",
  bound = BOUND,
  kindMarks = false,
  floor = false,
  narrow = false,
  controls,
}: JobLedgerProps) {
  const drawn = rows.slice(0, bound);
  const leftOut = rows.length - drawn.length;

  return (
    <>
      <div className="armada-ledger" data-narrow={narrow || undefined}>
        <section className="armada-ledger__list armada-glass" aria-label="What the Record holds">
          <div className="armada-ledger__head">
            <TabsWithCounts
              label="What the Record holds"
              value={filter}
              onChange={onFilter}
              items={filters.map((one) => ({ id: one.id, label: one.label, count: one.count }))}
            />
            {controls === undefined ? null : (
              <div className="armada-ledger__controls">{controls}</div>
            )}
          </div>

          {note === undefined ? null : (
            <p className="armada-ledger__note armada-ledger__note--inset" role="note">
              {note}
            </p>
          )}

          {drawn.length === 0 ? (
            <p className="armada-ledger__note armada-ledger__note--inset" role="note">
              {emptyNote}
            </p>
          ) : (
            <Table className="armada-ledger__table">
              <TableHead>
                <TableRow>
                  {/* The mark's heading is read and never drawn: ALL CAPS over a
                      12px glyph would be wider than the column it labels. */}
                  {kindMarks ? (
                    <TableHeaderCell className="armada-ledger__mark">
                      <span className="armada-ledger__mark-name">Kind</span>
                    </TableHeaderCell>
                  ) : null}
                  <TableHeaderCell className="armada-ledger__when">When</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__where">Where</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__who">Who</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__what">What</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__outcome">Outcome</TableHeaderCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {drawn.map((row) => (
                  <TableRow
                    key={row.id}
                    selected={row.id === openRow}
                    data-tone={row.tone}
                    data-row-id={row.id}
                    // The whole row opens it, the way a verdict's row opens its
                    // refusal: a sentence is a larger thing to hit than a glyph.
                    onClick={onOpenRow === undefined ? undefined : () => onOpenRow(row.id)}
                  >
                    {kindMarks ? (
                      <TableCell className="armada-ledger__mark">
                        {row.mark === undefined ? null : (
                          <Tooltip label={row.mark.says}>{row.mark.glyph}</Tooltip>
                        )}
                      </TableCell>
                    ) : null}
                    <TableCell
                      variant="metadata"
                      className="armada-ledger__when"
                      title={row.whenExact}
                    >
                      {row.when}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__where">
                      {row.where}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__who">
                      <span className="armada-ledger__actor" data-who={row.who}>
                        {row.whoSays}
                      </span>
                    </TableCell>
                    <TableCell className="armada-ledger__what">
                      {onOpenRow === undefined ? (
                        row.what
                      ) : (
                        // The keyboard's path to the same act, and the row's
                        // accessible name. The press stops here, or the row's
                        // own handler answers it a second time — harmless on
                        // screen, and a second report to whoever is counting.
                        <button
                          type="button"
                          className="armada-ledger__open"
                          aria-expanded={row.id === openRow}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenRow(row.id);
                          }}
                        >
                          {row.what}
                        </button>
                      )}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__outcome">
                      {row.outcome}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {leftOut > 0 ? (
            <p className="armada-ledger__note armada-ledger__note--inset" role="note">
              {leftOut} older {leftOut === 1 ? "row is" : "rows are"} not drawn
            </p>
          ) : null}
        </section>

        {narrow ? null : (
          <aside
            className="armada-ledger__pane armada-glass"
            aria-label={inspectorTitle ?? "This row"}
          >
            {inspector ?? (
              <p className="armada-ledger__note armada-ledger__note--inset" role="note">
                {inspectorAbsent}
              </p>
            )}
          </aside>
        )}
      </div>

      {/* Contained, so the shell's rail stays out from under the layer. */}
      <Sheet
        open={narrow && openRow !== null}
        contained
        floor={floor}
        title={inspectorTitle ?? "This row"}
        closeLabel="Close"
        closeBinding="Esc"
        bleed
        onClose={() => onOpenRow?.(null)}
      >
        <div className="armada-ledger__panel">
          {inspector ?? (
            <p className="armada-ledger__note" role="note">
              {inspectorAbsent}
            </p>
          )}
        </div>
      </Sheet>
    </>
  );
}
