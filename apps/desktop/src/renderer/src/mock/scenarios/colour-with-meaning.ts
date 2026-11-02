// Every page outside the Cockpit that holds a state, on one Fleet, so the walk `colour-with-meaning` can visit them: the Projects
// tab's board with its issues, a week of burn with the plan at 84%, Checks out and ended, pull requests to review, and Fleet's
// probes with a warning and a failure.

import type { PlanUsage } from "@armada/protocol";

import { checking } from "../checks-fleet";
import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { hosted } from "../sessions-fleet";
import { usageRulesNow } from "../usage-rules";
import { JOBS, usageSummaryNow } from "../usage-summary";
import { doctoring } from "../workflows-fleet";
import { PAIRING, SIGNING, s331Projects as board } from "./projects";

const plan = (): PlanUsage => {
  const later = (hours: number) => new Date(Date.now() + hours * 3_600_000).toISOString();
  return {
    windows: [
      {
        kind: "five_hour",
        percent_used: 4,
        resets_at: later(4),
      },
      {
        kind: "seven_day",
        percent_used: 84,
        resets_at: later(5 * 24),
      },
    ],
    read_at: new Date().toISOString(),
  };
};

const checks = checking();

/** The board's issues, two of them worked by its Jobs: one running, one waiting on him. */
const issues = (board.draft?.issuesTab ?? []).map((one) =>
  one.tracker.manifestId === "armada"
    ? {
        ...one,
        held: {
          "#2815": [
            PAIRING,
          ],
          "#2801": [
            SIGNING,
          ],
        },
      }
    : one,
);

/** Two pull requests asked of him: the one he opens first passed, and one failed. */
const review = () => {
  const seed = reviewSeed();
  return {
    ...seed,
    prs: seed.prs.map((one) =>
      one.number === 212
        ? {
            ...one,
            checks: "failed" as const,
          }
        : one,
    ),
  };
};

export const colourWithMeaning: Scenario = {
  ...board,
  name: "colour-with-meaning",
  says: "Usage, Doctor, Checks, Issues, Reviews and Projects on one Fleet, each holding a state worth a colour",
  state: {
    ...board.state,
    sessions: {
      state: "read",
      sessions: JOBS.map((job) =>
        hosted(job.id, {
          title: job.title,
        }),
      ),
    },
    health: doctoring().state.health,
    mergeLines: checks.state.mergeLines,
    repository: checks.state.repository,
  },
  draft: {
    ...board.draft,
    issuesTab: issues,
    prReview: review(),
  },
  behaves: (fleet) => ({
    ...board.behaves?.(fleet),
    ...checks.behaves?.(fleet),
    getUsageSummary: async (tzOffsetMinutes) => ({
      ok: true,
      value: usageSummaryNow(tzOffsetMinutes),
    }),
    getUsageRules: async () => ({
      ok: true,
      value: usageRulesNow(),
    }),
    getPlanUsage: async () => ({
      ok: true,
      value: plan(),
    }),
    onPlanUsageChanged: () => () => {},
  }),
};
