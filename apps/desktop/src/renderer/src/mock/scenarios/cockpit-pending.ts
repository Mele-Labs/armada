// The Cockpit while answers go to Fleet. A Session asks two questions: Fleet takes the first, slowly,
// and drops it from the Session a moment after it answered; Fleet refuses the second. Each wait
// stands until a walk's step lets Fleet go (`cockpit-routes.ts`).

import type { Session } from "@armada/screens/src/draft/sessions";

import { REFUSES, SLOW_FLEET, gate } from "../cockpit-routes";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s14",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  rows: [
    { id: "p-u", at: "13:58:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests, and tidy the retry cap while you are there." },
    { id: "p-a", at: "14:00:11", kind: "message", from: { kind: "agent" }, text: "Two things before I go on." },
  ],
  attachments: [],
  waitingFor: [
    { id: "q1", text: "Which clock should the store tests use?", since: ago(3), source: "ask_card", act: { kind: "answer", target: "q-1" }, options: [{ label: "A fake the test sets" }, { label: "The real one, stopped" }] },
    { id: "q2", text: "How many retries should the cap allow?", since: ago(2), source: "ask_card", act: { kind: "answer", target: "q-2" }, options: [{ label: "Keep three" }, { label: "Raise it to five" }] },
  ],
};

// Fleet turns the second question down: the Session moved on while it sat there.
REFUSES.set("q2", { code: "fleet.session_busy", message: "Session Pin the store clock is already working on that" });

function build(): Scenario {
  const base = holding("cockpit-pending", "The Cockpit with two questions waiting on a Session, answered slowly by Fleet");
  return {
    ...base,
    state: { ...base.state, repository: null },
    // Each moment is a step of the walk, and only lets Fleet go: the answer, then the item dropping,
    // then the refusal.
    later: [{}, {}, {}],
    draft: {
      sessions: () => {
        const store = sessionsStore([SESSION]);
        const { held, ...clocks } = SLOW_FLEET;
        // Fleet answers when the walk lets it, so the waiting state stands as long as a person wants to look at it.
        const hold = held ? gate() : undefined;
        return {
          ...store,
          pace: { ...clocks, ...(hold === undefined ? {} : { held: hold }) },
          later: () => (store.later(), hold?.release()),
        };
      },
    },
  };
}

export const s208CockpitPending: Scenario = build();
