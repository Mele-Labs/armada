// Three Sessions on the Cockpit whose Progress lists the issues each works on, with their pull requests under them: one with no issue and two pull
// requests, one with an issue and a merged and an open pull request, one with two issues (one with a pull request, one without) and a pull
// request that closes neither, under an approved plan under way. Each opens in the side panel, read from `../shared-panel-reads.ts`. The walk `progress-issues` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const BARE = "https://git.example/armada/pull/";
const ISSUES = "https://forge.invalid/issues/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

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

const issue = (number: number, title: string, state: "open" | "closed" = "open"): SessionAttachment => ({ kind: "issue", number, title, address: `${ISSUES}${number}`, state });

const session = (id: string, title: string, minutes: number, asked: string, attachments: SessionAttachment[]): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(minutes),
  standing: "idle",
  rows: [{ id: `${id}-1`, at: "13:40:00", kind: "message", from: { kind: "you" }, text: asked }],
  attachments,
  work: { proposal: { workflow: null } },
});

/** No issue: two pull requests, after the issues there are none of. */
const loose = session("pi-a", "Speed up the Cockpit list", 6, "The Cockpit list stutters with forty Jobs. Make it smooth.", [
  { kind: "branch", name: "bridge/window-job-list", slot: 3 },
  pr(2251, "Window the Cockpit job list", "bridge/window-job-list"),
  pr(2253, "Drop the list's second render", "bridge/one-list-render", { checks: { state: "pending" } }),
]);

/** One issue, with a pull request merged and one open under it. */
const one = session("pi-b", "Keep a quiet Session after reload", 14, "Work #2214.", [
  issue(2214, "Sessions list drops a quiet Session on reload"),
  { kind: "branch", name: "fleet/pause-marker-column", slot: 4 },
  pr(2248, "Read the pause marker on load", "fleet/read-pause-marker", { state: "merged", closes: [2214] }),
  pr(1892, "Add the pause marker column", "fleet/pause-marker-column", { checks: { state: "pending" }, closes: [2214] }),
]);

/** Two issues, the first with a pull request and the second with none, and a pull request that closes neither. Its plan is approved and under way. */
const two: Session = {
  ...session("pi-c", "Fix Helm after a repository switch", 3, "Work #2212 and #2259.", [
    issue(2212, "Helm answers about the wrong repository after a switch"),
    issue(2259, "Helm composer keeps the old repository chip", "closed"),
    { kind: "branch", name: "helm/clear-on-switch", slot: 5 },
    pr(2256, "Clear Helm's cache on a repository switch", "helm/clear-on-switch", { checks: { state: "failed", failing: "test · bridge" }, closes: [2212] }),
    pr(2260, "Name Helm's repository in its header", "helm/header-repository"),
  ]),
  work: {
    proposal: { workflow: null },
    plan: {
      scope: ["crates/fleet", "apps/desktop"],
      approved: true,
      groups: [
        {
          id: "h1",
          agent: "rust-engineer",
          title: "Clear Helm's cache on a switch",
          paths: ["crates/fleet"],
          tasks: [
            { id: "H1", text: "Key the cache by repository", state: "done", seen: true },
            { id: "H2", text: "Drop the cache when the repository changes", state: "working", seen: true },
          ],
        },
        {
          id: "h2",
          agent: "bridge-engineer",
          title: "Name the repository in Helm",
          paths: ["apps/desktop"],
          tasks: [
            { id: "H3", text: "Show the repository in Helm's header", state: "done", seen: true },
            { id: "H4", text: "Read the chip from the switch", state: "open", seen: false },
          ],
        },
      ],
    },
  },
};

const base = holding("progress-issues", "Progress lists the issues a Session works on, each with its pull requests, then the pull requests that close none of them");

export const s311ProgressIssues: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: {
    sessions: () => sessionsStore([loose, one, two], [], new Map(), []),
    areas: () => mockAreas("/armada"),
    sharedPanel: () => sharedPanelReads({ lines: [] }),
  },
};
