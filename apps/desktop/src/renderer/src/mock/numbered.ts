// Issues and pull requests for `#`, in the shapes a real Fleet answers them in: `TrackedIssue` from
// `list_issues`, `HubPullRequest` off the merge line's hub, and `PullRequestState` from
// `get_pull_request` for a number that is not open. Three repositories, the ones the routing
// scenarios serve; any other Manifest reads as armada.

import type { HubPullRequest, PullRequestState, TrackedIssue } from "@armada/protocol";

import { issueHit, lookedUpPullRequest, pullRequestHit } from "../numbered";
import type { NumberedSource } from "../numbered";

const issue = (number: number, title: string): TrackedIssue => ({
  reference: `#${number}`,
  title,
  url: `https://forge.invalid/issues/${number}`,
  labels: [],
  assignees: [],
  updated_at: "2026-10-11T11:40:00Z",
  excerpt: "",
  linked_pull_request: false,
});

/** An issue of a tracker that names them by key, as Jira does: its reference is the key. */
const keyed = (reference: string, title: string, labels: string[] = []): TrackedIssue => ({
  reference,
  title,
  url: `https://acme.atlassian.net/browse/${reference}`,
  labels,
  assignees: [],
  updated_at: "2026-10-11T11:40:00Z",
  excerpt: "",
  linked_pull_request: false,
});

const pull = (number: number, title: string, branch: string): HubPullRequest => ({
  number,
  title,
  branch,
  url: `https://forge.invalid/pull/${number}`,
});

const done = (manifest: string, number: number, title: string, state: PullRequestState["state"]): PullRequestState => ({
  manifest_id: manifest,
  number,
  state,
  auto_merge: false,
  checks: { state: "passed" },
  title,
  branch: number === 2207 ? "fix/drone-count" : `merged/${number}`,
  address: `https://forge.invalid/pull/${number}`,
});

type Repository = { issues: TrackedIssue[]; pulls: HubPullRequest[]; looked: PullRequestState[]; closed: TrackedIssue[] };

const REPOSITORIES: Record<string, Repository> = {
  armada: {
    issues: [
      issue(2214, "Sessions list drops a quiet Session on reload"),
      issue(2212, "Helm answers about the wrong repository after a switch"),
      issue(198, "Walk window keeps the old page after Back"),
      issue(87, "Retro items repeat across Sessions"),
    ],
    pulls: [
      pull(2219, "Pin the Bridge store clock", "fix/store-clock"),
      pull(2216, "Fold the two notification routes into one", "notify/one-route"),
      pull(203, "Sessions page: Active, Quiet and Ended views", "sessions/list-views"),
    ],
    looked: [done("armada", 2207, "Stop the Drone count drifting", "merged"), done("armada", 2190, "Drop the retired lock watchdog", "closed")],
    closed: [issue(61, "Doctor reports a stale pid as running")],
  },
  storefront: {
    issues: [
      issue(412, "The log reader drops the last line"),
      issue(409, "Order history pages past the last order"),
      issue(405, "Coupon field accepts an expired code"),
      issue(398, "Checkout button stays disabled after a failed card"),
    ],
    pulls: [pull(418, "Round cart totals once, at the end", "fix/cart-rounding"), pull(415, "Disable checkout while a card is declined", "fix/declined-card")],
    looked: [done("storefront", 401, "Paginate the order history", "merged")],
    closed: [],
  },
  billing: {
    issues: [issue(77, "Invoice PDF cuts the last line item"), issue(70, "Receipt emails send twice")],
    pulls: [pull(81, "Back off retries after the third failed charge", "retry/backoff")],
    looked: [done("billing", 74, "Fold the two invoice totals", "merged")],
    closed: [],
  },
  // A repository whose issues are in Jira, across two projects: `ABC` and `OPS`.
  ops: {
    issues: [
      keyed("ABC-123", "Checkout total rounds twice", ["bug"]),
      keyed("OPS-7", "Pager rota skips the night shift", ["on-call"]),
      keyed("ABC-118", "Order export drops the VAT column"),
      keyed("OPS-12", "Rotate the staging database credentials", ["security"]),
    ],
    pulls: [],
    looked: [],
    closed: [],
  },
};

/** The open issues of the repositories whose own lists differ from the generic four, for the composer's Open issues card. */
export const REPOSITORY_ISSUES: Record<string, TrackedIssue[]> = {
  armada: REPOSITORIES["armada"]!.issues,
  billing: REPOSITORIES["billing"]!.issues,
  ops: REPOSITORIES["ops"]!.issues,
};

const of = (manifestId: string | undefined): Repository => REPOSITORIES[manifestId ?? "armada"] ?? REPOSITORIES["armada"]!;

/** One issue of a repository, open or closed, by its number. */
export const issueOfNumber = (manifestId: string, number: number): TrackedIssue | undefined => {
  const held = of(manifestId);
  return [...held.issues, ...held.closed].find((one) => one.reference === `#${number}`);
};

/** A pull request the repository has finished with, as `get_pull_request` answers it. */
export const finishedPullRequest = (manifestId: string, number: number): PullRequestState | undefined => of(manifestId).looked.find((one) => one.number === number);

/** A lookup takes a moment, as a read through Fleet does. */
const LOOKUP_MS = 250;

export const mockNumbered: NumberedSource = {
  open: (manifestId) => [...of(manifestId).issues.map(issueHit), ...of(manifestId).pulls.map(pullRequestHit)],
  find: (manifestId, number) =>
    new Promise((resolve) =>
      setTimeout(() => {
        const held = of(manifestId);
        const pr = held.looked.find((one) => one.number === number);
        const closed = held.closed.find((one) => one.reference === `#${number}`);
        resolve(pr !== undefined ? lookedUpPullRequest(pr) : closed === undefined ? undefined : { ...issueHit(closed), standing: "Closed" });
      }, LOOKUP_MS),
    ),
};
