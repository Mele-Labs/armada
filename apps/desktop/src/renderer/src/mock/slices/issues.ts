// Issues' member: two open issues for whichever repository asks, so the composer's list has
// something to draw. What Fleet reads and holds is `fleet::tracking`'s.

import type { IssueList } from "@armada/protocol";

import type { IssuesApi } from "../../../../shared/api/issues";
import type { Slice } from "../fake-context";

function listed(manifestId: string): IssueList {
  return {
    manifest_id: manifestId,
    fetched_at: "2026-10-11T12:00:00Z",
    issues: [
      {
        number: 412,
        title: "The log reader drops the last line",
        url: "https://forge.invalid/storefront/issues/412",
        labels: ["bug"],
        assignees: [],
        updated_at: "2026-10-11T11:40:00Z",
        excerpt: "A file that ends without a newline loses its last line in the viewer.",
        linked_pull_request: false,
      },
      {
        number: 398,
        title: "Checkout button stays disabled after a failed card",
        url: "https://forge.invalid/storefront/issues/398",
        labels: [],
        assignees: ["nick"],
        updated_at: "2026-10-10T09:12:00Z",
        excerpt: "",
        linked_pull_request: true,
      },
    ],
  };
}

export const issues: Slice<IssuesApi, Record<string, never>> = {
  name: "issues",
  state: {},
  api: () => ({
    listIssues: async (manifestId) => ({ ok: true, value: listed(manifestId) }),
  }),
};
