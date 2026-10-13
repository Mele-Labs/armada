// Helm's members as a mock Fleet answers them: asking, reading a thought, debugging and pointing the dock.

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
    // A mock Fleet reads every message as a question for Helm, at once, so only a scenario about
    // routing a thought changes what a dock does with its words.
    readThought: async () => ({ ok: true, reading: { kind: "question" } }),
    recordThoughtSent: async () => OK,
    startHelmFresh: async () => OK,
    /**
     * The dock's own switch, and *Discuss with Helm* on a card. **Which
     * repository Helm answers for is main's decision and not Fleet's**
     * (`main/helm.ts`), so the fake takes it rather than dropping it: a mock
     * where the one control that points Helm does nothing is a mock where
     * nothing pointed can be reached at all. A scenario that holds a Helm
     * Session answers this itself.
     */
    pointHelm: async (manifestId) => publish({ helm: { state: "pointed", manifestId } }),
  };
}
