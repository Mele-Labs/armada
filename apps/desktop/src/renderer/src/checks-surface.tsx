// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run — a Session's run, the merge line, a run in the checkout. Apart from
// `App.tsx`, which is at its length.

import { useMemo, useState } from "react";

import { ManifestChecks, type ManifestChecksProps } from "@armada/screens/src/ManifestChecks";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import {
  followCheckOutput,
  followLandCheck,
  getCheckoutRunOutput,
  listCheckoutRuns,
  observeCheckoutRun,
  readCheckOutput,
  readSessionCheckOutput,
  stopSessionCheck,
  readManifestChecks,
} from "./commands";

/**
 * Where a requester link on this page sent a person: the merge line branch to mark, and the Check to
 * have open. **Each is let go by `goTo`, which clears it on every move**, so a later visit by the rail
 * opens plain.
 */
export function useAsked() {
  const [mergeFocus, setMergeFocus] = useState<string | undefined>(undefined);
  const [checkFocus, setCheckFocus] = useState<string | undefined>(undefined);
  return { mergeFocus, setMergeFocus, checkFocus, setCheckFocus };
}

type ChecksState = Pick<
  BridgeState,
  "checkoutRunSheet" | "checkoutRunFollowed" | "mergeLines" | "landFollowed" | "repository" | "jobs" | "followed"
>;

/** What the Checks page reads: everything `ManifestChecks` takes but the window's floor and the label of a Job. */
export function useChecksHost(
  state: ChecksState,
  onOpenMergeLine: (branch?: string) => void,
): Omit<ManifestChecksProps, "floor" | "jobLabel"> {
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
    onReadSessionCheckOutput: readSessionCheckOutput,
    onStopSessionCheck: stopSessionCheck,
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
  onOpenMergeLine,
  focus,
  only,
  onClosed,
}: {
  focus?: string;
  only?: boolean;
  onClosed?: () => void;
  state: ChecksState;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenMergeLine: (branch?: string) => void;
}) {
  const floor = useAtFloor();
  const { jobs } = state;
  const host = useChecksHost(state, onOpenMergeLine);
  const checks = (
      <ManifestChecks
        {...host}
        jobLabel={(jobId) => jobs.find((one) => one.id === jobId)?.handle ?? jobId}
        floor={floor}
        {...(focus === undefined ? {} : { focus })}
        {...(only === true ? { only } : {})}
        {...(onClosed === undefined ? {} : { onClosed })}
      />
  );
  // Only the panel, over a Session: no Checks region laid out beside it.
  if (only === true) return checks;
  return (
    <Boundary region="Checks" bridge={bridge} onCopied={onCopied}>
      {checks}
    </Boundary>
  );
}
