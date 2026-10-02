import type { ReactNode } from "react";

import { Prose } from "../../primitives/Prose/Prose";
import { DestinationCard } from "../DestinationCard/DestinationCard";

/**
 * Verdict sheet — the record of a Job at the one place it stops for a person,
 * and again once it is over: a card per section in the Settings board's
 * balanced columns, then the buttons. **The owner's pick of three on 2 Oct
 * 2026** (#1680), and not folded: his note on the pick took the fold away.
 *
 * **The buttons decide nothing here.** `actions` is `Decide`'s own region;
 * where it is absent nothing is asked, and the dashed `recordNote` says so.
 */
export type VerdictFigure = {
  /** `Branch`, `Files`, `Took`, `Steps`, `Pull request`. */
  label: string;
  /** Absent where nothing serves it, which the caller says in words instead. */
  value?: ReactNode;
  absent?: ReactNode;
  /** Machine-derived — a branch, a count, a duration, a cost. */
  mono?: boolean;
};

export type VerdictSheetProps = {
  /**
   * The record's own headline, above the cards. **Only where the Job is
   * closed and a person answered it.** Both the word and the moment are the
   * screen's, off the transition that closed the Job.
   */
  header?: { done: ReactNode; when: ReactNode };
  /** What was asked for — the Job's own title. */
  title: ReactNode;
  /** The acceptance criteria the Job was frozen with, one line each. */
  criteria: readonly ReactNode[];
  /** What stands in for the criteria list where the Job carries none. */
  criteriaAbsent?: ReactNode;
  /** What was done — the Drone's own claim, `Submitted.claimed`, in its markdown. */
  cameBack: string;
  /** The deliverable this step kept, as a control that opens it. */
  deliverable?: ReactNode;
  /**
   * The pull request, where this Job has one — a `PullRequestCard`. **Presence
   * is the whole of what draws it**: absent is a workflow that never delivers,
   * or one whose Drone has not pushed yet.
   */
  pullRequest?: ReactNode;
  /** What proves it — the Job's Check lists, or the sentence that stands in for them. */
  provesIt: ReactNode;
  /** The line under the lists, where one is owed. */
  provesItNote?: ReactNode;
  /** What was not checked — Fleet's own `risks` section, in its markdown. */
  risks?: string;
  /** What was skipped — `Submitted.not_claimed`, or why there is nothing here. Markdown. */
  leftAlone: string;
  /** The figures, in the order the drawing runs them. Read, never edited. */
  figures: readonly VerdictFigure[];
  /** A standing sentence above the buttons — what merging costs, what ending here means. */
  note?: ReactNode;
  /** `Decide`'s own region. Absent is nothing being asked of anyone. */
  actions?: ReactNode;
  /** Drawn instead of `actions`, in the dashed frame an empty state takes. */
  recordNote?: ReactNode;
};

export function VerdictSheet({
  header,
  title,
  criteria,
  criteriaAbsent,
  cameBack,
  deliverable,
  pullRequest,
  provesIt,
  provesItNote,
  risks,
  leftAlone,
  figures,
  note,
  actions,
  recordNote,
}: VerdictSheetProps) {
  return (
    <div className="armada-verdict">
      {header === undefined ? null : (
        <div className="armada-verdict__header">
          <span className="armada-verdict__done">{header.done}</span>
          <span className="armada-verdict__when">{header.when}</span>
        </div>
      )}
      <div className="armada-verdict__cards">
        <DestinationCard label="What you asked for">
          <p className="armada-verdict__lede">{title}</p>
          {criteria.length === 0 ? (
            criteriaAbsent === undefined ? null : <p className="armada-verdict__said">{criteriaAbsent}</p>
          ) : (
            <ul className="armada-verdict__criteria">
              {criteria.map((one, i) => (
                <li key={i}>{one}</li>
              ))}
            </ul>
          )}
        </DestinationCard>

        <DestinationCard label="The work">
          {pullRequest}
          {figures.length === 0 ? null : (
            <dl className="armada-verdict__figures">
              {figures.map((figure) => (
                <div className="armada-verdict__figure" key={figure.label}>
                  <dt className="armada-verdict__figure-label">{figure.label}</dt>
                  <dd className="armada-verdict__figure-value" data-mono={figure.mono || undefined}>
                    {figure.value === undefined ? figure.absent : figure.value}
                  </dd>
                </div>
              ))}
            </dl>
          )}
        </DestinationCard>

        <DestinationCard label="What proves it">
          {provesIt}
          {provesItNote === undefined ? null : <p className="armada-verdict__said">{provesItNote}</p>}
        </DestinationCard>

        {risks === undefined ? null : (
          <DestinationCard label="What was not checked">
            <Said text={risks} />
          </DestinationCard>
        )}

        <DestinationCard label="What was done">
          <Said text={cameBack} />
          {deliverable === undefined ? null : <div className="armada-verdict__document">{deliverable}</div>}
        </DestinationCard>

        <DestinationCard label="What was skipped">
          <Said text={leftAlone} />
        </DestinationCard>
      </div>

      {note === undefined ? null : <p className="armada-verdict__said">{note}</p>}

      {actions === undefined ? (
        <div className="armada-verdict__record">
          {recordNote ?? "Nothing is asked of anyone. This is the Job's record."}
        </div>
      ) : (
        <div className="armada-verdict__actions">{actions}</div>
      )}
    </div>
  );
}

/** Markdown a Drone or Fleet wrote, at the record's reading size. */
function Said({ text }: { text: string }) {
  return (
    <div className="armada-verdict__said">
      <Prose text={text} />
    </div>
  );
}
