// Overview over Jobs that wait on each other: a chain, a fan-out and a join, across the sections.
// `waits_on` is the wire's own field (23.14), so nothing here is invented beyond the rows.
//
//   schema ─▶ cache ─┬─▶ review ─┐
//                    ├─▶ docs ───┼─▶ release
//                    └─▶ bench ──┘
//   old-poke (killed) ─▶ poke-retry

import type { JobSummary } from "@armada/protocol";
import { boardWorkflows } from "@armada/screens/src/fixtures/build/board";
import { job } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./moment";
import type { Scenario } from "./moment";

const id = (name: string) => `01M2C1TJ8G00${name.toUpperCase().padEnd(14, "0")}`;

function row(name: string, status: string, title: string, over: Partial<JobSummary> = {}): JobSummary {
  return job(status, {
    id: id(name),
    handle: `${name}-${title.toLowerCase().replace(/[^a-z]+/g, "-").slice(0, 28)}`,
    title,
    current_step_id: "fix",
    ...over,
  });
}

export function dependencyJobs(): JobSummary[] {
  return [
    row("schema", "completed_success", "Migrate the job schema", { current_step_id: "land" }),
    row("cache", "running", "Cache the manifest read", { waits_on: [id("schema")] }),
    row("review", "awaiting_review", "Review the cache invalidation", { waits_on: [id("cache")] }),
    row("docs", "queued", "Document the cache keys", {
      waits_on: [id("cache")],
      branch: undefined,
      assigned_drone: undefined,
    }),
    row("bench", "queued", "Benchmark the cached read", {
      waits_on: [id("cache")],
      branch: undefined,
      assigned_drone: undefined,
    }),
    row("release", "queued", "Cut the release notes", {
      waits_on: [id("docs"), id("bench"), id("review")],
      branch: undefined,
      assigned_drone: undefined,
    }),
    row("oldpoke", "killed", "Retire the legacy poke path", { ended_at: "2026-09-10T20:05:00Z" }),
    row("retry", "queued", "Add a retry ceiling to the poke loop", {
      waits_on: [id("oldpoke")],
      branch: undefined,
      assigned_drone: undefined,
    }),
  ];
}

export const DEPENDENCY_BOARD: Scenario = {
  ...onBoard(dependencyJobs(), { workflows: boardWorkflows() }),
  name: "overview/dependencies",
  says: "Overview over Jobs that wait on each other: a chain, a fan-out and a join",
};
