import { Skeleton } from "../../primitives/Skeleton/Skeleton";

/** Two lines: what a brief on job detail usually wraps to. */
const FACTS_SKELETON_WIDTHS = ["90%", "60%"];

/**
 * The brief, before it has come back — what Overview's board draws in the
 * brief's place while the Job's own read is out.
 */
export function JobBriefSkeleton() {
  return (
    <div className="armada-job-brief" role="status" aria-label="Reading the brief" aria-busy>
      <div className="armada-job-brief__block">
        <div className="armada-job-brief__facts armada-skeleton-text">
          {FACTS_SKELETON_WIDTHS.map((width, i) => (
            <Skeleton key={i} width={width} />
          ))}
        </div>
      </div>
    </div>
  );
}
