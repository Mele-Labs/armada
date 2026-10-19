// Move, in the mock: a Session ends and a new one starts in another repository with the same title and its conversation.
// The slot, branch and files stay with the old one. **Recorded as Fleet records it**: a `forked_from` and a `forked_to` that
// carry `moved`, and the branch on `forked_from`; the conversation is the old Session's rows, which the new one reads.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { SessionsStore } from "./script";

/** How long Fleet takes to end the old Session and start the new one, long enough to see the press underway. */
const MOVE_MS = 700;

/**
 * The store, serving only the Sessions in `own`, and able to move one. `start` is the store's way of making a
 * Session, so the new one is made by it and then given the old one's conversation; the first start is the store's
 * own fixture and is spent by the caller.
 */
export function movable(store: SessionsStore, own: Set<string>): SessionsStore {
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
      const branches = old.attachments.flatMap((one) => (one.kind === "branch" ? [one.name] : [])).join(",");
      store.edit(made, () => ({
        id: made,
        ...(old.title === undefined ? {} : { title: old.title }),
        manifestId,
        ...(old.model === undefined ? {} : { model: old.model }),
        ...(old.effort === undefined ? {} : { effort: old.effort }),
        ...(old.mode === undefined ? {} : { mode: old.mode }),
        ...(old.context === undefined ? {} : { context: old.context }),
        attachments: [{ kind: "forked_from", id, ...(old.manifestId === undefined ? {} : { moved: old.manifestId }), ...(branches === "" ? {} : { branch: branches }) }],
        trail: { started: new Date().toISOString() },
        rows: [],
        turn: { state: "idle" },
        ...(old.lastTurnAt === undefined ? {} : { lastTurnAt: old.lastTurnAt }),
      }));
      // The old one keeps its slot, branch and files, and ends with where it went.
      store.edit(id, (one) => {
        const { asked, waitingFor, ...rest } = one;
        void asked;
        void waitingFor;
        return { ...rest, dead: "ended", turn: { state: "idle" }, attachments: [...one.attachments, { kind: "forked_to", id: made, moved: manifestId }] };
      });
      return made;
    },
  };
}
