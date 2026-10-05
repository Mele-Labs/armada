// What the list row offers on a Job, for any view that draws one in place of the row. #920.
//
// The timeline's card carries these, so the two views never make a person switch to do something.

import type { JobSummary } from "@armada/protocol";
import { sectionOf } from "./board";
import { ACT_LABEL } from "./copy";
import { isTerminal } from "./Row";

export type OverviewActs = {
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onRedispatch: (jobId: string) => void;
};

export function actsOf(job: JobSummary, acts: OverviewActs): { label: string; onPress: () => void }[] {
  const redispatches = sectionOf(job) === "recently-ended" && job.status !== "rejected";
  return [
    ...(redispatches ? [{ label: ACT_LABEL.redispatch, onPress: () => acts.onRedispatch(job.id) }] : []),
    ...(isTerminal(job) ? [] : [{ label: "Kill", onPress: () => acts.onKill(job.id) }]),
  ];
}
