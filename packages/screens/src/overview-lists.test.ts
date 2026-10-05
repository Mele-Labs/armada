// Overview's lists, case by case. `board.test.ts` already pins `sectionsOf`'s own rules; these
// tests pin only what `overview-lists.ts` adds on top of it — the scope, the fold, and Done left off.

import { describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { job } from "./fixtures/build/base";
import { nestedOf, overviewListsOf } from "./overview-lists";

const manifest = (id: string) => ({ id, repository: id, path: `${id}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] });
const SHOP: RepositorySummary = { root: "/Users/user/shop", records_root: "/records/shop", manifest: manifest("shop") };

describe("overviewListsOf", () => {
  it("draws every section in order, Done among them since the Job Board went", () => {
    const jobs = [
      job("completed_success", { id: "done" }),
      job("queued", { id: "q" }),
      job("running", { id: "r" }),
      job("awaiting_approval", { id: "ny" }),
      job("killed", { id: "k" }),
    ];
    expect(overviewListsOf(jobs, null).sections.map((section) => section.id)).toEqual([
      "needs-you",
      "running",
      "queued",
      "recently-ended",
      // Done was left off while the Board still drew it. Every Job that
      // completed or was cleared would have gone with that page.
      "done",
    ]);
  });

  // Overview 28 (#1092)'s own claim: a Job killed to redispatch no longer
  // vanishes, and one actually redispatched still folds away as any
  // lineage does — `foldLineages` did not need to change for this.
  it("draws a killed job under recently-ended until a redispatch folds it away", () => {
    const killed = job("killed", { id: "k", created_at: "2026-09-10T00:00:00Z" });
    const { sections } = overviewListsOf([killed], null);
    expect(sections.find((section) => section.id === "recently-ended")?.jobs.map((one) => one.id)).toEqual([
      "k",
    ]);

    const redispatched = job("running", {
      id: "replacement",
      created_at: "2026-09-11T00:00:00Z",
      redispatched_from: "k",
    });
    const after = overviewListsOf([killed, redispatched], null);
    expect(after.sections.find((section) => section.id === "recently-ended")).toBeUndefined();
    expect(after.sections.find((section) => section.id === "running")?.jobs.map((one) => one.id)).toEqual([
      "replacement",
    ]);
  });

  it("scopes to the pick, and to every repository on All", () => {
    const jobs = [
      job("queued", { id: "a", owner_manifest_id: "armada" }),
      job("queued", { id: "b", owner_manifest_id: "shop" }),
    ];
    expect(overviewListsOf(jobs, null).sections[0]?.jobs.map((one) => one.id)).toEqual(["a", "b"]);
    expect(overviewListsOf(jobs, SHOP).sections[0]?.jobs.map((one) => one.id)).toEqual(["b"]);
  });

  it("draws no section with nothing in it", () => {
    expect(overviewListsOf([job("running")], null).sections.map((section) => section.id)).toEqual(["running"]);
  });

  it("folds a redispatch chain to its live member", () => {
    const jobs = [
      job("killed", { id: "first", created_at: "2026-09-10T00:00:00Z" }),
      job("running", { id: "second", created_at: "2026-09-11T00:00:00Z", redispatched_from: "first" }),
    ];
    const { sections, dispatch } = overviewListsOf(jobs, null);
    expect(sections.find((section) => section.id === "running")?.jobs.map((one) => one.id)).toEqual(["second"]);
    expect(dispatch.get("second")).toMatchObject({ nth: 2, of: 2 });
  });

  it("names a Job the row shape cannot draw, rather than placing it in Other", () => {
    const jobs = [job("not_a_status_the_registry_has", { id: "x" })];
    const { sections, undrawable } = overviewListsOf(jobs, null);
    expect(sections.find((section) => section.id === "other")).toBeUndefined();
    expect(undrawable.map((one) => one.id)).toEqual(["x"]);
  });

  it("sorts oldest first within a section, the Board's own default", () => {
    const jobs = [
      job("running", { id: "newer", created_at: "2026-09-12T00:00:00Z" }),
      job("running", { id: "older", created_at: "2026-09-01T00:00:00Z" }),
    ];
    expect(overviewListsOf(jobs, null).sections[0]?.jobs.map((one) => one.id)).toEqual(["older", "newer"]);
  });
});

describe("nestedOf", () => {
  const at = (id: string, waits_on: string[] = []) => job("queued", { id, waits_on });
  const shape = (jobs: ReturnType<typeof at>[]) =>
    nestedOf(jobs).map((one) => `${one.depth}:${one.job.id}${one.alsoWaits ? "+" : ""}`);

  it("nests a chain one level per step", () => {
    expect(shape([at("c", ["b"]), at("b", ["a"]), at("a")])).toEqual(["0:a", "1:b", "2:c"]);
  });

  it("draws a fan-out's children under their parent, in list order", () => {
    expect(shape([at("a"), at("x", ["a"]), at("y", ["a"])])).toEqual(["0:a", "1:x", "1:y"]);
  });

  it("draws a join once, under its first parent, and marks it", () => {
    expect(shape([at("a"), at("b"), at("j", ["a", "b"])])).toEqual(["0:a", "1:j+", "0:b"]);
  });

  it("leaves a Job whose parent is in another section a marked root", () => {
    expect(shape([at("k", ["elsewhere"])])).toEqual(["0:k+"]);
  });

  it("draws every Job of a cycle once", () => {
    expect(shape([at("a", ["b"]), at("b", ["a"])])).toEqual(["0:a+", "1:b"]);
  });
});
