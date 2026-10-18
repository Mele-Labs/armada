// Calls that no Job's row holds. Main is red with nobody on it, and pull request #1896 failed its checks with nobody on it. Each is a call,
// and each pulses the place that already shows it: main's light and the pull request's chip in the merge queue. The one Job listed has
// told him something and asks nothing, so no row pulses. The walk `calls-pulse-where-they-live` plays it.

import type { HubPullRequest, MainStanding, MergeLines } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const told: Session = {
  id: "t1",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurn: "14:08",
  lastTurnAt: ago(10),
  standing: "replied",
  rows: [
    { id: "t1-u", at: "13:55:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "t1-a", at: "14:08:09", kind: "message", from: { kind: "agent" }, text: "The clock is pinned in all four store tests and the fixture is gone." },
  ],
  attachments: [],
};

const RED: MainStanding = {
  state: "red",
  commit: "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334",
  read_at: ago(1),
  red_since: ago(12),
  failed: [{ name: "desktop_test", check: "desktop_test", tests: ["resize.test.ts > keeps the sidebar width"] }],
};

const pull = (number: number, title: string, branch: string, ci: string): HubPullRequest => ({ number, title, branch, url: `${FORGE}${number}`, ci, opened_at: ago(40) });

const lines = (): MergeLines => ({
  lines: [
    {
      root: repository().root,
      line: [],
      off: [],
      landed: [],
      sent_back: [],
      hub: {
        main: RED,
        pull_requests: [pull(1896, "Fix the reconnect wait", "fleet/reconnect-wait", "failed"), pull(1892, "Add the pause marker column", "fleet/pause-marker-column", "passed")],
        merged: [],
      },
    },
  ],
});

function build(): Scenario {
  const base = holding("calls-pulse-where-they-live", "Main red and a failed pull request, each pulsing its place in the merge queue, with no Job calling");
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: lines() },
    later: [{}],
    draft: { sessions: () => sessionsStore([told]) },
  };
}

export const callsPulseWhereTheyLive: Scenario = build();
