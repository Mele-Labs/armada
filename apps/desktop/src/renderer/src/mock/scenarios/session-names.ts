// The Sessions list with a name over its ledger, and a Session renamed from its header. Over the
// `sessions` scenario, with two more Sessions open beside its own: one from a terminal whose first
// prompt came wrapped in an agent message, and one holding fourteen pull requests, four of them merged. The walk
// `session-names` plays it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { cleanTitle } from "@armada/screens/src/sessions-wire";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;
const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pull = (number: number, checks: "passed" | "pending" | "failed", state: "open" | "merged" = "open"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: `Retire sleeps, part ${number - 1850}`,
  branch: `fix/retire-sleeps-${number - 1850}`,
  address: `https://example.com/pull/${number}`,
  checks: checks === "failed" ? { state: "failed", failing: "store: 2 failed" } : { state: checks },
  state,
  auto: false,
});

const more: Session[] = [
  {
    id: "s5",
    terminal: true,
    title: cleanTitle('<agent-message from="a45d14071172cd311">Review the ledger change'),
    turn: idle,
    lastTurn: "14:02",
    lastTurnAt: minutesAgo(31),
    rows: [],
    attachments: [{ kind: "branch", name: "review/ledger-change", slot: 1 }],
  },
  {
    id: "s6",
    terminal: true,
    title: "Retire the sleeps",
    turn: idle,
    lastTurn: "13:15",
    lastTurnAt: minutesAgo(3 * 24 * 60),
    rows: [],
    attachments: [
      { kind: "slot", slot: 2 },
      { kind: "branch", name: "fix/retire-sleeps-1", slot: 2 },
      pull(1851, "passed", "merged"),
      pull(1852, "passed", "merged"),
      pull(1853, "passed", "merged"),
      pull(1854, "passed", "merged"),
      pull(1855, "passed"),
      pull(1856, "pending"),
      pull(1857, "failed"),
      pull(1858, "passed"),
      pull(1859, "pending"),
      pull(1860, "passed"),
      pull(1861, "passed"),
      pull(1862, "failed"),
      pull(1863, "passed"),
      pull(1864, "pending"),
    ],
  },
];

export const s201SessionNames: Scenario = {
  ...s200Sessions,
  name: "session-names",
  says: "The Sessions list with the name over its ledger, and a Session renamed from its header",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
