// One Session holding five pull requests: one merged on main, a root on main with a chain of three above it, and a second child of the root. The Landing draws the
// stack parent first with one lane, a `base:` tag on the root and an `on #` chip on the child whose parent is not the row above. The walk `stacked-pull-requests` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

const pr = (number: number, title: string, branch: string, base: string, rest: Partial<PullRequest> = {}): PullRequest => ({
  kind: "pull_request",
  number,
  title,
  branch,
  base,
  address: `https://git.example/armada/pull/${number}`,
  checks: { state: "passed" },
  state: "open",
  auto: false,
  ...rest,
});

const stacked: Session = {
  id: "sp-a",
  title: "Split the export into steps",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(5),
  standing: "idle",
  rows: [{ id: "sp-a-1", at: "13:40:00", kind: "message", from: { kind: "you" }, text: "Split the export into steps, each its own pull request." }],
  attachments: [
    { kind: "branch", name: "export/format-names", slot: 3 },
    pr(3101, "Rename the export's formats", "export/rename-formats", "main", { state: "merged" }),
    pr(3102, "Read rows through one cursor", "export/cursor", "main"),
    pr(3103, "Page the cursor", "export/cursor-pages", "export/cursor", { checks: { state: "pending" } }),
    pr(3104, "Stream each page to disk", "export/stream", "export/cursor-pages", { state: "draft" }),
    pr(3105, "Resume a stopped stream", "export/resume", "export/stream", { checks: { state: "failed", failing: "test · export" } }),
    pr(3106, "Count rows while reading", "export/count", "export/cursor"),
  ],
  work: { proposal: { workflow: null } },
};

const base = holding("stacked-pull-requests", "A Job's pull requests stacked on one another read parent first in the Landing, joined by one lane, with the root's base and a chip naming a parent that is not the row above");

export const stackedPullRequests: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: {
    sessions: () => sessionsStore([stacked], [], new Map(), []),
    areas: () => mockAreas("/armada"),
    sharedPanel: () => sharedPanelReads({ lines: [] }),
  },
};
