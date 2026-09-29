import { useState, type ReactNode } from "react";
import { ChevronDown, ShieldCheck, ShieldMinus, ShieldX, type LucideIcon } from "lucide-react";

import { ConceptLabel } from "../../concepts";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_BOUNDARY } from "../../guides";
import { RowLink } from "../RowLink/RowLink";
import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";

/**
 * A group's boundary — what runs once every task in the group has stopped, and
 * what it came to. `#1536`, and `docs/journeys/monitor-active-work.md`.
 *
 * **One strip per kind, segments inside it.** Names wrapping under a bar read
 * as a claim they lined up (owner, 28 Sep 2026); they sit behind its press.
 *
 * **The head sums up only what is folded.** Bar, verdict and count say what
 * the rows say, so they go when the strip opens (owner, 29 Sep 2026). A Checks
 * strip carries no count: its bar is how many. `design-system.md`, rule 7.
 *
 * **A failed boundary opens itself.** **It composes no sentence**: every line
 * of prose is the caller's.
 */

/** What one Check at the boundary came to. Spelled as the wire spells it. */
export type GroupBoundaryCheckReads = "not run" | "running" | "passed" | "failed";

export type GroupBoundaryCheck = {
  /** The Check's declared name — `screens_test`, `typecheck`. */
  name: string;
  reads: GroupBoundaryCheckReads;
  /** What the Check was held to. Drawn under a Check that carries it, labelled `Expected`. */
  expected?: string;
  /** What it came back with. Drawn under a Check that carries it, labelled `Result`. */
  result?: string;
  /**
   * Open this Check's own Record row. **Absent where there is no row to open**
   * — a Check that never ran — and the row is then not a button.
   */
  onOpen?: () => void;
};

/** One case that runs at this same boundary, drawn apart from the Checks. */
export type GroupBoundaryTest = {
  id: string;
  /** The spec's repository path, or the case's own id where it has none. */
  spec: string;
  /** What the run came to, or why there is nothing to run — `not covered`. */
  reads: string;
  named?: FactChipNamed;
};

export type GroupBoundaryProps = {
  /**
   * The strip's trailing clause while nothing has run — `will run at this
   * boundary`. **The clause, not the sentence**: the bar draws how many, and
   * `7 checks will run` beside seven segments is one number said twice.
   */
  clause: string;
  checks: readonly GroupBoundaryCheck[];
  /** Why no Check runs here. Drawn instead of the strip, never beside an empty one. */
  checksAbsent?: string;
  /** What the boundary came to — `all passed`, `screens_test failed`. Drawn while folded. */
  verdictSays?: string;
  verdictNamed?: FactChipNamed;
  /** `attempt 2`, where the group has been run again. Absent on the first. */
  retrySays?: string;
  /** The commit the group left. Absent until it left one. */
  commit?: string;
  /** `run at this boundary`. Absent where no case does. */
  testsClause?: string;
  tests?: readonly GroupBoundaryTest[];
};

/** A Check's reading, on the bar's own segment grammar. */
const SEGMENT: Record<GroupBoundaryCheckReads, TaskBarSegment> = {
  "not run": "open",
  running: "working",
  passed: "done",
  failed: "failed",
};

/**
 * A Check's result mark — the `shield-*` family, `icons.toml`. `not run` is
 * `shield-minus`, Check's own not-reached. **Running takes none**: the registry
 * has no shield for it, and the bar's segment already says so.
 */
const MARK: Partial<Record<GroupBoundaryCheckReads, LucideIcon>> = {
  "not run": ShieldMinus,
  passed: ShieldCheck,
  failed: ShieldX,
};

/**
 * One Check, a row of its own (the owner, 29 Sep 2026: *each one get their own
 * row or maybe two columns of them*): its mark, its name, and what it came to.
 * **Pressing it opens the Check's own Record row**, which is where its output
 * and what it stopped are read.
 */
function CheckRow({ check }: { check: GroupBoundaryCheck }) {
  const Mark = MARK[check.reads];
  return (
    <li
      data-reads={check.reads}
      data-told={check.expected !== undefined || check.result !== undefined ? "true" : undefined}
    >
      <RowLink
        mark={Mark === undefined ? undefined : <Mark size={12} strokeWidth={2} />}
        mono
        says={check.reads}
        {...(check.reads === "passed" || check.reads === "failed" ? { tone: check.reads } : {})}
        label={`${check.name}, ${check.reads}`}
        {...(check.onOpen === undefined ? {} : { onOpen: check.onOpen })}
      >
        {check.name}
      </RowLink>
      {check.expected === undefined ? null : <Told label="Expected" value={check.expected} />}
      {check.result === undefined ? null : <Told label="Result" value={check.result} />}
    </li>
  );
}

/** A value under its label — the Record sheet's own field treatment. */
function Told({ label, value }: { label: string; value: string }) {
  return (
    <div className="armada-boundary__told">
      <ConceptLabel className="armada-inside__field-label">{label}</ConceptLabel>
      <p className="armada-boundary__said">{value}</p>
    </div>
  );
}

/** Whether a Check carries what it was held to or what it got. */
const told = (check: GroupBoundaryCheck) => check.expected !== undefined || check.result !== undefined;

/**
 * One strip — the kind, the run, and, while folded, how many and what it came to.
 *
 * **The whole strip is the press.** A chevron alone is a 12px target beside
 * 300px of row that does nothing, and the row is what a person aims at. The
 * button is the kind's label, and its press is stretched over the head, so the
 * guide mark beside the label is a button of its own rather than one nested in
 * another.
 *
 * **It wraps rather than cutting.** A narrow card puts the run and the verdict
 * on a line of their own instead of ending them in `…` (the owner, 29 Sep 2026:
 * *What is this saying? Its cut off?*).
 */
function Strip({
  label,
  guide,
  count,
  segments,
  trailing,
  chips,
  open,
  onToggle,
  children,
}: {
  label: string;
  /** The mark beside the label, where the kind has a guide. */
  guide?: ReactNode;
  count?: number;
  segments?: readonly TaskBarSegment[];
  trailing: ReactNode;
  chips?: ReactNode;
  open?: boolean;
  onToggle?: () => void;
  children?: ReactNode;
}) {
  // What sums up the rows, drawn only where the rows are not.
  const sums = onToggle === undefined || open !== true;
  return (
    <div className="armada-boundary__strip" data-open={open === true ? "true" : undefined}>
      <div className="armada-boundary__head" data-press={onToggle === undefined ? undefined : "true"}>
        {onToggle === undefined ? (
          <span className="armada-boundary__kind">{label}</span>
        ) : (
          <button
            type="button"
            className="armada-boundary__kind armada-boundary__press"
            aria-expanded={open === true}
            onClick={onToggle}
          >
            {label}
          </button>
        )}
        {guide}
        {count === undefined || !sums ? null : <span className="armada-boundary__count">{count}</span>}
        {segments === undefined || segments.length === 0 || !sums ? null : (
          <span className="armada-boundary__bar">
            <StepBar tasks={segments} />
          </span>
        )}
        <span className="armada-boundary__gap" />
        {chips}
        {sums ? <span className="armada-boundary__trailing">{trailing}</span> : null}
        {onToggle === undefined ? null : (
          <ChevronDown className="armada-boundary__chevron" size={12} strokeWidth={2} aria-hidden />
        )}
      </div>
      {open === true && children !== undefined ? (
        <div className="armada-boundary__detail">{children}</div>
      ) : null}
    </div>
  );
}

export function GroupBoundary({
  clause,
  checks,
  checksAbsent,
  verdictSays,
  verdictNamed,
  retrySays,
  commit,
  testsClause,
  tests = [],
}: GroupBoundaryProps) {
  const failed = checks.some((check) => check.reads === "failed");
  const [checksOpen, setChecksOpen] = useState(failed);
  const [testsOpen, setTestsOpen] = useState(false);
  return (
    <div className="armada-boundary">
      <section className="armada-boundary__region" aria-label="Checks at this boundary">
        {checks.length === 0 ? (
          <Strip
            label="Checks"
            trailing={
              <span className="armada-boundary__absent">
                {checksAbsent ?? "No Check runs at this group's end."}
              </span>
            }
          />
        ) : (
          <Strip
            label="Checks"
            // What a boundary is. The one Armada word here, so the one mark.
            guide={<GuideMark guide={GUIDE_GROUP_BOUNDARY} />}
            segments={checks.map((check) => SEGMENT[check.reads])}
            chips={
              <>
                {retrySays === undefined ? null : <FactChip>{retrySays}</FactChip>}
                {commit === undefined ? null : <FactChip title={commit}>{commit}</FactChip>}
              </>
            }
            trailing={
              verdictSays === undefined ? (
                clause
              ) : (
                <span className="armada-boundary__verdict" data-named={verdictNamed}>
                  {verdictSays}
                </span>
              )
            }
            open={checksOpen}
            onToggle={() => setChecksOpen((was) => !was)}
          >
            {/* The Check a person came for leads: what failed carries what it
                was held to and what it got, across the whole card. */}
            <ul className="armada-boundary__checks">
              {[...checks.filter(told), ...checks.filter((check) => !told(check))].map((check) => (
                <CheckRow key={check.name} check={check} />
              ))}
            </ul>
          </Strip>
        )}
      </section>

      {/* Apart from the Checks above, and never folded into them. Absent
          where there is no case, rather than empty under a heading. */}
      {tests.length === 0 || testsClause === undefined ? null : (
        <section className="armada-boundary__region" aria-label="Tests at this boundary">
          <Strip
            label="Tests"
            count={tests.length}
            trailing={testsClause}
            open={testsOpen}
            onToggle={() => setTestsOpen((was) => !was)}
          >
            <ul className="armada-boundary__tests">
              {tests.map((test) => (
                <li key={test.id}>
                  <span className="armada-boundary__check mono">{test.spec}</span>
                  <FactChip named={test.named}>{test.reads}</FactChip>
                </li>
              ))}
            </ul>
          </Strip>
        </section>
      )}
    </div>
  );
}
