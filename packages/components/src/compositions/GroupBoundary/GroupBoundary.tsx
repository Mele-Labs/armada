import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_BOUNDARY } from "../../guides";
import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";

/**
 * A group's boundary — what runs once every task in the group has stopped, and
 * what it came to. `#1536`, and `docs/journeys/monitor-active-work.md`.
 *
 * **One strip per kind, segments inside it.** The bar spanned the card with
 * the Check names wrapping underneath, which read as a claim they lined up —
 * *"that is a big miss"* (owner, 28 Sep 2026). Nothing is under the bar now;
 * the names are behind the strip's own press.
 *
 * **A failed boundary opens itself**: what failed is why anybody is here.
 *
 * **It composes no sentence.** Every line of prose is the caller's.
 */

/** What one Check at the boundary came to. Spelled as the wire spells it. */
export type GroupBoundaryCheckReads = "not run" | "running" | "passed" | "failed";

export type GroupBoundaryCheck = {
  /** The Check's declared name — `screens_test`, `typecheck`. */
  name: string;
  reads: GroupBoundaryCheckReads;
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
   * boundary`. **The clause, not the sentence**: the strip draws how many, and
   * `7` beside `7 checks will run` is one number said twice.
   */
  clause: string;
  checks: readonly GroupBoundaryCheck[];
  /** Why no Check runs here. Drawn instead of the strip, never beside an empty one. */
  checksAbsent?: string;
  /** What the boundary came to — `all seven passed`, `screens_test failed`. */
  verdictSays?: string;
  verdictNamed?: FactChipNamed;
  /** `second run`, where the group has been run again. Absent on the first. */
  retrySays?: string;
  /** The commit the group left. Absent until it left one. */
  commit?: string;
  /**
   * What the gate wrote down about the Check that failed — what it measured
   * against, and what it produced. **Not the Check's output**, which is a file.
   */
  toldNext?: string;
  /** How the next Drone reaches the rest of it. Absent draws nothing. */
  toldNextSays?: string;
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

/** The hue a Check's chip takes. Only the two that are a verdict take one. */
const NAMED: Partial<Record<GroupBoundaryCheckReads, FactChipNamed>> = {
  passed: "passed",
  failed: "failed",
};

/**
 * One strip — the kind, how many, the run, and what it came to.
 *
 * **The whole strip is the press.** A chevron alone is a 12px target beside
 * 300px of row that does nothing, and the row is what a person aims at.
 */
function Strip({
  label,
  count,
  segments,
  trailing,
  chips,
  open,
  onToggle,
  children,
}: {
  label: string;
  count?: number;
  segments?: readonly TaskBarSegment[];
  trailing: ReactNode;
  chips?: ReactNode;
  open?: boolean;
  onToggle?: () => void;
  children?: ReactNode;
}) {
  const body = (
    <>
      <span className="armada-boundary__kind">{label}</span>
      {count === undefined ? null : <span className="armada-boundary__count">{count}</span>}
      {segments === undefined || segments.length === 0 ? null : (
        <span className="armada-boundary__bar">
          <StepBar tasks={segments} />
        </span>
      )}
      <span className="armada-boundary__gap" />
      {chips}
      <span className="armada-boundary__trailing">{trailing}</span>
      {onToggle === undefined ? null : (
        <ChevronDown className="armada-boundary__chevron" size={12} strokeWidth={2} aria-hidden />
      )}
    </>
  );
  return (
    <div className="armada-boundary__strip" data-open={open === true ? "true" : undefined}>
      {onToggle === undefined ? (
        <div className="armada-boundary__head">{body}</div>
      ) : (
        <button
          type="button"
          className="armada-boundary__head"
          aria-expanded={open === true}
          onClick={onToggle}
        >
          {body}
        </button>
      )}
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
  toldNext,
  toldNextSays,
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
            count={checks.length}
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
            <ul className="armada-boundary__checks">
              {/* What a boundary is. The one Armada word here, so the one mark. */}
              <li className="armada-boundary__guide">
                <GuideMark guide={GUIDE_GROUP_BOUNDARY} />
              </li>
              {checks.map((check) => (
                <li key={check.name} data-reads={check.reads}>
                  <span className="armada-boundary__check mono">{check.name}</span>
                  <FactChip named={NAMED[check.reads]}>{check.reads}</FactChip>
                </li>
              ))}
            </ul>
            {toldNext === undefined ? null : (
              <div className="armada-boundary__told">
                <span className="armada-boundary__eyebrow">What the gate wrote down</span>
                <pre className="armada-boundary__output">{toldNext}</pre>
                {toldNextSays === undefined ? null : (
                  <p className="armada-boundary__reaches">{toldNextSays}</p>
                )}
              </div>
            )}
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
