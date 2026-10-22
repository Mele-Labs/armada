// A scenario's Sessions put on a workflow, as every Job Fleet hosts is (`packages/screens/src/session-work.ts`), so the Cockpit
// opens each as a Job: Field and Progress beside the thread, and what it made or used behind the Flight control keys. A Session
// with no work draws the older ledger, which only Helm still does. For a walk whose scenario a test still reads without work. Not
// under `scenarios/`, where every export is a scenario.

import type { SessionWork } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";

/** At Implement, on the feature workflow. */
export const IMPLEMENTING: SessionWork = { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } };

/** `scenario` named `name`, with the Sessions `ids` on `work`. */
export function onAWorkflow(scenario: Scenario, name: string, says: string, ids: readonly string[], work: SessionWork = IMPLEMENTING): Scenario {
  const sessions = scenario.draft!.sessions!;
  return {
    ...scenario,
    name,
    says,
    draft: {
      ...scenario.draft,
      sessions: (board, more) => {
        const store = sessions(board, more);
        for (const id of ids) store.edit(id, (one) => ({ ...one, work }));
        return store;
      },
    },
  };
}
