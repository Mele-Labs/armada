// A Session's thread names two other Sessions by their address, once in backticks (how an agent writes
// one) and once in plain text. Each is a link that opens that Session, the ended one too. Over the
// `sessions` scenario. The walk `session-id-links` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const now = new Date().toISOString();

const quiet = { turn: { state: "idle" }, lastTurnAt: now, attachments: [] } as const;

const reader: Session = {
  ...quiet,
  id: "e165b4dfc0a1",
  title: "Settings loader notes",
  rows: [{ id: "l1", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Picking this up from `s-e165b4df`." }],
};

const ended: Session = {
  ...quiet,
  id: "b7d20c9e11aa",
  title: "Retry loop findings",
  dead: "ended",
  rows: [{ id: "e1", at: "13:02:00", kind: "message", from: { kind: "agent" }, text: "Nothing more to add." }],
};

const holder: Session = {
  ...quiet,
  id: "4a9c0d13f7e2",
  title: "Hand-off thread",
  rows: [
    { id: "h1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Who has the loader?" },
    { id: "h2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Ask `s-e165b4df`, or read s-b7d20c9e before that. A `s-e165b4df --fix` stays code." },
  ],
};

// Another repository's Session, whose words name its own pull requests and issues, a merged one among them.
const FENCE = "```";
const forge: Session = {
  ...quiet,
  id: "c0ffee01aa11",
  title: "Cart rounding hand-off",
  manifestId: "storefront",
  attachments: [
    { kind: "pull_request", number: 2301, title: "Round the cart once", branch: "fix/round-once", address: "https://forge.invalid/pull/2301", checks: { state: "passed" }, state: "open", auto: false },
  ],
  rows: [
    { id: "f1", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Merged in #401, open in #418, the issue is #412, and #2207 is not this repository's." },
    { id: "f2", at: "14:40:12", kind: "message", from: { kind: "agent" }, text: "By address: https://forge.invalid/pull/401. In code: `#418` and `curl https://example.com/health`." },
    { id: "f3", at: "14:40:13", kind: "message", from: { kind: "agent" }, text: `${FENCE}sh\n# fix #412\ncurl https://example.com/ready\n${FENCE}` },
    { id: "f4", at: "14:40:14", kind: "message", from: { kind: "agent" }, text: "Mine is #2301." },
  ],
};

// A walk link, as Armada writes one, in plain text, as a markdown link, in inline code and in a block; and a localhost address that is not one.
const WALK = "http://localhost:47331/?walk=session-id-links";
const walker: Session = {
  ...quiet,
  id: "d3adbeef0001",
  title: "Walk links",
  rows: [
    { id: "w1", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: `Walk it: ${WALK}, or [the same](http://127.0.0.1:47331/?walk=session-id-links).` },
    { id: "w2", at: "14:40:12", kind: "message", from: { kind: "agent" }, text: `In code: \`${WALK}\`` },
    { id: "w3", at: "14:40:13", kind: "message", from: { kind: "agent" }, text: `${"```"}sh\nopen ${WALK}\n${"```"}` },
    { id: "w4", at: "14:40:14", kind: "message", from: { kind: "agent" }, text: "Not a walk: http://localhost:47331/docs" },
  ],
};

// Helm's repository and the one the last Session works in, so an issue named there opens the composer on its own.
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

export const s208SessionIdLinks: Scenario = {
  ...s200Sessions,
  name: "session-id-links",
  says: "A Session's thread names other Sessions by their address",
  state: { ...s200Sessions.state, holds: { ...s200Sessions.state.holds, repositories: [...(s200Sessions.state.holds.repositories ?? []), STOREFRONT] } },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [reader, ended, holder, forge, walker]) },
};
