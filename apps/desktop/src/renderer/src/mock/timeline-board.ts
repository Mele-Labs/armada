// Overview over a day's worth of Jobs: 150 of them, in chains, fanned out and long-running, for the
// timeline view. Times are laid out from the minute the mock loads, so the right edge is now and
// the live Jobs are still going. A fixed sequence, not random: the same board every load.
//
//   Migrate the job store ─┬─▶ ... (long-running, dispatches across the day)
//   chains of 5-9, each Job dispatched by the one before while it ran
//   fan-outs of 5-8 from a Job that stays running while they go
//
// `dispatched_by` is the wire's own field; nothing here is invented beyond the rows.

import type { JobSummary } from "@armada/protocol";
import { boardWorkflows } from "@armada/screens/src/fixtures/build/board";
import { job } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./moment";
import type { Scenario } from "./moment";

const MIN = 60_000;
const HOURS = 10;
const TOTAL = 150;

const VERBS = ["Migrate", "Cache", "Index", "Retire", "Document", "Benchmark", "Split", "Rename", "Backfill", "Refuse", "Trace", "Batch"];
const NOUNS = [
  "the job store", "the manifest read", "the poke loop", "the settings reducer", "the drone count", "the merge line",
  "the retro writer", "the slot pool", "the event stream", "the check log", "the plan trail", "the lease table",
  "the approval gate", "the diff sheet", "the evidence bundle", "the notification queue", "the runtime file",
];

const id = (n: number) => `01M2C1TJ8G00TL${String(n).padStart(10, "0")}`;
const iso = (t: number) => new Date(t).toISOString();

export function timelineJobs(now = Date.now()): JobSummary[] {
  const end = Math.floor(now / MIN) * MIN;
  const begin = end - HOURS * 60 * MIN;
  let seed = 7;
  const rand = () => {
    // mulberry32: exact in 32-bit arithmetic, where a plain multiply loses its low bits.
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const between = (a: number, b: number) => a + Math.floor(rand() * (b - a + 1));

  const jobs: JobSummary[] = [];
  const titles = new Set<string>();
  const nameOf = () => {
    for (let n = 0; ; n += 1) {
      const title = `${VERBS[between(0, VERBS.length - 1)]} ${NOUNS[between(0, NOUNS.length - 1)]}${n > 8 ? ` ${n}` : ""}`;
      if (!titles.has(title)) {
        titles.add(title);
        return title;
      }
    }
  };
  // `created` is when it was minted (dispatched), `ran` how long it ran after starting, in minutes.
  // Past the end of the board it is still going.
  const add = (created: number, wait: number, ran: number, by?: JobSummary, title = nameOf()): JobSummary => {
    const started = created + wait * MIN;
    const over = started + ran * MIN;
    const live = over >= end;
    const status = live
      ? started >= end
        ? "queued"
        : (["running", "running", "running", "awaiting_review"] as const)[between(0, 3)]!
      : (["completed_success", "completed_success", "completed_success", "completed_success", "completed_failed", "killed"] as const)[between(0, 5)]!;
    const n = jobs.length;
    const row = job(status, {
      id: id(n),
      handle: `${n + 1}-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-").slice(0, 28)}`,
      title,
      current_step_id: "fix",
      created_at: iso(created),
      started_at: started >= end ? undefined : iso(started),
      ended_at: live ? undefined : iso(over),
      ...(by === undefined ? {} : { dispatched_by: by.id, origin: "sub_dispatched" }),
      ...(status === "queued" ? { branch: undefined, assigned_drone: undefined } : {}),
    });
    jobs.push(row);
    return row;
  };
  const endOf = (one: JobSummary) => (one.ended_at === undefined ? end : Date.parse(one.ended_at));
  const startOf = (one: JobSummary) => Date.parse(one.started_at ?? one.created_at);

  // Long-running: running since the start of the board, dispatching all day. The first is the walk's.
  const anchors = ["Migrate the job store", "Move the poke loop onto the new queue", "Rework the review sheet", "Tidy the lease table"].map((title, at) =>
    add(begin + at * 25 * MIN, 1, HOURS * 60, undefined, title),
  );
  for (const [at, anchor] of anchors.entries()) {
    const count = at === 0 ? 8 : 4;
    for (let k = 0; k < count; k += 1) {
      const when = begin + Math.floor(((k + 1.5) / (count + 2)) * (end - begin));
      add(when, between(0, 4), between(20, 70), anchor);
    }
  }
  // Chains: each Job dispatched by the one before while that one was still running.
  for (let chain = 0; jobs.length < 100; chain += 1) {
    let last = add(begin + between(10, 8 * 60) * MIN, between(0, 6), between(25, 70));
    const length = between(5, 9);
    for (let k = 1; k < length && jobs.length < 100 && endOf(last) < end; k += 1) {
      const at = startOf(last) + Math.floor(rand() * (endOf(last) - startOf(last)) * 0.8);
      last = add(at, between(0, 6), between(15, 60), last);
    }
  }
  // Fan-outs: one Job, still going, dispatching 5-8 across its run.
  for (let fan = 0; jobs.length < 138; fan += 1) {
    const start = begin + between(30, 6 * 60) * MIN;
    const parent = add(start, 1, between(150, 240));
    for (let k = 0; k < between(5, 8) && jobs.length < 138; k += 1) {
      add(startOf(parent) + Math.floor(((k + 1) / 9) * (endOf(parent) - startOf(parent))), between(0, 8), between(10, 45), parent);
    }
  }
  // Alone: nothing dispatched them and they dispatched nothing.
  while (jobs.length < TOTAL) add(begin + between(0, HOURS * 60 - 5) * MIN, between(0, 10), between(10, 90));
  return jobs;
}

export const TIMELINE_BOARD: Scenario = {
  ...onBoard(timelineJobs(), { workflows: boardWorkflows() }),
  name: "overview/timeline",
  says: "Overview over 150 Jobs across ten hours: chains, fan-outs and long-running ones, on a timeline",
};
