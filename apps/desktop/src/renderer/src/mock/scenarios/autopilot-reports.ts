// The Cockpit with two repositories served, armada and storefront, and nothing running: the Autopilot setup draws Reported bugs only
// while armada is checked, and a start with it checked sets aside one report (`slices/autopilot.ts`). The walk `autopilot-reports` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const served = (id: string): RepositorySummary => ({
  root: `/Users/user/code/${id}`,
  records_root: `/records/${id}`,
  manifest: {
    ...repository().manifest!,
    id,
    repository: id,
    path: `${id}/armada.yml`,
  },
});
const ARMADA = served("armada");

function build(): Scenario {
  const base = holding(
    "autopilot-reports",
    "The Cockpit with Autopilot's setup offering Reported bugs for the armada repository",
    [
      ARMADA,
      served("storefront"),
    ],
  );
  return {
    ...base,
    state: {
      ...base.state,
      repository: ARMADA.root,
    },
    draft: {
      sessions: (board) => s200Sessions.draft!.sessions!(board, []),
    },
    behaves: (handle) => ({
      ...(base.behaves?.(handle) ?? {}),
      getPlanUsage: async () => ({
        ok: true,
        value: {
          windows: [
            {
              kind: "seven_day",
              percent_used: 62,
            },
          ],
          read_at: new Date().toISOString(),
        },
      }),
      onPlanUsageChanged: () => () => {},
    }),
  };
}

export const s212AutopilotReports: Scenario = build();
