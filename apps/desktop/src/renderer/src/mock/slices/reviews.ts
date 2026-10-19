// The Reviews tab's Fleet: the four lists, a pull request's page, its patches, a posted review and a resolved
// thread, served from the scenario's mock forge (`draft.prReview`, `../review-forge.ts`). A scenario with none
// answers that nothing is behind the window, as every other surface's slice does, except a pull request's page:
// the side panel opens a pull request from anywhere, so a number the forge holds none of reads as a bare page.

import { refusedWith } from "@armada/protocol";
import type { PullRequestPage } from "@armada/protocol";

import type { ReviewsApi } from "../../../../shared/api/reviews";
import type { Slice } from "../fake-context";
import { NO_FORGE, createMockForge } from "../review-forge";
import type { PullRequestPages } from "../session-pr-pages";

const barePage = (manifest_id: string, number: number): PullRequestPage => ({
  manifest_id,
  number,
  base: "main",
  additions: 0,
  deletions: 0,
  mergeable: "unknown",
  review: "none",
  approvals: 0,
  conversation: [],
  checks: [],
  files: [],
  commits: [],
});

const withBarePages = (forge: ReviewsApi): ReviewsApi => ({
  ...forge,
  readReviewPage: async (manifestId, number) => {
    const read = await forge.readReviewPage(manifestId, number);
    return read.ok ? read : { ok: true, value: barePage(manifestId, number) };
  },
});

/** The pages a scenario holds for a Session's pull requests, read and written ahead of the forge. */
const withPages = (forge: ReviewsApi, pages: PullRequestPages): ReviewsApi => ({
  ...forge,
  readReviewPage: async (manifestId, number) => {
    const page = pages.read(number);
    return page === undefined ? forge.readReviewPage(manifestId, number) : { ok: true, value: page };
  },
  writeReview: async (manifestId, write) => {
    if (pages.read(write.pull_request) === undefined) return forge.writeReview(manifestId, write);
    const done = pages.write(write.pull_request, write);
    if (!("said" in done)) return { ok: true, value: done };
    const path = `/pull_request_writes/${manifestId}`;
    return { ok: false, outcome: refusedWith(409, JSON.stringify({ code: "fleet.pull_request_write_refused", message: done.said }), { method: "POST", path }) };
  },
});

export const reviews: Slice<ReviewsApi, Record<string, never>> = {
  name: "reviews",
  state: {},
  api: (scenario) => {
    const forge = scenario.draft?.prReview === undefined ? NO_FORGE : createMockForge(scenario.draft.prReview);
    return withBarePages(scenario.draft?.pullRequestPages === undefined ? forge : withPages(forge, scenario.draft.pullRequestPages()));
  },
};
