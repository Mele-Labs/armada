// The Job and its Drones, held for every Job that has begun and not ended.
//
// `job-focus.ts` holds these for the one Job a person has open. The Cockpit and
// the Dashboard draw a Now for every active Job without opening any of them, so
// main keeps a pair of readers per active Job and publishes what they last read.
// Routes are the open Job's own — `get_job` and `list_job_drones` — read per Job
// rather than through a batch route; a Job is read when it begins and again on
// the events that already re-read the open one, so the cost is the Jobs that
// move, not the ones that are listed.
//
// **Only while a surface is following**, `reports.ts`'s rule: nothing is read
// for a window that draws no Now, and following ending drops what was held.

import { JOB_LIFECYCLE } from "@armada/components/src/generated/vocabulary";
import type { Active, JobDetail, JobDrones, JobSummary } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { JobReader } from "./reader";

/** The two statuses a Job passes before anything runs: the approval draws its own canvas. */
const BEFORE_THE_RUN = new Set(["awaiting_approval", "proposing"]);

/** Whether a Job has begun and not ended. */
export function isActive(job: JobSummary): boolean {
  return JOB_LIFECYCLE[job.status]?.terminal === false && !BEFORE_THE_RUN.has(job.status);
}

export type ActiveJobsWiring = {
  publish: (change: Partial<BridgeState>) => void;
};

type Pair = {
  detail: JobReader<{ detail: JobDetail }>;
  drones: JobReader<{ drones: JobDrones }>;
};

export class ActiveJobs {
  private readonly wiring: ActiveJobsWiring;
  private readonly pairs = new Map<string, Pair>();
  private following = false;
  /** What was last read, by Job. Replaced rather than mutated, so a published value never moves. */
  private kept: Active = {};

  constructor(wiring: ActiveJobsWiring) {
    this.wiring = wiring;
  }

  /** The ids held, for a test to read. */
  held(): string[] {
    return [...this.pairs.keys()];
  }

  /** Start or stop holding a pair per active Job. Stopping drops them all. */
  follow(want: boolean, port: number | null, jobs: readonly JobSummary[]): void {
    this.following = want;
    this.sync(port, jobs);
  }

  /**
   * Hold a pair for each active Job on the Board, and drop the ones that ended
   * or are gone. A Job already held is left alone: the events re-read it.
   */
  sync(port: number | null, jobs: readonly JobSummary[]): void {
    const wanted = new Set(this.following ? jobs.filter(isActive).map((job) => job.id) : []);
    let dropped = false;
    for (const [jobId, pair] of this.pairs) {
      if (wanted.has(jobId)) continue;
      pair.detail.close();
      pair.drones.close();
      this.pairs.delete(jobId);
      dropped = this.forget(jobId) || dropped;
    }
    if (dropped) this.wiring.publish({ active: this.kept });
    for (const jobId of wanted) {
      if (this.pairs.has(jobId)) continue;
      const pair = this.pairOf(jobId);
      this.pairs.set(jobId, pair);
      void pair.detail.want(port, jobId);
      void pair.drones.want(port, jobId);
    }
  }

  /** Read again what an event named, or everything where it named none. */
  async again(port: number, only: string | null): Promise<void> {
    const reads: Promise<void>[] = [];
    for (const [jobId, pair] of this.pairs) {
      if (only !== null && jobId !== only) continue;
      reads.push(pair.detail.again(port), pair.drones.again(port));
    }
    await Promise.all(reads);
  }

  close(): void {
    for (const pair of this.pairs.values()) {
      pair.detail.close();
      pair.drones.close();
    }
    this.pairs.clear();
  }

  private pairOf(jobId: string): Pair {
    return {
      // A failure keeps the last good answer, and a first failure keeps
      // nothing: a Job with no reading yet is absent from `active`, not blank.
      detail: new JobReader<{ detail: JobDetail }>({
        route: (id) => `/jobs/${encodeURIComponent(id)}`,
        keeps: (body) => ({ detail: body as JobDetail }),
        keepsLastGood: true,
        publish: (read) => {
          if (read.state === "read") this.keep(jobId, { detail: read.detail });
        },
      }),
      drones: new JobReader<{ drones: JobDrones }>({
        route: (id) => `/jobs/${encodeURIComponent(id)}/drones`,
        keeps: (body) => ({ drones: body as JobDrones }),
        keepsLastGood: true,
        publish: (read) => {
          if (read.state === "read") this.keep(jobId, { drones: read.drones });
        },
      }),
    };
  }

  private keep(jobId: string, read: Active[string]): void {
    // A read that lands after its Job was dropped has no pair to belong to.
    if (!this.pairs.has(jobId)) return;
    this.kept = { ...this.kept, [jobId]: { ...this.kept[jobId], ...read } };
    this.wiring.publish({ active: this.kept });
  }

  private forget(jobId: string): boolean {
    if (!(jobId in this.kept)) return false;
    const { [jobId]: _gone, ...rest } = this.kept;
    this.kept = rest;
    return true;
  }
}
