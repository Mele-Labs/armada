// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run, off the checkout's run sheet and the merge line's Checks. Apart from `App.tsx`,
// which is at its length.

import { useMemo } from "react";

import { ManifestChecks } from "@armada/screens";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { followLandCheck, getCheckoutRunOutput, listCheckoutRuns, observeCheckoutRun } from "./commands";

export function ChecksSurface({
  state,
  bridge,
  onCopied,
  onOpenJob,
  onOpenMergeLine,
}: {
  state: Pick<BridgeState, "checkoutRunSheet" | "checkoutRunFollowed" | "mergeLines" | "landFollowed" | "repository" | "jobs">;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenJob: (jobId: string) => void;
  onOpenMergeLine: () => void;
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
