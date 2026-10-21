// The Job `pull-chip-refresh` opens: it holds pull request #2290, whose checks are running, and the forge has failed them
// by the time Refresh reads it. #2291 in the same line is a teammate's, held by no Job here.

import type { HubPullRequest, MergeLine, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { held, hosted, pullRequest } from "../sessions-fleet";
import type { ForgeReading } from "../sessions-fleet";

export const CONFIG_JOB = "01SESSIONCONFIGREADERSAAAA";
export const CONFIG_PR = 2290;
export const TEAMMATE_PR = 2291;
const AT = "2026-10-21T09:12:00.000Z";
const FORGE = "https://forge.example/armada/pull/";

const OURS = { title: "Who sees your config", branch: "fix/config-readers", address: `${FORGE}${CONFIG_PR}` };

export const configSessions = () => [
  hosted(CONFIG_JOB, {
    title: "Who sees your config",
    attachments: [held("slot", "3"), held("branch", OURS.branch), pullRequest(CONFIG_PR, { ...OURS, checks: "pending" })],
  }),
];

export const configThreads = (): Record<string, SessionRow[]> => ({
  [CONFIG_JOB]: [{ kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "List who can read the config file, and open a pull request." }],
});

const hub = (number: number, title: string, branch: string, ci: string, rest: Partial<HubPullRequest> = {}): HubPullRequest => ({ number, title, branch, url: `${FORGE}${number}`, ci, opened_at: AT, updated_at: AT, ...rest });

export const configLine = (): MergeLine => ({
  root: repository().root,
  line: [],
  off: [],
  landed: [],
  sent_back: [],
  hub: {
    pull_requests: [hub(CONFIG_PR, OURS.title, OURS.branch, "running"), hub(TEAMMATE_PR, "Bump the lockfile", "chore/lockfile", "passed", { author: "dana" })],
  },
});

/** What the forge says of #2290 when Refresh reads it: the test job failed. */
export const configFailed = (): Record<number, ForgeReading> => ({
  [CONFIG_PR]: { checks: "failed", head: "4be1c07d2a", failing: [{ name: "ci / test", log: "https://forge.example/armada/actions/runs/88/job/3" }] },
});
