// The Checks surface, as the window draws it: every Check this repository's Manifest has had
// requested or run, off the checkout's run sheet. Apart from `App.tsx`, which is at its length.

import { ManifestChecks } from "@armada/screens";
import { Boundary, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { getCheckoutRunOutput, listCheckoutRuns, observeCheckoutRun } from "./commands";

export function ChecksSurface({
  state,
  bridge,
  onCopied,
}: {
  state: Pick<BridgeState, "checkoutRunSheet" | "checkoutRunFollowed">;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
}) {
  const floor = useAtFloor();
  return (
    <Boundary region="Checks" bridge={bridge} onCopied={onCopied}>
      <ManifestChecks
        sheet={state.checkoutRunSheet}
        followed={state.checkoutRunFollowed}
        onObserveRun={observeCheckoutRun}
        onListRuns={listCheckoutRuns}
        onGetRunOutput={getCheckoutRunOutput}
        floor={floor}
      />
    </Boundary>
  );
}
