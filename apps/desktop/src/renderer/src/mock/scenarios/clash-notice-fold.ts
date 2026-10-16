// A Session whose clash with main names thirty-seven files, to see the list fold to three and a chip for the rest.
// Over the `sessions` scenario. The walk `clash-notice-fold` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import type { WriteClash } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s31",
  title: "Checkout card sync",
  turn: { state: "idle" },
  lastTurn: "14:40",
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "r1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Bring the checkout card onto the new hub." },
    { id: "r2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The card reads the hub now." },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "feat/checkout-card-hub", slot: 3 }],
};

const PATHS = [
  "apps/desktop/src/renderer/src/cockpit/checkout-diff.test.ts",
  "apps/desktop/src/renderer/src/cockpit/checkout-diff.ts",
  "apps/desktop/src/renderer/src/mock/checkout-card-wired.test.tsx",
  "apps/desktop/src/renderer/src/mock/checkout-held-hub.ts",
  "apps/desktop/src/renderer/src/mock/scenarios/checkout-held-back.ts",
  "apps/desktop/src/renderer/src/mock/walks/checkout-held-back.ts",
  "apps/desktop/src/renderer/src/sessions-wired.tsx",
  "crates/fleet/src/catching_up.rs",
  "crates/fleet/src/tests/checkout_caught_up.rs",
  "crates/ipc/src/merge_hub.rs",
  "crates/ipc/src/tests/merge_hub.rs",
  "docs/concepts/cockpit.md",
  "docs/concepts/fleet.md",
  "packages/protocol/src/merge-lines.ts",
  "packages/screens/src/draft/sessions.ts",
  ...Array.from({ length: 11 }, (_, i) => `crates/store/src/migrations/${String(i + 40).padStart(4, "0")}_checkout.rs`),
  ...Array.from({ length: 11 }, (_, i) => `packages/components/src/compositions/Checkout${i + 1}/Checkout${i + 1}.tsx`),
];

const CLASHES: readonly WriteClash[] = [
  { one: { kind: "session", id: SESSION.id, label: SESSION.title }, other: { kind: "main", id: "main", label: "main" }, paths: PATHS, since: minutesAgo(5) },
];

export const clashNoticeFold: Scenario = {
  ...s200Sessions,
  name: "clash-notice-fold",
  says: "A Session in a clash with main over thirty-seven files",
  draft: { ...s200Sessions.draft, clashes: CLASHES, sessions: (board) => s200Sessions.draft!.sessions!(board, [SESSION]) },
};
