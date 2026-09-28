import { ChevronRight, ChevronUp } from "lucide-react";
import type { ReactNode } from "react";

/**
 * A rounded panel in the left column — Navigation, Stats and Fleet stack
 * inside these. Bridge/1088.
 *
 * **Controlled, not persisted**: the caller reads and writes `open`, the way
 * `Sidebar`'s own `collapsed` works, so a restart can restore it.
 *
 * **Collapses to its head, never to nothing** — `trailing` stays visible
 * either way, so a glance still answers the one question the panel is for.
 *
 * `chevron-up` / `chevron-right`, `Chapter`'s own bare-toggle pair — not
 * `chevron-down`, which the mock draws by rotating one glyph in CSS.
 */
export type PanelProps = {
  label: ReactNode;
  /**
   * A mark against the label, on the head's leading edge — Fleet's status dot.
   * **Not `trailing`**: that slot is the head's opposite end, beside the
   * chevron, and a 6px dot 150px from the word it is about reads as decoration
   * rather than as the subject's state. Photographed at the column's 200px
   * resting width on 28 Sep 2026, which is what moved it.
   */
  mark?: ReactNode;
  /** Kept beside the label whether the panel is open or collapsed — a count, a dot. */
  trailing?: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /**
   * Below `--layout-breakpoint`. The head centres on one status dot and the
   * body never draws — the same floor the rail's own 48px form holds.
   */
  narrow?: boolean;
  /** The dot's tone at `narrow`, and nothing else — Panel carries no colour of its own. */
  dotTone?: "success" | "warn" | "escalated" | "muted";
  /**
   * What the `narrow` dot says in words. **Absent leaves it silent** — a named
   * region around an `aria-hidden` mark, so a reader hears "Fleet" and never
   * "running", which is every collapsed panel's state before 18 Sep 2026.
   *
   * The sentence is the caller's because only the caller has one: Fleet passes
   * `fleetSaid(label)`, and Stats' dot is a rollup of six rows with no wording
   * specified for it, so it passes none. See design-system.md → Left column.
   */
  dotLabel?: string;
  /**
   * **Absent draws no body and no chevron** — the head alone, and nothing to
   * press. A panel whose caller has nothing to put under the head is one where
   * a chevron would open an empty box, which is the same defect as a control
   * that looks pressable and does nothing. Fleet passes none in the one state
   * that has no figure, no sentence and no Doctor reading: `reading`, Bridge's
   * first moment before the runtime file has been read. Its dot still carries
   * the state in words.
   */
  children?: ReactNode;
};

export function Panel({
  label,
  mark,
  trailing,
  open,
  onOpenChange,
  narrow = false,
  dotTone = "muted",
  dotLabel,
  children,
}: PanelProps) {
  if (narrow) {
    return (
      <section className="armada-panel armada-glass" data-narrow aria-label={typeof label === "string" ? label : undefined}>
        {/* The head carries the name, and the dot keeps its `aria-hidden`: a
            6px `title` target is one a person misses, and the region's own
            name has to stay put rather than follow a status. */}
        <div
          className="armada-panel__head"
          {...(dotLabel === undefined ? {} : { role: "img", "aria-label": dotLabel, title: dotLabel })}
        >
          <span className="armada-panel__dot" data-tone={dotTone} aria-hidden />
        </div>
      </section>
    );
  }

  // Nothing to open, so nothing says it opens: the label and whatever sits
  // beside it, in the same 40px head, on a `div` rather than a `button`.
  if (children === undefined) {
    return (
      <section
        className="armada-panel armada-glass"
        data-bodyless
        aria-label={typeof label === "string" ? label : undefined}
      >
        <div className="armada-panel__head">
          <span className="armada-panel__name">
            <span className="armada-panel__label">{label}</span>
            {mark}
          </span>
          <span className="armada-panel__trailing">{trailing}</span>
        </div>
      </section>
    );
  }

  return (
    <section className="armada-panel armada-glass" data-open={open || undefined}>
      <button
        type="button"
        className="armada-panel__head"
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
      >
        <span className="armada-panel__name">
          <span className="armada-panel__label">{label}</span>
          {mark}
        </span>
        <span className="armada-panel__trailing">
          {trailing}
          {open ? (
            <ChevronUp size={14} strokeWidth={2.2} aria-hidden />
          ) : (
            <ChevronRight size={14} strokeWidth={2.2} aria-hidden />
          )}
        </span>
      </button>
      {open ? <div className="armada-panel__body">{children}</div> : null}
    </section>
  );
}
