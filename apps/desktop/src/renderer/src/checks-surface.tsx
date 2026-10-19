// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run — the merge line, a run in the checkout. Apart from
// `App.tsx`, which is at its length.

import { useCallback, useMemo, useState } from "react";

import { ManifestChecks, type ManifestChecksProps } from "@armada/screens/src/ManifestChecks";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { openInPanel } from "./shared-panel/store";
import {
  followLandCheck,
  getCheckoutRunOutput,
  listCheckoutRuns,
  observeCheckoutRun,
} from "./commands";

/**
 * Where a requester link on this page sent a person: the merge line branch to mark, and the Session
 * whose Loadout is open. A Check opens in the side panel. **Each is let go by `goTo`, which clears it on
 * every move**, so a later visit by the rail opens plain.
 */
export function useAsked() {
  const [mergeFocus, setMergeFocus] = useState<string | undefined>(undefined);
  const [loadoutFocus, setLoadoutFocus] = useState<string | undefined>(undefined);
  const setCheckFocus = useCallback((next: string | undefined) => {
    if (next !== undefined) openInPanel({ kind: "check", id: next });
  }, []);
  return { mergeFocus, setMergeFocus, setCheckFocus, loadoutFocus, setLoadoutFocus };
}

type ChecksState = Pick<
  BridgeState,
  "checkoutRunSheet" | "checkoutRunFollowed" | "mergeLines" | "landFollowed" | "repository"
>;

/** What the Checks page reads: everything `ManifestChecks` takes but the window's floor. */
export function useChecksHost(
  state: ChecksState,
  onOpenMergeLine: (branch?: string) => void,
): Omit<ManifestChecksProps, "floor"> {
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
}: {
  state: ChecksState;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenMergeLine: (branch?: string) => void;
}) {
  const floor = useAtFloor();
  const host = useChecksHost(state, onOpenMergeLine);
  return (
    <Boundary region="Checks" bridge={bridge} onCopied={onCopied}>
      <ManifestChecks {...host} floor={floor} />
    </Boundary>
  );
}
