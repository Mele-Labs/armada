// A message sent to a Session, while Fleet takes it. One Session is open on the flight deck. Fleet takes the first message slowly,
// refuses the second, and takes the second again when it is sent again. The message is in the thread from the press, with a mark
// that it is going, and Fleet's own row takes its place when it arrives. The claims are in `walks/session-message-sending.ts`,
// and each wait stands until one of its steps lets Fleet go (`SessionsStore.delivery`).

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { gate } from "../cockpit-routes";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const SESSION: Session = {
  id: "msg-1",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  context: { tokens: 62_000, window: 200_000 },
  turn: { state: "idle" },
  lastTurnAt: ago(4),
  standing: "idle",
  rows: [
    you("m-1", "13:58:00", "Pin the clock in the store tests."),
    agent("m-2", "14:11:03", "Pinned in clock.rs. The pool test still reads the wall clock."),
  ],
  attachments: [],
};

function build(): Scenario {
  const base = holding("session-message-sending", "The flight deck with one Session open, to which Fleet takes a message slowly, refuses one, and takes it when it is sent again");
  return {
    ...base,
    state: { ...base.state, repository: null },
    // Each moment is a step of the walk that lets Fleet answer the message waiting on it.
    later: [{}, {}, {}],
    draft: {
      sessions: () => {
        const store = sessionsStore([SESSION]);
        const delivery = { gate: gate(), outcomes: ["taken", "refused", "taken"] as ("taken" | "refused")[] };
        store.delivery = delivery;
        // The ship holds this Session alone; the mock's own first Session is spent.
        store.start();
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => one.id === SESSION.id);
            }
            return only;
          },
          later: () => delivery.gate.release(),
        };
      },
    },
  };
}

export const s264SessionMessageSending: Scenario = build();
