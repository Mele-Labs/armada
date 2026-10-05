// Overview's Jobs on a time axis, a lane to a family, with the list's acts on each Job's card. #920.
//
// **The playhead is Overview's, not this view's**: the strip above the panels draws the board as
// of it, so it is lifted. Null is now, and follows the clock.

import { JobTimeline } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import type { OverviewActs } from "./overview-acts";
import { familiesOf, timelineBarsOf } from "./overview-timeline";

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
  acts: OverviewActs;
}) {
  const t = playhead ?? now;
  const { families, alone, dispatches } = familiesOf(jobs);
  return (
    <JobTimeline
      bars={timelineBarsOf(jobs, t, now, acts)}
      dispatches={dispatches}
      families={families}
      alone={alone}
      playhead={t}
      now={now}
      onPlayhead={(next) => onPlayhead(next >= now ? null : next)}
    />
  );
}
