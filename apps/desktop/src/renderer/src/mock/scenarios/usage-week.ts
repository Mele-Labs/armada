// The Usage page over a week of burn: Fleet answers a usage summary shaped from the measured baseline, the plan's weekly
// window reads 66%, and six Jobs hold the titles the list joins to. The walk `burnBySessionModelTool` plays it.

import type { PlanUsage, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { hosted } from "../sessions-fleet";
import { usageRulesNow } from "../usage-rules";
import { JOBS, usageSummaryNow } from "../usage-summary";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const plan = (): PlanUsage => {
  const later = (hours: number) => new Date(Date.now() + hours * 3_600_000).toISOString();
  return {
    windows: [
      { kind: "five_hour", percent_used: 4, resets_at: later(4) },
      { kind: "seven_day", percent_used: 66, resets_at: later(5 * 24) },
    ],
    read_at: new Date().toISOString(),
  };
};

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...board,
    name: "usage/week",
    says: "The Usage page over a week of burn, with six titled Jobs and the plan's weekly window at 66%",
    state: { ...board.state, sessions: { state: "read", sessions: JOBS.map((job) => hosted(job.id, { title: job.title })) } },
    behaves: () => ({
      getUsageSummary: async (tzOffsetMinutes) => ({ ok: true, value: { ...usageSummaryNow(tzOffsetMinutes), rules: usageRulesNow() } }),
      getPlanUsage: async () => ({ ok: true, value: plan() }),
      onPlanUsageChanged: () => () => {},
    }),
  };
}

export const usageWeek: Scenario = build();
