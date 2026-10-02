// Whether this Job's own read is still out, and why there is no brief.

import type { Watched } from "@armada/protocol";

/** Open the Studio a job came off, landing on the job's own node. */
export type OpenStudioFrom = (studioId: string, nodeId: string) => void;

/**
 * Whether this Job's own read has yet to answer, `read` or `failed`. `whole` is
 * `null` both while it has not and once Fleet refused, and only the first draws
 * as waiting.
 */
export function stillReading(watched: Watched, jobId: string): boolean {
  if (watched.state === "none") return true;
  if (watched.jobId !== jobId) return true;
  return watched.state !== "read" && watched.state !== "failed";
}

/**
 * Why there is no brief. **Three sentences, and none describes the wire** — a
 * Job that has not arrived, a Job Fleet would not answer for, and a Job nothing
 * has written one for yet, which are three different things to do next.
 */
export function whyNoBrief(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
  }
  // Read, and nothing wrote a brief: the proposer writes it, and the request is
  // the Job's own title until it does. Saying *reading this job* about a Job
  // Fleet has already answered for is the one reading that is never true.
  if (watched.state === "read" && watched.jobId === jobId) {
    return watched.detail.job.status === "proposing"
      ? "The proposer has not written one yet. The Job's title is the request as it was sent."
      : "No brief was written.";
  }
  return "Reading this job.";
}
