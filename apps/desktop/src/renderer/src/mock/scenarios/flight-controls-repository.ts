// Two repositories, lendinghome-monolith and armada, a Job open in each, and the actions each keeps for the owner alone On this Mac:
// none yet, for the walk `flight-controls-repository` to add one to lendinghome-monolith's and find it absent from armada's. The
// repositories' files are the mock slice's, kept for the life of the window (`slices/flight-controls.ts`). The owner's decision of 19 Oct 2026.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ARMADA = repository();
const MONOLITH_ID = "lendinghome-monolith";
const MONOLITH: RepositorySummary = {
  root: "/Users/user/lendinghome-monolith",
  records_root: "/Users/user/Library/Application Support/Armada/records/lendinghome-monolith",
  manifest: { ...repository().manifest!, id: MONOLITH_ID, repository: MONOLITH_ID, path: "lendinghome-monolith/armada.yml", records_root: "/Users/user/Library/Application Support/Armada/records/lendinghome-monolith" },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = (seconds: number): string => new Date(Date.now() - seconds * 1000).toTimeString().slice(0, 8);

const SLOT = 4;
const BRANCH = "mock/loan-pricing";

const PRICING: Session = {
  id: "fr1",
  title: "Upgrade the loan pricing gem",
  manifestId: MONOLITH_ID,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(2),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [{ id: "fr1-u", at: clock(600), kind: "message", from: { kind: "you" }, text: "Move loan_pricing to 4.2 and keep the specs green." }],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: BRANCH, slot: SLOT },
  ],
};

const STYLES: Session = {
  id: "fr2",
  title: "Tidy the deck stylesheet",
  manifestId: ARMADA.manifest!.id,
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(5),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [{ id: "fr2-u", at: clock(900), kind: "message", from: { kind: "you" }, text: "Fold the deck's repeated rules together." }],
  attachments: [],
};

function build(): Scenario {
  const base = holding("flight-controls-repository", "Two repositories with a Job open in each. An action added in Edit as Only in lendinghome-monolith is drawn in its Jobs and in no other", [MONOLITH, ARMADA]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: {
      slots: [
        {
          manifest_id: MONOLITH_ID,
          slot: SLOT,
          path: `${MONOLITH.root}/.armada/slots/slot-${SLOT}`,
          base: "main",
          warm: true,
          behind: 0,
          held: { state: "hosted_session", session_id: PRICING.id, session_title: PRICING.title ?? PRICING.id },
          branch: BRANCH,
          since: ago(10),
        },
      ],
    },
    draft: { sessions: (_control, more = []) => sessionsStore([...more, PRICING, STYLES], [], new Map(), []) },
  };
}

export const flightControlsRepository: Scenario = build();
