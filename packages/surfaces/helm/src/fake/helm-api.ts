// Helm's members as a mock Fleet answers them: asking, reading a thought, debugging and pointing the dock.

import type { FleetHandle } from "@armada/bridge-api";
import type { HelmBoxFacts, Outcome } from "@armada/protocol";

import type { HelmApi, HelmState } from "../api";

const OK: Outcome = { ok: true };

/** What Helm's members reach: the conversation they publish, and the refusal a read with no session gives. */
export type HelmFleet = FleetHandle<HelmState> & {
  refused: (route: string) => { ok: false; outcome: Outcome };
};

export function helmApi({ publish, refused, state }: HelmFleet): HelmApi {
  return {
    answerHelmCall: async () => OK,
    askHelm: async () => OK,
    /**
     * Keeps the model and effort in the conversation's facts, as Fleet's `facts` message would
     * replace them, and leaves the command names as they were. A mock with no conversation open has
     * nothing to keep them in.
     */
    tuneHelm: async (_manifestId, model, effort) => {
      const held = state().helm;
      const facts: HelmBoxFacts = {
        ...(model === null ? {} : { model }),
        ...(effort === null ? {} : { effort }),
        ...((held.state === "open" || held.state === "failed") && held.facts.commands !== undefined ? { commands: held.facts.commands } : {}),
      };
      if (held.state === "open" || held.state === "failed") publish({ helm: { ...held, facts } });
      return { ok: true, facts };
    },
    // No file behind a mock Fleet unless a scenario keeps one: the refusal a window with nothing connected draws.
    readHelmFile: async () => refused("/helm/file"),
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
      publish({ helm: { state: "open", manifestId, replying: false, skipped: 0, missed: 0, items: [], facts: {} } }),
  };
}
