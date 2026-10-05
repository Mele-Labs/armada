// Overview's Jobs on a time axis: the graph view's cards, with the list's acts, placed by when
// they started. #920.
//
// **The playhead is Overview's, not this view's**: the strip above the panels draws the board as
// of it, so it is lifted. Null is now, and follows the clock.

import { JobTimeline } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import type { OverviewGraphActs } from "./overview-graph";
import { timelineBarsOf, timelineDispatchesOf } from "./overview-timeline";

export function OverviewTimeline({
  jobs,
  now,
  playhead,
  onPlayhead,
  acts,
}: {
  jobs: readonly JobSummary[];
  now: number;
  /** Epoch milliseconds, or null for now. */
  playhead: number | null;
  onPlayhead: (t: number | null) => void;
  acts: OverviewGraphActs;
}) {
  const t = playhead ?? now;
  return (
    <JobTimeline
      bars={timelineBarsOf(jobs, t, now, acts)}
      dispatches={timelineDispatchesOf(jobs)}
      playhead={t}
      now={now}
      onPlayhead={(next) => onPlayhead(next >= now ? null : next)}
    />
  );
}
