// The title bar's plan limits before any Session has reported: Fleet answers with no windows, and the
// ring glyph draws loading. A walk's `later` starts the wait; 4 seconds after it a real reading arrives
// (5h 12%, wk 87%). The walk `plan-loading` plays it.

import type { PlanUsage, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const ARRIVES_AFTER_MS = 4_000;

const fromNow = (hours: number): string => new Date(Date.now() + hours * 3_600_000).toISOString();

const reading = (): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: 12, resets_at: fromNow(4) },
    { kind: "seven_day", percent_used: 87, resets_at: fromNow(2 * 24) },
  ],
  read_at: new Date().toISOString(),
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...board,
    name: "plan-loading",
    says: "The title bar's plan limits with no reading yet, then a reading arriving",
    behaves: () => {
      const heard = new Set<(usage: PlanUsage) => void>();
      queueMicrotask(() =>
        onTimePassing(() => {
          setTimeout(() => {
            const arrived = reading();
            heard.forEach((on) => on(arrived));
          }, ARRIVES_AFTER_MS);
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

export const planLoading: Scenario = build();
