// A judge's concern on a Session, raised by the Judge tile with long parts: what was expected, what was found and what follows. Over the
// `one-session-view` scenario, with one of its Sessions given the concern. The walk `judgeConcernCard` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const QUIET = "one-view-2";

const judged = (one: Session): Session => ({
  ...one,
  work: {
    ...one.work,
    concerns: [
      {
        id: "jc-1",
        judge: "Judge",
        consequence: "Nothing in the dispatch panel reflects the feature, so a person starting a Session from it has no way to tell it exists or to use it.",
        expected: "A diff touching dispatch-panel that adds the queue act and the keys that start it, with a test beside each change, the registry rows those keys read from, and the generated vocabulary rebuilt from them, so the panel offers Queue wherever it offers Start.",
        produced: "No file has changed yet.",
        text: "Expected: A diff touching dispatch-panel that adds the queue act and the keys that start it, with a test beside each change, the registry rows those keys read from, and the generated vocabulary rebuilt from them, so the panel offers Queue wherever it offers Start.\nProduced: No file has changed yet.\nConsequence: Nothing in the dispatch panel reflects the feature, so a person starting a Session from it has no way to tell it exists or to use it.",
        cites: ["apps/desktop/src/renderer/src/dispatch-panel/dispatching.tsx"],
      },
    ],
  },
});

export const s263JudgeConcernCard: Scenario = {
  ...base,
  name: "judge-concern-card",
  says: "A judge's concern with its consequence, what was expected and what was found",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(QUIET, judged);
      return store;
    },
  },
};
