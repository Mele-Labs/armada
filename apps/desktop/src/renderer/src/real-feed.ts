// What the Cockpit and the Dashboard draw a Job's Now from. Main holds a detail and a Drone list for
// every active Job (`main/active-jobs.ts`), and the Now is built from the pair as the open Job's is,
// so a real Fleet fills the panel and the call cards without a draft. **The draft wins for a Job it
// names**: the mock carries rows Fleet does not serve, and a walk holds them.

import { callViewOf, nowViewOf } from "@armada/jobs";
import { droneViewsOf } from "@armada/jobs/draft/drone";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import type { Active, JobSummary } from "@armada/protocol";

export type Draft = {
  now?: Readonly<Record<string, NowView>> | undefined;
  calls?: Readonly<Record<string, CallView>> | undefined;
};

export type Feed = {
  nows: Readonly<Record<string, NowView>> | undefined;
  calls: Readonly<Record<string, CallView>> | undefined;
};

/** `undefined` where there is nothing to draw from, so a surface that tells the mock by it still can. */
const kept = <T>(rows: Record<string, T>, drafted: Readonly<Record<string, T>> | undefined) =>
  drafted === undefined && Object.keys(rows).length === 0 ? undefined : { ...rows, ...drafted };

export function feedOf(active: Active, board: readonly JobSummary[], draft?: Draft, at: number = Date.now()): Feed {
  const nows: Record<string, NowView> = {};
  const calls: Record<string, CallView> = {};
  for (const [jobId, read] of Object.entries(active)) {
    const view = nowViewOf({ whole: read.detail ?? null, drones: droneViewsOf(read.drones, read.detail), board, at });
    if (view === undefined) continue;
    nows[jobId] = view;
    const call = callViewOf(view);
    if (call !== undefined) calls[jobId] = call;
  }
  return { nows: kept(nows, draft?.now), calls: kept(calls, draft?.calls) };
}
