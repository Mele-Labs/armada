import { describe, expect, it } from "vitest";
import type { JobSummary } from "@armada/protocol";

import { job } from "./fixtures/build/base";
import { jobsAt, statusAt, familiesOf, timelineBarsOf } from "./overview-timeline";

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

});

describe("familiesOf", () => {
  const at = (id: string, min: number, by?: string) => made("running", { id, created_at: T(min), started_at: T(min), ended_at: undefined, ...(by === undefined ? {} : { dispatched_by: by }) });
  const ids = (jobs: JobSummary[]) => familiesOf(jobs).families.map((one) => one.members);

  it("follows a chain down to its last Job, and reads each edge at the child's creation", () => {
    const got = familiesOf([at("c", 20, "b"), at("a", 0), at("b", 10, "a")]);
    expect(got.families).toEqual([{ root: "a", members: ["a", "b", "c"] }]);
    expect(got.dispatches).toEqual([
      { parent: "a", child: "b", at: Date.parse(T(10)) },
      { parent: "b", child: "c", at: Date.parse(T(20)) },
    ]);
  });

  it("keeps a fan-out in one family, in the order minted", () => {
    expect(ids([at("a", 0), at("z", 5, "a"), at("y", 6, "a"), at("x", 7, "a")])).toEqual([["a", "z", "y", "x"]]);
  });

  it("sets aside a Job with no dispatcher and nothing it dispatched", () => {
    const got = familiesOf([at("a", 0), at("b", 5, "a"), at("solo", 3)]);
    expect(got.alone).toEqual(["solo"]);
    expect(got.families.map((one) => one.root)).toEqual(["a"]);
  });

  it("takes a Job whose dispatcher is not on the board as a root", () => {
    expect(familiesOf([at("b", 5, "gone"), at("c", 6, "b")]).families).toEqual([{ root: "b", members: ["b", "c"] }]);
    expect(familiesOf([at("b", 5, "gone")]).alone).toEqual(["b"]);
  });

  it("places every Job round a cycle once, and draws no edge into the Job it broke at", () => {
    const got = familiesOf([at("a", 0, "b"), at("b", 5, "a"), at("c", 8, "b")]);
    expect(got.families).toEqual([{ root: "a", members: ["a", "b", "c"] }]);
    expect(got.dispatches.map((one) => `${one.parent}>${one.child}`)).toEqual(["a>b", "b>c"]);
    expect(familiesOf([at("s", 0, "s")]).alone).toEqual(["s"]);
  });

  it("orders families by when their root was minted", () => {
    expect(familiesOf([at("late", 30), at("l2", 31, "late"), at("early", 1), at("e2", 2, "early")]).families.map((one) => one.root)).toEqual(["early", "late"]);
  });
});
