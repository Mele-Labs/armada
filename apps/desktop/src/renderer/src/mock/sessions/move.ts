// Move, in the mock: a Session ends and a new one starts in another repository with the whole conversation and the same title.
// The slot, branch and files stay with the old one. Mock-only; `SessionsDraft.move` says what a real Fleet owes.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { SessionsStore } from "./script";

/** How long Fleet takes to end the old Session and start the new one, long enough to see the press underway. */
const MOVE_MS = 700;

const stamp = () => new Date().toLocaleTimeString("en-GB");

/**
 * The store, serving only the Sessions in `own`, and able to move one. `start` is the store's way of making a
 * Session, so the new one is made by it and then given the old one's conversation; the first start is the store's
 * own fixture and is spent by the caller.
 */
export function movable(store: SessionsStore, own: Set<string>, repositoryOf: (manifestId: string) => string): SessionsStore {
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  return {
    ...store,
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    move: async (id, manifestId) => {
      const old = store.get().find((one) => one.id === id);
      // Fleet cannot move the person's terminal while it is listening.
      if (old === undefined || (old.terminal === true && old.dead === undefined)) return undefined;
      await new Promise<void>((resolve) => window.setTimeout(resolve, MOVE_MS));
      const made = await store.start();
      if (made === undefined) return undefined;
      own.add(made);
      const cameFrom: SessionRow = { id: `${made}-from`, at: stamp(), kind: "moved", way: "from", session: id, repository: old.manifestId === undefined ? "" : repositoryOf(old.manifestId) };
      const wentTo: SessionRow = { id: `${id}-to`, at: stamp(), kind: "moved", way: "to", session: made, repository: repositoryOf(manifestId) };
      store.edit(made, () => ({
        id: made,
        ...(old.title === undefined ? {} : { title: old.title }),
        manifestId,
        ...(old.model === undefined ? {} : { model: old.model }),
        ...(old.effort === undefined ? {} : { effort: old.effort }),
        ...(old.mode === undefined ? {} : { mode: old.mode }),
        ...(old.context === undefined ? {} : { context: old.context }),
        attachments: [],
        trail: { started: new Date().toISOString() },
        rows: [cameFrom, ...old.rows],
        turn: { state: "idle" },
        ...(old.lastTurn === undefined ? {} : { lastTurn: old.lastTurn }),
        ...(old.lastTurnAt === undefined ? {} : { lastTurnAt: old.lastTurnAt }),
      }));
      // The old one keeps its slot, branch and files, and ends with where it went.
      store.edit(id, (one) => {
        const { asked, waitingFor, ...rest } = one;
        void asked;
        void waitingFor;
        return { ...rest, dead: "ended", turn: { state: "idle" }, rows: [...one.rows, wentTo] };
      });
      return made;
    },
  };
}
