// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run — a Job's gate, a Drone, the merge line, a run in the checkout. Apart from
// `App.tsx`, which is at its length.

import { useEffect, useMemo, useState } from "react";

import { ManifestChecks, type JobDetailProps, type JobOpening } from "@armada/jobs";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import {
  followCheckOutput,
  followLandCheck,
  getCheckoutRunOutput,
  listCheckoutRuns,
  observeCheckoutRun,
  readCheckOutput,
  readManifestChecks,
} from "./commands";

/**
 * Where a requester link on this page sent a person: the step or Drone a Job opens on, and the
 * merge line branch to mark. **Each is let go with what it was for** — the Job's when it closes, the
 * branch's by `goTo`, which clears it on every move — so a later visit by the rail opens plain.
 *
 * `host` is what a Job's Checks tab reads, the page's own reads under another name; `toMergeLine`
 * is the move to the merge line that a requester link makes.
 */
export function useAsked(openJob: string | null, state: ChecksState, toMergeLine: () => void) {
  const [opening, setOpening] = useState<{ jobId: string; to: JobOpening } | null>(null);
  const [mergeFocus, setMergeFocus] = useState<string | undefined>(undefined);
  const [checkFocus, setCheckFocus] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (openJob === null) setOpening(null);
  }, [openJob]);
  const host = useChecksHost(state, (branch) => {
    toMergeLine();
    setMergeFocus(branch);
  });
  return { opening, setOpening, mergeFocus, setMergeFocus, checkFocus, setCheckFocus, host };
}

type ChecksState = Pick<
  BridgeState,
  "checkoutRunSheet" | "checkoutRunFollowed" | "mergeLines" | "landFollowed" | "repository" | "jobs" | "followed"
>;

/**
 * What the Checks page reads, shared with a Job's Checks tab so the two list the same Checks off the
 * same reads. Everything `ManifestChecks` takes but the window's floor and where a link goes.
 */
export function useChecksHost(
  state: ChecksState,
  onOpenMergeLine: (branch?: string) => void,
): NonNullable<JobDetailProps["checks"]> {
  const { mergeLines, repository } = state;
  const lines = useMemo(
    () => (mergeLines?.lines ?? []).filter((one) => repository === null || one.root === repository),
    [mergeLines, repository],
  );
  return {
    sheet: state.checkoutRunSheet,
    followed: state.checkoutRunFollowed,
    onObserveRun: observeCheckoutRun,
    onListRuns: listCheckoutRuns,
    onGetRunOutput: getCheckoutRunOutput,
    onReadChecks: readManifestChecks,
    onReadCheckOutput: readCheckOutput,
    onFollowCheckOutput: followCheckOutput,
    followedLog: state.followed,
    lines,
    landFollowed: state.landFollowed,
    onFollowLand: followLandCheck,
    onOpenMergeLine,
  };
}

export function ChecksSurface({
  state,
  bridge,
  onCopied,
  onOpenJob,
  onOpenMergeLine,
  focus,
}: {
  focus?: string;
  state: ChecksState;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenJob: (jobId: string, to?: JobOpening) => void;
  onOpenMergeLine: (branch?: string) => void;
}) {
  const floor = useAtFloor();
  const { jobs } = state;
  const host = useChecksHost(state, onOpenMergeLine);
  return (
    <Boundary region="Checks" bridge={bridge} onCopied={onCopied}>
      <ManifestChecks
        {...host}
        jobLabel={(jobId) => jobs.find((one) => one.id === jobId)?.handle ?? jobId}
        onOpenJob={onOpenJob}
        floor={floor}
        {...(focus === undefined ? {} : { focus })}
      />
    </Boundary>
  );
}
