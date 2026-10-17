// A restart Fleet reports as failed. Nobody pressed anything at the moment it is read, so it is not a command's answer: it is
// what `get_fleet_build` says the last restart came to, raised once as a toast. The line is Fleet's own; the log holds the rest.

import type { BridgeCode, FailureDetail } from "@armada/components";
import type { BridgeIdentity, BuildSource } from "@armada/protocol";

import type { Failure } from "./notice";
import { logField, machineLog, versions } from "./notice";

/** Where `scripts/restart-build` writes what it ran. A Session reads it itself. */
export const RESTART_BUILD_LOG = "~/Library/Logs/Armada/restart-build.log";

/** The last restart did not take. */
const FLEET_BUILD_FAILED: BridgeCode = "bridge.fleet.build_failed";

/** What the report says, and which build the restart was moving onto where Bridge saw it start. */
export type BuildFailed = {
  /** The build the restart was moving onto. Absent where this window did not see it start. */
  build?: BuildSource;
  /** The build Fleet runs on. */
  running: BuildSource;
  commit?: string;
  /** The one line of plain facts Fleet reports. */
  line: string;
};

const NAME: Record<BuildSource, string> = { main: "Main", preview: "Preview" };

/** The headline a failed restart is raised under. */
export function buildFailedHeadline({ build, line }: BuildFailed): string {
  return `${build === undefined ? "Build" : `${NAME[build]} build`} failed: ${line}`;
}

/**
 * A restart that did not take, as the toast draws it.
 *
 * **A fault, and what survives is said first.** Fleet reported it, so it is up and on the build it was on: nothing here
 * touched a Drone or a Job.
 */
export function buildFailedFailure(failed: BuildFailed, bridge: BridgeIdentity): Failure {
  const headline = buildFailedHeadline(failed);
  const facts = [
    ...(failed.build === undefined ? [] : [{ key: "build", value: failed.build }]),
    { key: "running", value: failed.running },
    ...(failed.commit === undefined ? [] : [{ key: "commit", value: failed.commit }]),
    { key: "failed", value: failed.line },
    { key: "log", value: RESTART_BUILD_LOG },
  ];
  const details: FailureDetail[] = [
    ...(failed.build === undefined ? [] : [{ label: "Build", value: failed.build }]),
    { label: "Running", value: failed.running },
    ...(failed.commit === undefined ? [] : [{ label: "Commit", value: failed.commit.slice(0, 9) }]),
    { label: "Failed", value: failed.line },
    { label: "Log", value: RESTART_BUILD_LOG },
  ];
  return {
    kind: "fault",
    headline,
    next: `Fleet still runs on ${failed.running}. Dispatch a session to fix the build, or read the log.`,
    detailsLabel: "What failed",
    details,
    values: machineLog(bridge),
    note: "Fleet reports one line. The log holds the rest of the output.",
    payload: {
      code: FLEET_BUILD_FAILED,
      message: headline,
      fields: [...facts, ...logField(bridge)],
      ...versions(bridge),
    },
  };
}
