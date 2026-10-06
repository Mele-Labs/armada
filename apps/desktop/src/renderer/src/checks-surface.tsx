// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run — a Job's gate, a Drone, the merge line, a run in the checkout. Apart from
// `App.tsx`, which is at its length.

import { useEffect, useMemo, useState } from "react";

import { ManifestChecks, type JobOpening } from "@armada/screens";
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
 */
export function useAsked(openJob: string | null) {
  const [opening, setOpening] = useState<{ jobId: string; to: JobOpening } | null>(null);
  const [mergeFocus, setMergeFocus] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (openJob === null) setOpening(null);
  }, [openJob]);
  return { opening, setOpening, mergeFocus, setMergeFocus };
}

export function ChecksSurface({
  state,
  bridge,
  onCopied,
  onOpenJob,
  onOpenMergeLine,
}: {
  state: Pick<
    BridgeState,
    "checkoutRunSheet" | "checkoutRunFollowed" | "mergeLines" | "landFollowed" | "repository" | "jobs" | "followed"
  >;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenJob: (jobId: string, to?: JobOpening) => void;
  onOpenMergeLine: (branch?: string) => void;
}) {
  const floor = useAtFloor();
  const { mergeLines, repository, jobs } = state;
  const lines = useMemo(
    () => (mergeLines?.lines ?? []).filter((one) => repository === null || one.root === repository),
    [mergeLines, repository],
  );
  return (
    <Boundary region="Checks" bridge={bridge} onCopied={onCopied}>
      <ManifestChecks
        sheet={state.checkoutRunSheet}
        followed={state.checkoutRunFollowed}
        onObserveRun={observeCheckoutRun}
        onListRuns={listCheckoutRuns}
        onGetRunOutput={getCheckoutRunOutput}
        onReadChecks={readManifestChecks}
        onReadCheckOutput={readCheckOutput}
        onFollowCheckOutput={followCheckOutput}
        followedLog={state.followed}
        lines={lines}
        landFollowed={state.landFollowed}
        onFollowLand={followLandCheck}
        jobLabel={(jobId) => jobs.find((one) => one.id === jobId)?.handle ?? jobId}
        onOpenJob={onOpenJob}
        onOpenMergeLine={onOpenMergeLine}
        floor={floor}
      />
    </Boundary>
  );
}
