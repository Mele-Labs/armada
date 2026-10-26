// Four Sessions on the Cockpit whose issues stand in the Blueprint (a group that works one), in Deferred (an issue no group names) and the pull requests in Delivery: one with no issue and two pull
// requests, one with an issue and a merged and an open pull request, one with two issues (one with a pull request, one without) and a pull
// request that closes neither, under an approved plan under way whose two groups name the issues, and one whose plan of three groups names three
// issues, one with an open pull request whose Checks run, one with a merged one, one with none. Each opens in the side panel, read from `../shared-panel-reads.ts`. The walk `progress-issues` plays it.

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

const issue = (number: number, title: string, state: "open" | "closed" = "open"): SessionAttachment => ({ kind: "issue", number: String(number), title, address: `${ISSUES}${number}`, state });

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
          issue: "2212",
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
          issue: "2259",
          tasks: [
            { id: "H3", text: "Show the repository in Helm's header", state: "done", seen: true },
            { id: "H4", text: "Read the chip from the switch", state: "open", seen: false },
          ],
        },
      ],
    },
  },
};

/** A plan of three groups, each naming an issue the Session holds: the first with an open pull request whose Checks run, the second with a merged one, the third with none yet. Every pull request closes its issue. */
const three: Session = {
  ...session("pi-d", "Work three Helm tickets", 1, "Work #2271, #2272 and #2273.", [
    issue(2271, "Helm drops its draft when the window resizes"),
    issue(2272, "Helm shows the previous answer while a new one loads", "closed"),
    issue(2273, "Helm cannot be reached by the keyboard from the Cockpit"),
    { kind: "branch", name: "helm/keep-draft-on-resize", slot: 6 },
    pr(2281, "Keep Helm's draft across a resize", "helm/keep-draft-on-resize", { checks: { state: "pending" }, closes: [2271] }),
    pr(2282, "Clear the old answer when Helm asks again", "helm/clear-old-answer", { state: "merged", closes: [2272] }),
  ]),
  work: {
    proposal: { workflow: null },
    plan: {
      scope: ["apps/desktop"],
      approved: true,
      groups: [
        {
          id: "t1",
          agent: "bridge-engineer",
          title: "Keep the draft through a resize",
          paths: ["apps/desktop"],
          issue: "2271",
          tasks: [
            { id: "T1", text: "Hold the draft outside the resized pane", state: "done", seen: true },
            { id: "T2", text: "Restore the caret after the resize", state: "working", seen: true },
          ],
        },
        {
          id: "t2",
          agent: "bridge-engineer",
          title: "Clear the old answer",
          paths: ["apps/desktop"],
          issue: "2272",
          tasks: [
            { id: "T3", text: "Drop the answer when a question is sent", state: "done", seen: true },
            { id: "T4", text: "Show the wait in its place", state: "done", seen: true },
          ],
        },
        {
          id: "t3",
          agent: "bridge-engineer",
          title: "Reach Helm by keyboard",
          paths: ["apps/desktop"],
          issue: "2273",
          tasks: [
            { id: "T5", text: "Give Helm a Cockpit shortcut", state: "open", seen: false },
            { id: "T6", text: "Focus its composer on open", state: "open", seen: false },
          ],
        },
      ],
    },
  },
};

const base = holding("progress-issues", "The issues a Session works on stand on their plan group in the Blueprint or in Deferred, and their pull requests in Delivery, and a plan group can name the issue it works");

export const s311ProgressIssues: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: {
    sessions: () => sessionsStore([loose, one, two, three], [], new Map(), []),
    areas: () => mockAreas("/armada"),
    sharedPanel: () => sharedPanelReads({ lines: [] }),
  },
};
