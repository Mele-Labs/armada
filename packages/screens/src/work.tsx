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
 * Why there is no brief, where that is a failure: Fleet would not answer for
 * the Job. **A Job nothing has written one for yet, and a read still out, say
 * nothing** — an empty slot stays empty.
 */
export function whyNoBrief(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
  }
  return undefined;
}
