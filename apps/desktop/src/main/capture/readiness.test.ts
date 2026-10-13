import { expect, it } from "vitest";

import type { SessionRecord } from "@armada/protocol";
import { jobReadiness, sessionReadiness } from "./readiness";

const AT = "2026-10-12T10:00:00Z";
const url = "http://localhost:5173/";
const record = (detail: Record<string, string>): SessionRecord =>
  ({
    id: "a",
    harness: "h",
    origin: "bridge",
    cwd: "/repo",
    state: "live",
    started_at: AT,
    last_seen_at: AT,
    usage: {},
    attachments: [{ kind: "artifact", target: url, state: "standing", detail: { form: "window", ...detail }, since: AT, changed_at: AT }],
    waiting_for: [],
  }) as SessionRecord;

it("reads a Session's window as ready unless Fleet marks it working, and a Job's off where the Job stands", () => {
  expect(sessionReadiness(record({}), url)).toBe("ready");
  expect(sessionReadiness(record({ working: "true" }), url)).toBe("working");
  expect(sessionReadiness(record({}), "http://localhost:6006/")).toBeUndefined();
  expect(sessionReadiness(undefined, url)).toBeUndefined();
  expect(["awaiting_review", "awaiting_approval", "running", "queued", undefined].map(jobReadiness)).toEqual([
    "ready",
    "ready",
    "working",
    undefined,
    undefined,
  ]);
});
