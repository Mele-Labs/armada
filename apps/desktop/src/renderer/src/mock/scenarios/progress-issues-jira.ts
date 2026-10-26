// `progress-issues` in a repository whose tracker names its issues by key: one Session holding two tickets, each with the pull requests
// `claim_issue` named on its row, one ticket closed. No pull request closes a key, so each stands under its ticket by the row's `pull_requests`.
// Each opens in the side panel, read from `../shared-panel-reads.ts`. The walk `progress-issues-jira` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { sessionsStore } from "../sessions/script";

const BARE = "https://git.example/armada/pull/";
const TRACKER = "https://tracker.invalid/browse/";

type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

const pr = (number: number, title: string, branch: string, rest: Partial<PullRequest> = {}): PullRequest => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `${BARE}${number}`,
  checks: { state: "passed" },
  state: "open",
  auto: false,
  ...rest,
});

const ticket = (key: string, title: string, pulls: number[], state: "open" | "closed" = "open"): SessionAttachment => ({ kind: "issue", number: key, title, address: `${TRACKER}${key}`, state, pull_requests: pulls });

const tickets: Session = {
  id: "pi-j",
  title: "Work two checkout tickets",
  manifestId: repository().manifest!.id,
  cwd: repository().root,
  turn: { state: "idle" },
  lastTurnAt: new Date(Date.now() - 4 * 60_000).toISOString(),
  standing: "idle",
  rows: [{ id: "pi-j-1", at: "13:40:00", kind: "message", from: { kind: "you" }, text: "Work ABC-41 and ABC-42." }],
  attachments: [
    ticket("ABC-41", "Checkout total rounds twice", [2301, 2303]),
    ticket("ABC-42", "Coupon field drops its code on a resize", [2302], "closed"),
    { kind: "branch", name: "checkout/round-once", slot: 7 },
    pr(2301, "Round the checkout total once", "checkout/round-once", { checks: { state: "pending" } }),
    pr(2302, "Keep the coupon code through a resize", "checkout/keep-coupon", { state: "merged" }),
    pr(2303, "Show the rounded total in the receipt", "checkout/receipt-total"),
  ],
  work: { proposal: { workflow: null } },
};

const base = holding("progress-issues-jira", "A Session's tickets by their tracker's key stand in Deferred, and the pull requests their rows name in the Landing");

export const s311ProgressIssuesJira: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: {
    sessions: () => sessionsStore([tickets], [], new Map(), []),
    areas: () => mockAreas("/armada"),
    sharedPanel: () => sharedPanelReads({ lines: [] }),
  },
};
