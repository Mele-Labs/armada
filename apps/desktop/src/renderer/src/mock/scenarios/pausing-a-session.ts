// Pausing a Session from Flight control and from its row. Pause stops the running turn, commits the work to the branch and gives the slot
// back; the Session stays listed, marked paused. Only Resume takes a slot again: a note typed to it asks to Resume and send first, and
// anything Fleet tells it waits in its thread. A paused Job that was waiting on him stops pulsing and drops its call. The claims are in
// `walks/pausing-a-session.ts` and `walks/pausing-quiets-a-call.ts`.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const RUNNING: Session = {
  id: "7c3d9a10-2b45-4e6f-8a91-0d1e2f3a4b5c",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
  ],
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const ASKING: Session = {
  id: "2e8b6f41-93c0-4d7a-b5e2-6a1c0d9f7e38",
  title: "Rebuild the stage grid",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(6),
  rows: [{ id: "g-1", at: ago(6), kind: "message", from: { kind: "agent" }, text: "Two grids fit the stage." }],
  attachments: [
    { kind: "slot", slot: 5 },
    { kind: "branch", name: "fix/stage-grid", slot: 5 },
  ],
  waitingFor: [{ id: "ask:grid:0", text: "Which grid should the stage use?", since: ago(5), source: "ask_card", options: [{ label: "Twelve columns" }, { label: "Sixteen columns" }] }],
};

const MINE = new Set([RUNNING.id, ASKING.id]);

function build(): Scenario {
  const base = holding("pausing-a-session", "The flight deck with one working Session and one waiting on him, each holding a slot and a branch");
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      sessions: () => {
        const store = sessionsStore([RUNNING, ASKING]);
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        const closed = new Set<string>();
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => MINE.has(one.id) && !closed.has(one.id));
            }
            return only;
          },
          // The slot goes back with the work committed; the Session keeps its branch.
          pause: async (id) => {
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, paused: true, turn: { state: "idle" }, standing: "idle", attachments: one.attachments.filter((it) => it.kind !== "slot") }));
            // Fleet tells the Job something while it is parked: the notice waits in its thread, and the Job stays paused.
            if (id === ASKING.id) {
              window.setTimeout(() => store.edit(id, (one) => ({ ...one, rows: [...one.rows, { id: "g-n", at: new Date().toISOString(), kind: "message", from: { kind: "session", id: "fleet", title: "Fleet" }, text: "main moved 3 commits ahead of fix/stage-grid.", notice: "behind_base" }] })), 1200);
            }
            return true;
          },
          // The mock's end: the row goes.
          close: (id) => {
            closed.add(id);
            store.edit(id, (one) => ({ ...one }));
          },
          // The note answers what it was asking, so the Job has nothing owed to him once it is sent.
          send: (id, sent) => {
            store.send(id, sent);
            if (id === ASKING.id) store.edit(id, ({ waitingFor: _, ...one }) => one);
          },
          resume: async (id) => {
            // Resume starts no turn: the Job that was waiting on him is waiting again, and the one that was working works.
            const waiting = id === ASKING.id;
            const bay = waiting ? 5 : 4;
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, turn: { state: waiting ? "idle" : "working" }, standing: waiting ? "idle" : "working", attachments: [{ kind: "slot", slot: bay }, ...one.attachments] }));
            return true;
          },
        };
      },
    },
  };
}

export const s264PausingASession: Scenario = build();
