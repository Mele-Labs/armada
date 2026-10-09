import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Labels on the left, their figures justified to the list's right edge where
 * the Fleet panel's rows sit — the key/value rows Pulse draws under a Job's
 * run, and the Fleet panel draws under its state.
 *
 * **One treatment, not two.** This was `JobHoldsSummary`'s own `Figure`, and a
 * second key/value style would drift from it the day either changed. So the
 * right edge is both callers' or neither's — Pulse's top line included, which
 * is the whole of what `words` and `detail` are for — at the accepted cost
 * that *Where things are*, not a `FigureList`, keeps its left-aligned values.
 *
 * **A figure with no value is not a row.** The caller leaves it out of the
 * list, because a label beside a blank reads the same whether the value is
 * nothing or never arrived.
 */

export type Figure = {
  /** Sans, `--text-label` — `Processes`, `pid`, `Drone`. */
  label: string;
  /** Mono, clipped from the right — `None`, `4242`, `171h 00m`. */
  value: string;
  /**
   * A second line under the value, ending on the same right edge — the instant
   * Pulse's top line was said. Mono `--text-2xs` `--fg-subtle`, the Fleet
   * panel's detail line at the width of one row.
   *
   * **It belongs to the value, so it goes where the value goes.** A time left
   * under a right-aligned phrase reads as a third column that lines up with
   * nothing.
   */
  detail?: string;
  /**
   * Whether the value is words somebody said rather than a figure — sans, and
   * held to two lines instead of clipped to one.
   *
   * **Pulse's top line is a sentence, and a sentence is not a reading.** It
   * carries a Drone's `Edit packages/settings/src/selectors.ts` as readily as
   * `thinking`, which is a turn to read in the log; mono and one line would
   * make it a figure that happens to be long.
   */
  words?: boolean;
  /** Whether the value is a fault. Draws it in `--error`. */
  wrong?: boolean;
  /**
   * A rule before this figure, splitting the list into two kinds of reading.
   *
   * **`strip` alone draws it.** A column of rows is already read down one
   * edge, and a rule across it would cut the edge in half; a strip runs across
   * with nothing between one figure and the next, which is what leaves the
   * groups indistinguishable. Pulse puts it before the first figure that is a
   * cost, so what is running reads apart from what it is taking.
   */
  apart?: boolean;
  /**
   * What pressing the figure does, where the thing it reads is changed
   * somewhere else — Pulse's Spend and Turns go to the caps on Settings.
   *
   * **The value is the button, and the label stays a `dt`.** A `dl` holds
   * terms and their definitions, and a button round both would be neither; so
   * the button is the definition, and on a `strip` it is drawn up behind the
   * label to take the whole figure — label, value and cap — as one press.
   */
  onPress?: () => void;
  /**
   * What the press does, said in the figure's tooltip and nowhere on screen —
   * the owner's rule that a bare thing names itself on hover. `Change the cost
   * cap in Settings`.
   */
  pressLabel?: string;
};

/**
 * How wide the label column is — where a value *starts* and so how much room
 * it has before it clips, not where it sits.
 *
 * `wide` is two `--space-12`, the column *Where things are* draws. `fit` sizes
 * it to the longest label, leaving the value every pixel left over: the Fleet
 * panel is 160px at its narrowest and `171h 55m` needs them.
 */
export type FigureColumn = "wide" | "fit" | "strip";

export type FigureListProps = {
  figures: Figure[];
  /**
   * **`strip` is not a column at all** — the label sits over its figure and
   * they run across, wrapping as the width allows. A panel is 160 to 380px
   * wide and a destination is the width of the window, where the right edge
   * puts a label and its figure a hand's width apart. #1538.
   */
  column?: FigureColumn;
  /**
   * A detail wraps under its value rather than clipping. **The Land board's
   * cost card only** (owner, 1 Oct 2026): `group one, group two, group three,
   * g…` cut the one thing the line was there to say. A panel 160px wide keeps
   * the clip, where a wrapped detail would be a column of single words.
   */
  wraps?: boolean;
};

export function FigureList({ figures, column = "wide", wraps = false }: FigureListProps) {
  return (
    <dl className="armada-figures" data-column={column} data-wraps={wraps || undefined}>
      {figures.map((figure) => {
        const lines = (
          <>
            <span className="armada-figures__reading">{figure.value}</span>
            {figure.detail === undefined ? null : <span className="armada-figures__detail">{figure.detail}</span>}
          </>
        );
        const press =
          figure.onPress === undefined ? null : (
            <button type="button" className="armada-figures__press" onClick={figure.onPress}>
              {lines}
            </button>
          );
        return (
          <div
            key={figure.label}
            className="armada-figures__row"
            data-wrong={figure.wrong || undefined}
            data-apart={figure.apart || undefined}
            data-presses={press === null ? undefined : true}
          >
            <dt className="armada-figures__label">{figure.label}</dt>
            {/* The value is a box of its own lines rather than the text itself,
                because a detail under it has to clip on its own terms — and a
                block dropped into a clipping `dd` takes none of its clipping. */}
            <dd className="armada-figures__value" data-words={figure.words || undefined}>
              {press === null
                ? lines
                : figure.pressLabel === undefined
                  ? press
                  : <Tooltip label={figure.pressLabel} asChild>{press}</Tooltip>}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}
