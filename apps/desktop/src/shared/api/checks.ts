// Checks: one running Check's log, a Session Check's output, and the merge line the Checks page reads.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { CheckOutputRead, FollowedLandLog, FollowedLog, LandCheckAt, MergeLines, Outcome } from "@armada/protocol";

export type ChecksApi = {
  /**
   * Read one running Check's log as it is written, or `null` to stop.
   *
   * **Read-only, like every read here**, and its own entry because it is its own
   * socket. `kept` is the last component of a `CheckUnderway.output_path`, and
   * Fleet resolves it against the Checks it is running before opening anything.
   */
  followCheckOutput: (jobId: string | null, kept: string | null) => Promise<void>;
  /**
   * Read one merge line Check's log, running or ended, or `null` to stop.
   *
   * **Read-only, `followCheckOutput`'s terms**, by the line's three names and never a path:
   * Fleet finds the file from the branch's outcome and opens nothing else.
   */
  followLandCheck: (at: LandCheckAt | null) => Promise<void>;
  /**
   * One Check a Session's agent ran, by its run id: `GET /sessions/checks/:run/output`. **It answers
   * once rather than publishing**: a recorded output is finished, and a person opening one Check is
   * asking about that Check. It brings the lines in, so a suite that went green for the wrong reason
   * can be argued with in the app.
   */
  readSessionCheckOutput: (run: number) => Promise<CheckOutputRead>;
  /**
   * Ask a Session's running Check to stop: `POST /sessions/checks/:run/stop` (`SIGTERM` to its group,
   * then `SIGKILL` after a grace period) or `/force_stop` (`SIGKILL`) where `force`. The run ends as any does.
   */
  stopSessionCheck: (run: number, force: boolean) => Promise<Outcome>;
};

export type ChecksState = {
  followed: FollowedLog;
  mergeLines: MergeLines | null;
  landFollowed: FollowedLandLog;
};

export const CHECKS_NOTHING_YET: ChecksState = {
  followed: { state: "none" },
  mergeLines: null,
  landFollowed: { state: "none" },
};

export const CHECKS_CHANNELS = {
  followCheckOutput: "bridge:follow-check-output",
  followLandCheck: "bridge:follow-land-check",
  readSessionCheckOutput: "bridge:read-session-check-output",
  stopSessionCheck: "bridge:stop-session-check",
} as const;
