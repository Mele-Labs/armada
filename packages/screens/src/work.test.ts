// Whether a Job's own read is still out.

import { describe, expect, it } from "vitest";

import type { JobDetail, JobSummary, Watched } from "@armada/protocol";
import { stillReading } from "./work";

function job(): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "escalated",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
  };
}

function detail(over: Partial<JobDetail> = {}): JobDetail {
  return {
    job: job(),
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    facts: "The refresh path is in `auth/session.ts`.",
    ...over,
  };
}

const JOB_ID = "01M130Y1380016YK5S0JXBXDQ5";

describe("whether this job's own read has answered yet", () => {
  it("is still reading before anything has been asked", () => {
    expect(stillReading({ state: "none" }, JOB_ID)).toBe(true);
  });

  it("is still reading while this job's own read is in flight", () => {
    expect(stillReading({ state: "reading", jobId: JOB_ID }, JOB_ID)).toBe(true);
  });

  it("is not still reading once this job's read has come back", () => {
    const read: Watched = { state: "read", jobId: JOB_ID, detail: detail() };
    expect(stillReading(read, JOB_ID)).toBe(false);
  });

  it("is not still reading once Fleet has answered that it will not answer", () => {
    const failed: Watched = { state: "failed", jobId: JOB_ID, outcome: { ok: false, why: "not_connected" } };
    expect(stillReading(failed, JOB_ID)).toBe(false);
  });

  it("is still reading for a job that is not the one just read", () => {
    // A stale reading from the job open before this one must not be mistaken
    // for this job's own answer — the whole reason `JobRead` carries a
    // `jobId` rather than one flag every per-job read shares.
    const read: Watched = { state: "read", jobId: "a-different-job", detail: detail() };
    expect(stillReading(read, JOB_ID)).toBe(true);
  });
});
