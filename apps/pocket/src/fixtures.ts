// Fixture data for the phone mock (#1997). Job rows are `JobSummary` fields from
// `@armada/protocol`; what the protocol has no field for yet (Judge verdict, Check
// results, the PR link) is local to the mock and named as such.

import type { JobSummary } from "@armada/protocol";

export type PocketJob = Pick<JobSummary, "id" | "handle" | "title" | "status" | "created_at" | "branch"> & {
  repository: string;
  /** Why it stopped, as the `reason.named` key Fleet gives. */
  reason?: "stalled" | "thrashing" | "rate_cap" | "interrupted";
  step?: { at: number; of: number; name: string };
  /** Ages are strings: the mock does not tick. */
  age: string;
  quiet?: string;
  verdict?: { says: "pass" | "veto" | "unsure"; line: string };
  checks?: { name: string; passed: boolean }[];
  pr?: { number: number; url: string };
  lastAction?: string;
};

const job = (rest: Omit<PocketJob, "created_at">): PocketJob => ({ created_at: "2026-10-08T09:00:00Z", ...rest });

export const BLOCKED: PocketJob[] = [
  job({ id: "j-2041", handle: "2041-the-drone-count-is-wrong", title: "The drone count is wrong", status: "escalated", reason: "stalled", repository: "armada", step: { at: 2, of: 5, name: "Implement" }, age: "2h 10m", quiet: "41m", verdict: { says: "unsure", line: "No verdict yet" }, checks: [{ name: "typecheck", passed: true }, { name: "desktop_test", passed: false }], lastAction: "Edited DroneCount.tsx" }),
  job({ id: "j-2038", handle: "2038-slot-pool-size", title: "Slot pool size survives a restart", status: "escalated", reason: "thrashing", repository: "armada", step: { at: 3, of: 5, name: "Verify" }, age: "3h 02m", quiet: "2m", verdict: { says: "veto", line: "Same Check failed 4 times" }, checks: [{ name: "typecheck", passed: true }, { name: "store_test", passed: false }], lastAction: "Ran store_test" }),
  job({ id: "j-2036", handle: "2036-tailnet-docs", title: "Tailnet setup page", status: "escalated", reason: "rate_cap", repository: "docs-site", step: { at: 1, of: 3, name: "Draft" }, age: "55m", quiet: "9m", verdict: { says: "unsure", line: "No verdict yet" }, checks: [], lastAction: "Rate limit reached" }),
  job({ id: "j-2033", handle: "2033-merge-line-order", title: "Merge line keeps its order", status: "escalated", reason: "interrupted", repository: "armada", step: { at: 4, of: 5, name: "Gate" }, age: "4h 40m", quiet: "1h 05m", verdict: { says: "pass", line: "Pass" }, checks: [{ name: "typecheck", passed: true }, { name: "ci", passed: true }], pr: { number: 2003, url: "example.test/armada/pull/2003" }, lastAction: "Fleet restarted" }),
];

export const APPROVAL: PocketJob = job({ id: "j-2044", handle: "2044-retry-the-export", title: "Retry the export", status: "awaiting_approval", repository: "ledger", age: "6m" });

export const REVIEW: PocketJob = job({ id: "j-2039", handle: "2039-pr-state-fresh", title: "PR rows stay current", status: "awaiting_review", repository: "armada", step: { at: 5, of: 5, name: "Review" }, age: "1h 20m", verdict: { says: "pass", line: "Pass, no objections" }, checks: [{ name: "typecheck", passed: true }, { name: "components_test", passed: true }, { name: "desktop_test", passed: true }], pr: { number: 1992, url: "example.test/armada/pull/1992" }, lastAction: "Opened the pull request" });

export const RUNNING: PocketJob[] = [
  job({ id: "j-2046", handle: "2046-sessions-gc", title: "Sessions are collected when they end", status: "running", repository: "armada", step: { at: 2, of: 4, name: "Implement" }, age: "14m", lastAction: "Edited sessions.rs" }),
  job({ id: "j-2045", handle: "2045-stale-branch-sweep", title: "Stale branch sweep", status: "running", repository: "armada", step: { at: 1, of: 4, name: "Plan" }, age: "3m", lastAction: "Read cleanup.rs" }),
  job({ id: "j-2042", handle: "2042-csv-import", title: "CSV import keeps leading zeros", status: "running", repository: "ledger", step: { at: 3, of: 4, name: "Verify" }, age: "48m", lastAction: "Ran import_test" }),
];

export const DONE: PocketJob[] = [
  job({ id: "j-2035", handle: "2035-fleet-protocol-banner", title: "Fleet protocol banner", status: "completed_success", repository: "armada", age: "1h", pr: { number: 1999, url: "example.test/armada/pull/1999" } }),
  job({ id: "j-2031", handle: "2031-footer-links", title: "Footer links", status: "completed_success", repository: "docs-site", age: "3h", pr: { number: 14, url: "example.test/docs-site/pull/14" } }),
  job({ id: "j-2029", handle: "2029-migrate-tokens", title: "Migrate tokens", status: "completed_failed", repository: "ledger", age: "5h" }),
];

export type SessionAsk = { id: string; title: string; repository: string; age: string; question: string; options: string[] };

export const HOSTED_ASK: SessionAsk = { id: "s-71", title: "Trim the pool", repository: "armada", age: "12m", question: "Close the idle bays now, or when their Jobs end?", options: ["Now", "When their Jobs end"] };

export const TERMINAL_WAITING = { id: "s-68", title: "Terminal in armada", repository: "armada", age: "27m" };

export const REPOSITORIES = ["armada", "ledger", "docs-site"];

export const MAC = "Studio Mac";
