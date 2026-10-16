// A failure toast that can start a Session. Pressing New Session on the Sessions page raises an uncaught rejection from the
// Sessions store, as the real one did, so the toast stands with Copy debug info, File an issue, Dispatch session and
// Dismiss. Dispatch session starts a Session in Armada's repository told the debug info, the way a Session's Report does, and
// opens it. Over the `sessions` scenario. The walk `failure-dispatch-session` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/build/base";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

/** What the window hears when a promise nobody handled is rejected, without a real rejection the test runner would fail on. */
function rejectUnhandled(reason: Error): void {
  setTimeout(() => window.dispatchEvent(Object.assign(new Event("unhandledrejection"), { reason, promise: Promise.resolve() })), 0);
}

export const s261FailureDispatchSession: Scenario = {
  ...s200Sessions,
  name: "failure-dispatch-session",
  says: "New Session throws from the Sessions store; the failure toast offers Dispatch session beside Copy debug info",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board);
      // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the list.
      const spent = store.start();
      let crashed = false;
      let from: readonly Session[] | undefined;
      let only: readonly Session[] = [];
      return {
        ...store,
        get: () => {
          const all = store.get();
          if (all !== from) {
            from = all;
            only = all.filter((one) => one.id !== spent);
          }
          return only;
        },
        start: (tag, root) => {
          // The first start is the press that crashed. Every one after it works, in the repository it is given.
          if (!crashed) {
            crashed = true;
            rejectUnhandled(new TypeError("Cannot read properties of undefined (reading 'rows')"));
            return Promise.resolve(undefined);
          }
          const id = store.start(tag, root);
          if (typeof id === "string" && root === ROOT) store.edit(id, (one) => ({ ...one, manifestId: OWNER }));
          return id;
        },
      };
    },
  },
};
