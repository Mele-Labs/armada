// Overview's Jobs placed as the graph their `waits_on` edges make, for `WaveCanvas`. #920.
//
// **The wave's own placement, over the board's Jobs.** `waveDepths` already puts a Job one column
// past the furthest of what it waits on, and the edge runs from the Job waited on to the Job
// waiting, as `waveRunOf` draws it. Only the cards differ: each carries the acts the list row
// offers, so the two views never make a person switch to do something.

import type { WaveCanvasEdge, WaveCanvasNode } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { sectionOf } from "./board";
import { ACT_LABEL } from "./copy";
import { isTerminal } from "./Row";
import { titleOf } from "./title";
import { waveDepths, waveNodeId } from "./wave";

const APART = 320;
const DOWN = 132;

export type OverviewGraphActs = {
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onRedispatch: (jobId: string) => void;
};

export type OverviewGraph = { nodes: WaveCanvasNode[]; edges: WaveCanvasEdge[] };

/**
 * One node per Job and one edge per dependency both ends of which are drawn.
 * A Job waiting on one the board does not hold draws no hanging edge.
 */
export function overviewGraphOf(jobs: readonly JobSummary[], acts: OverviewGraphActs): OverviewGraph {
  const held = new Set(jobs.map((job) => job.id));
  const depth = waveDepths(
    jobs.map((job) => ({ job: job.id, title: job.title, status: job.status, round: 1, waits_on: job.waits_on ?? [] })),
  );
  const down = new Map<number, number>();
  const nodes = jobs.map((job): WaveCanvasNode => {
    const at = depth.get(job.id) ?? 0;
    const row = down.get(at) ?? 0;
    down.set(at, row + 1);
    const redispatches = sectionOf(job) === "recently-ended" && job.status !== "rejected";
    const offered = [
      ...(redispatches ? [{ label: ACT_LABEL.redispatch, onPress: () => acts.onRedispatch(job.id) }] : []),
      ...(isTerminal(job) ? [] : [{ label: "Kill", onPress: () => acts.onKill(job.id) }]),
    ];
    return {
      id: waveNodeId(job.id),
      position: { x: at * APART, y: row * DOWN },
      card: {
        job: job.id,
        title: titleOf(job),
        status: job.status,
        handle: job.handle,
        onOpen: () => acts.onOpen(job.id),
        acts: offered,
      },
    };
  });
  const edges = jobs.flatMap((job) =>
    (job.waits_on ?? [])
      .filter((one) => held.has(one))
      .map((one): WaveCanvasEdge => ({ id: `${one}>${job.id}`, source: waveNodeId(one), target: waveNodeId(job.id) })),
  );
  return { nodes, edges };
}
