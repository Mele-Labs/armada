// Issues' member: four open issues for whichever repository asks, so the composer's list has
// something to draw. What Fleet reads and holds is `fleet::tracking`'s.

import type { IssueList } from "@armada/protocol";

import type { IssuesApi } from "../../../../shared/api/issues";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";
import { REPOSITORY_ISSUES } from "../numbered";

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
      },
    ],
  };
}

export const issues: Slice<IssuesApi, Record<string, never>> = {
  name: "issues",
  state: {},
  api: () => ({
    listIssues: async (manifestId) => ({ ok: true, value: listed(manifestId) }),
    // `#` reads these through `mock/numbered.ts` here; a Fleet is what answers them.
    getIssue: async () => ({ ok: false, outcome: unanswered("/issues") }),
    getPullRequest: async () => ({ ok: false, outcome: unanswered("/pull_requests") }),
  }),
};
