// Issues' member: four open issues for whichever repository asks, so the composer's list has
// something to draw. What Fleet reads and holds is `fleet::tracking`'s.

import type { IssueList, IssueRead, MergeLines, PullRequestState } from "@armada/protocol";

import type { IssuesApi } from "../../../../shared/api/issues";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";
import { REPOSITORY_ISSUES, finishedPullRequest, issueOfNumber } from "../numbered";

/** A forge's name for an issue: its number after a `#`. */
const forge = (number: number): string => `#${number}`;

function listed(manifestId: string): IssueList {
  const own = REPOSITORY_ISSUES[manifestId];
  if (own !== undefined) return { manifest_id: manifestId, fetched_at: "2026-10-11T12:00:00Z", issues: own };
  return {
    manifest_id: manifestId,
    fetched_at: "2026-10-11T12:00:00Z",
    issues: [
      {
        reference: forge(412),
        title: "The log reader drops the last line",
        url: "https://forge.invalid/storefront/issues/412",
        labels: ["bug"],
        assignees: [],
        updated_at: "2026-10-11T11:40:00Z",
        excerpt: "A file that ends without a newline loses its last line in the viewer.",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(398),
        title: "Checkout button stays disabled after a failed card",
        url: "https://forge.invalid/storefront/issues/398",
        labels: [],
        assignees: ["nick"],
        updated_at: "2026-10-10T09:12:00Z",
        excerpt: "",
        linked_pull_request: true,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(405),
        title: "Coupon field accepts an expired code",
        url: "https://forge.invalid/storefront/issues/405",
        labels: ["bug"],
        assignees: [],
        updated_at: "2026-10-10T16:20:00Z",
        excerpt: "",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
      {
        reference: forge(409),
        title: "Order history pages past the last order",
        url: "https://forge.invalid/storefront/issues/409",
        labels: [],
        assignees: [],
        updated_at: "2026-10-09T08:05:00Z",
        excerpt: "",
        linked_pull_request: false,
        state: "open",
        fields: [],
        holders: [],
      },
    ],
  };
}

/** An open pull request the merge lines hold, as `get_pull_request` states it. */
function openOn(manifestId: string, number: number, lines: MergeLines | null): PullRequestState | undefined {
  const one = (lines?.lines ?? []).flatMap((line) => line.hub?.pull_requests ?? []).find((pr) => pr.number === number);
  if (one === undefined) return undefined;
  const checks: PullRequestState["checks"] = one.ci === "failed" ? { state: "failed", failing: [] } : one.ci === "passed" ? { state: "passed" } : { state: "pending" };
  return { manifest_id: manifestId, number, state: "open", auto_merge: false, ...(one.queue === undefined ? {} : { queued: true }), checks, title: one.title, branch: one.branch, address: one.url };
}

function issueRead(manifestId: string, number: number): IssueRead | undefined {
  const one = issueOfNumber(manifestId, number);
  if (one === undefined) return undefined;
  return { reference: one.reference, title: one.title, url: one.url, state: "open", body: one.excerpt, author: "", opened_at: one.updated_at, labels: one.labels, assignees: one.assignees, comments: [], linked_pull_requests: [], fields: [], holders: [] };
}

export const issues: Slice<IssuesApi, Record<string, never>> = {
  name: "issues",
  state: {},
  api: (_scenario, fleet) => ({
    listIssues: async (manifestId) => ({ ok: true, value: listed(manifestId) }),
    // `#` reads these through `mock/numbered.ts` here; the side panel reads the same fixtures, as a Fleet's answers.
    getIssue: async (manifestId, reference) => {
      const one = issueRead(manifestId, Number(reference.replace(/^#/, "")));
      return one === undefined ? { ok: false, outcome: unanswered("/issues") } : { ok: true, value: one };
    },
    getPullRequest: async (manifestId, number) => {
      const one = openOn(manifestId, number, fleet.state().mergeLines) ?? finishedPullRequest(manifestId, number);
      return one === undefined ? { ok: false, outcome: unanswered("/pull_requests") } : { ok: true, value: one };
    },
    // A forge that has not moved since the line was read: the refresh answers what a read would, and nothing changes.
    refreshPullRequest: async (manifestId, number) => {
      const one = openOn(manifestId, number, fleet.state().mergeLines) ?? finishedPullRequest(manifestId, number);
      return one === undefined ? { ok: false, outcome: unanswered("/pull_requests/refresh") } : { ok: true, value: one };
    },
  }),
};
