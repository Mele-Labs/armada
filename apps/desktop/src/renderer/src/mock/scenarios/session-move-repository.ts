// A Session that started in the wrong repository, with three served: it holds a branch in shop, and the work
// belongs in armada. A second Session in shop runs in a terminal that is still listening, which is not moved.
// The walks `session-move-repository` and `cockpit-session-move-repository` play it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { movable } from "../sessions/move";
import { sessionsStore } from "../sessions/script";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada", repository: "armada" } };
const SHOP = served("shop");
const BILLING = served("billing");

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "tool", text });

const WRONG: Session = {
  id: "9d41c6a0-52be-4e7f-8a13-6c0b7e2f4d58",
  title: "Pin the store clock",
  manifestId: "shop",
  cwd: SHOP.root,
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(6),
  standing: "idle",
  trail: { started: ago(40), bays: [{ slot: 4, from: ago(30) }] },
  rows: [
    you("r-1", "13:31:00", "Pin the clock in the store tests, then open a pull request."),
    tool("r-2", "13:33:10", "Grep Clock::now"),
    { id: "r-3", at: "13:34:02", kind: "lease", slot: 4, branch: "fix/pin-store-clock" },
    tool("r-4", "13:34:30", "Edit src/cart/clock.ts"),
    agent("r-5", "13:41:30", "There are no store tests in this repository. The store crate is in armada."),
  ],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
  ],
};

const LISTENING: Session = {
  id: "c27e90b4-1f6d-4a85-b3c9-0e5a8d7f1263",
  title: "Why the reader drops lines",
  manifestId: "shop",
  cwd: SHOP.root,
  terminal: true,
  mode: "ask",
  turn: { state: "idle" },
  lastTurnAt: ago(20),
  standing: "idle",
  rows: [
    you("t-1", "13:46:00", "Why does the reader drop the last line of a stream?"),
    tool("t-2", "13:47:10", "Read src/feed/reading.ts"),
    agent("t-3", "13:48:02", "It splits on newlines and keeps nothing after the last one."),
  ],
  attachments: [],
};

function build(): Scenario {
  const base = holding("session-move-repository", "A Session in shop that belongs in armada, with a branch there, and a terminal Session that cannot be moved", [ARMADA, SHOP, BILLING]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      sessions: () => {
        const store = sessionsStore([WRONG, LISTENING]);
        // The first start is the store's own fixture, spent here so a move makes the next.
        store.start();
        return movable(store, new Set([WRONG.id, LISTENING.id]));
      },
    },
  };
}

export const s256SessionMoveRepository: Scenario = build();
