// Fleet offers a refresh of the preview on the Job holding a branch the preview left out, once that branch has
// moved past the tip it was left out at. The Fleet panel's build section marks the moved branch beside one
// that has not moved; its seed is `../fleet-build.tsx`'s, by name. The walks `preview-offer` and
// `preview-offer-fleet-panel` play it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s31",
  title: "Export billing as CSV",
  turn: { state: "idle" },
  lastTurnAt: ago(3),
  rows: [
    { id: "o1", at: "10:02:15", kind: "message", from: { kind: "you" }, text: "Add a CSV export to the billing page." },
    { id: "o2", at: "10:41:08", kind: "message", from: { kind: "agent" }, text: "The export is on the branch, with the ledger conflict resolved." },
  ],
  attachments: [{ kind: "slot", slot: 4 }, { kind: "branch", name: "feat/billing-export", slot: 4 }],
  waitingFor: [
    {
      id: "preview_offer:feat/billing-export",
      text: "feat/billing-export moved since the preview left it out: merge conflict with feat/ledger",
      since: ago(1),
      source: "preview_offer",
      act: { kind: "run", target: "scripts/preview --restart" },
    },
  ],
};

function build(): Scenario {
  const base = holding("preview-offer", "Fleet offers a refresh of the preview on the Job whose left out branch moved, and the Fleet panel marks that branch");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const s337PreviewOffer: Scenario = build();
