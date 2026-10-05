import { describe, expect, it } from "vitest";
import type { JobSummary } from "@armada/protocol";

import { job } from "./fixtures/build/base";
import { jobsAt, statusAt, timelineBarsOf, timelineDispatchesOf } from "./overview-timeline";

const T = (min: number) => new Date(Date.UTC(2026, 9, 5, 12, min)).toISOString();
const NOW = Date.parse(T(60));
const made = (status: string, over: Partial<JobSummary>): JobSummary => job(status, { branch: undefined, ...over });

const done = made("completed_success", { id: "a", created_at: T(0), started_at: T(5), ended_at: T(20) });
const live = made("awaiting_review", { id: "b", created_at: T(10), started_at: T(12), ended_at: undefined, dispatched_by: "a" });

describe("statusAt", () => {
  it("is absent before the Job existed, queued, running, then its own end state", () => {
    const at = (min: number) => statusAt(done, Date.parse(T(min)), NOW);
    expect([at(-1), at(2), at(10), at(20), at(30)]).toEqual([null, "queued", "running", "completed_success", "completed_success"]);
  });

  it("holds a live Job's own status only at now", () => {
    expect(statusAt(live, Date.parse(T(30)), NOW)).toBe("running");
    expect(statusAt(live, NOW, NOW)).toBe("awaiting_review");
  });
});

describe("jobsAt", () => {
  it("drops what did not exist yet and leaves the board alone at now", () => {
    expect(jobsAt([done, live], Date.parse(T(7)), NOW).map((one) => one.id)).toEqual(["a"]);
    expect(jobsAt([done, live], NOW, NOW)).toEqual([done, live]);
  });
});

describe("the timeline's reads", () => {
  it("runs a live bar to now and a started one from its start", () => {
    const [a, b] = timelineBarsOf([done, live], NOW, NOW, { onOpen() {}, onKill() {}, onRedispatch() {} });
    expect([a!.from, a!.to]).toEqual([Date.parse(T(5)), Date.parse(T(20))]);
    expect(b!.to).toBe(NOW);
  });

  it("reads a dispatch off dispatched_by, at the child's creation, and ignores one that is not on the board", () => {
    expect(timelineDispatchesOf([done, live])).toEqual([{ parent: "a", child: "b", at: Date.parse(T(10)) }]);
    expect(timelineDispatchesOf([live])).toEqual([]);
  });
});
