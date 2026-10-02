import type { ReactNode } from "react";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";

/** A step of the run, named before the run itself has come back. */
export type RunTreeSkeletonStep = {
  id: string;
  label: ReactNode;
  labelIsAnIdentifier?: boolean;
};

export type RunTreeSkeletonProps = {
  /** The workflow's steps, in order. Absent or empty draws unnamed rows. */
  steps?: RunTreeSkeletonStep[];
  /** The step the Job is on. */
  current?: string;
};

/** Rows for a run whose workflow is not known either. */
const UNNAMED_WIDTHS = ["70%", "45%", "55%"];

/**
 * The run, before this Job's own read has answered.
 *
 * **The names, the step you are on, and one bar where the duration will go.**
 * Nothing else: a mark per row drew a column of empty boxes, and facts under
 * the current row guessed at how many it has. Both are what the read answers.
 * The row keeps `StepRow`'s grid, so a name does not move when its mark and
 * duration land beside it.
 */
export function RunTreeSkeleton({ steps = [], current }: RunTreeSkeletonProps) {
  const rows =
    steps.length > 0
      ? steps.map((step) => ({
          id: step.id,
          name: (
            <span className="armada-srow__name" data-identifier={step.labelIsAnIdentifier || undefined}>
              {step.label}
            </span>
          ),
          here: step.id === current,
        }))
      : UNNAMED_WIDTHS.map((width, at) => ({ id: String(at), name: <Skeleton width={width} />, here: false }));

  return (
    <div role="status" aria-label="Reading the run" aria-busy>
      <ol className="armada-run">
        {rows.map((row) => (
          <li className="armada-run__step" key={row.id}>
            {/* The group, because the row's own grid tracks are declared on it —
                without it every row collapses to one column. */}
            <div className="armada-srow-group">
              <div className="armada-srow" data-sel={row.here || undefined}>
                <span className="armada-srow__chevron" aria-hidden />
                {/* The mark's column, held empty. */}
                <span aria-hidden />
                {row.name}
                <Skeleton className="armada-run__dur-skeleton" />
              </div>
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}
