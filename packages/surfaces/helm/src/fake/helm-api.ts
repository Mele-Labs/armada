// Helm's members as a mock Fleet answers them: asking, debugging and pointing the dock.

import type { FleetHandle } from "@armada/bridge-api";
import type { Outcome } from "@armada/protocol";

import type { HelmApi, HelmState } from "../api";

const OK: Outcome = { ok: true };

/** What Helm's members reach: the conversation they publish, and the refusal a read with no session gives. */
export type HelmFleet = FleetHandle<HelmState> & {
  refused: (route: string) => { ok: false; outcome: Outcome };
};

export function helmApi({ publish, refused }: HelmFleet): HelmApi {
  return {
    answerHelmCall: async () => OK,
    askHelm: async () => OK,
    // No session behind a mock Fleet, so the record is the refusal a window
    // with nothing connected already draws — never an invented record.
    helmDebugInfo: async () => refused("/helm/debug"),
    startHelmFresh: async () => OK,
    /**
     * The dock's own switch, and *Discuss with Helm* on a card. **Which
     * repository Helm answers for is main's decision and not Fleet's**
     * (`main/helm.ts`), so the fake takes it rather than dropping it: this was
     * a no-op, and a mock where the one control that points Helm does nothing
     * is a mock where nothing pointed can be reached at all.
     *
     * What it publishes is a conversation with nothing in it, which is what
     * main's own socket publishes on a fresh connection whose backfill found
     * none — never an invented exchange. A scenario that holds one answers
     * this itself; `helm-talking` is pointed from the start and draws no
     * switch, so nothing there passes through here.
     */
    pointHelm: async (manifestId) =>
      publish({ helm: { state: "open", manifestId, replying: false, skipped: 0, missed: 0, items: [] } }),
  };
}
