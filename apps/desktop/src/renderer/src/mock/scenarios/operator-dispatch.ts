// With the Operator on, his dispatch goes to it before any Job exists. The sheet goes and he stays where he was; a moment later a toast
// names the Job the Operator started, or the live Job it handed his words to, and a press opens it. Words about the store migration go
// to `Store migration spike`; anything else starts a Job of its own. Over `job-start-flow`. The walk `operator-dispatch` plays it.

import type { OperatorDispatch } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s340JobStartFlow } from "./job-start-flow";

/** How long the Operator takes to rule. */
const RULE_MS = 900;

/** The live Job words about the migration go to. */
const NEAR = {
  id: "s3",
  title: "Store migration spike",
};

export const s341OperatorDispatch: Scenario = {
  ...s340JobStartFlow,
  name: "operator-dispatch",
  says: "The Operator is on: a dispatch goes to it first, and a toast names the Job it started or handed the words to",
  draft: {
    ...s340JobStartFlow.draft,
    sessions: (board, more) => {
      const store = s340JobStartFlow.draft!.sessions!(board, more);
      const listeners = new Set<(one: OperatorDispatch) => void>();
      let ruled = 0;
      return {
        ...store,
        dispatch({ root, request, tuning, then }) {
          ruled += 1;
          const id = `d${ruled}`;
          const subject = request.split("\n")[0] ?? request;
          window.setTimeout(() => {
            if (/migration/i.test(request)) {
              store.send(NEAR.id, then);
              listeners.forEach((hear) =>
                hear({
                  id,
                  way: "handed",
                  session: NEAR,
                  subject,
                }),
              );
              return;
            }
            const made = store.start(undefined, root, request, tuning);
            if (typeof made !== "string") return;
            const title = subject.split(/\s+/).slice(0, 6).join(" ");
            store.edit(made, (one) => ({
              ...one,
              title,
            }));
            store.send(made, then);
            listeners.forEach((hear) =>
              hear({
                id,
                way: "started",
                session: {
                  id: made,
                  title,
                },
                subject,
              }),
            );
          }, RULE_MS);
          return Promise.resolve({
            operator: id,
          });
        },
        onDispatched(listener) {
          listeners.add(listener);
          return () => listeners.delete(listener);
        },
      };
    },
  },
};
