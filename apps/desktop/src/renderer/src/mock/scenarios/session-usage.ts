// Context fill and plan usage. Three live Sessions at three context levels, and the title bar holding the
// plan's limits at 5h 34% and wk 61%. A walk's `later` moves the limits to 5h 86% and wk 97%, so the
// warning and danger colours can be seen there too. The walk `session-usage` plays it.

import type { RepositorySummary } from "@armada/protocol";
import type { PlanUsage } from "@armada/components";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { mockPlanUsage } from "../plan-usage";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const used = (context_tokens: number, context_window: number) => ({ context_tokens, context_window });

const records = () => [
  hosted("01CTXLOWAAAAAAAAAAAAAAAAAA", { title: "Release notes script", last_turn_at: now(), usage: used(53_840, 1_000_000) }),
  hosted("01CTXMIDBBBBBBBBBBBBBBBBBB", { title: "Store migration spike", last_turn_at: now(), usage: used(142_000, 200_000) }),
  hosted("01CTXHIGCCCCCCCCCCCCCCCCCC", { title: "Cart totals rounding", last_turn_at: now(), usage: used(188_000, 200_000) }),
];

/** Today at this local hour and minute, as the mod would send it. */
function today(hour: number, minute: number): string {
  const at = new Date();
  at.setHours(hour, minute, 0, 0);
  return at.toISOString();
}

/** The coming Thursday at this local hour and minute. */
function thursday(hour: number, minute: number): string {
  const at = new Date();
  at.setDate(at.getDate() + ((4 - at.getDay() + 7) % 7 || 7));
  at.setHours(hour, minute, 0, 0);
  return at.toISOString();
}

const ROOMY: PlanUsage[] = [
  { kind: "five_hour", percentUsed: 34.2, resetsAt: today(18, 40) },
  { kind: "seven_day", percentUsed: 61.4, resetsAt: thursday(9, 0) },
];

const FULL: PlanUsage[] = [
  { kind: "five_hour", percentUsed: 86.1, resetsAt: today(18, 40) },
  { kind: "seven_day", percentUsed: 97.3, resetsAt: thursday(9, 0) },
];

function build(): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "session-usage",
    says: "Three live Sessions at three context levels, and the title bar holding the plan's 5-hour and weekly limits",
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle);
      // After the window has mounted and registered its own hook, and the latest wins.
      queueMicrotask(() => {
        mockPlanUsage.set(ROOMY);
        onTimePassing(() => mockPlanUsage.set(FULL));
      });
      return fleet;
    },
  };
}

export const s234SessionUsage: Scenario = build();
