// Jobs' mock data and its members as a mock Fleet answers them. Desktop's `mock/jobs-fake.ts` and
// `mock/slices/jobs.ts` register them; each moment and handler is generic over the app's whole state and API.
export * from "./fake/added-fleet";
export * from "./fake/approval-fleet";
export * from "./fake/check-logs-fleet";
export * from "./fake/feature-at-approval";
export * from "./fake/feature-gates";
export * from "./fake/feature-running";
export * from "./fake/held-fleet";
export * from "./fake/job-2-at-review";
export * from "./fake/job-2-landed";
export * from "./fake/job-2-repair";
export * from "./fake/job-3-retro";
export * from "./fake/job-detail-fixtures";
export * from "./fake/job-detail-refusal";
export * from "./fake/job-detail-undecided";
export * from "./fake/job-checks-fixture";
export * from "./fake/job-groups-fixture";
export * from "./fake/job-tiers-fixture";
export * from "./fake/jobs-api";
export * from "./fake/lessons-fleet";
export * from "./fake/origins-and-panel";
export * from "./fake/pause-fleet";
export * from "./fake/plan-fleet";
export * from "./fake/proposal-from-an-issue";
export * from "./fake/proposer-fleet";
export * from "./fake/prototype-fleet";
export * from "./fake/undecided-fleet";
