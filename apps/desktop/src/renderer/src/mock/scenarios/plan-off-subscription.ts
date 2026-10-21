// The title bar's plan limits on an account off a subscription, such as an enterprise one. Fleet answers with
// no windows until a Session reports; a walk's `later` starts the wait, and 4 seconds after it a Session
// reports none, so the loading rings go and nothing takes their place. The walk `plan-off-subscription` plays it.

import type { PlanUsage, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const REPORTS_AFTER_MS = 4_000;

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...board,
    name: "plan-off-subscription",
    says: "The title bar's plan limits loading, then a Session on an account off a subscription reporting no windows",
    behaves: () => {
      const heard = new Set<(usage: PlanUsage) => void>();
      queueMicrotask(() =>
        onTimePassing(() => {
          setTimeout(() => {
            const none: PlanUsage = { windows: [], read_at: new Date().toISOString() };
            heard.forEach((on) => on(none));
          }, REPORTS_AFTER_MS);
        }),
      );
      return {
        getPlanUsage: async () => ({ ok: true, value: { windows: [] } }),
        onPlanUsageChanged: (on) => {
          heard.add(on);
          return () => void heard.delete(on);
        },
      };
    },
  };
}

export const planOffSubscription: Scenario = build();
