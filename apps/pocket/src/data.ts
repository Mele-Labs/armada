// The one module the screens read their Jobs from: the Phone Gateway's lists, kept live
// from /api/live.

import { useEffect, useSyncExternalStore } from "react";

import { Refused, getJson, call } from "./client";
import type { JobsBody, LiveChange, NeedsBody, PhoneJob } from "./gateway";
import { events } from "./sse";


/** What the screens draw of a Job. Ages are text, counted at `now`. */
export type PocketJob = {
  id: string;
  title: string;
  status: string;
  repository: string;
  /** The escalation trigger's name, e.g. `stalled`. */
  reason?: string;
  asking: boolean;
  step?: { at: number; of: number; name: string };
  age: string;
  quiet?: string;
  verdict?: { says: "pass" | "veto"; line: string };
  checks?: { name: string; passed: boolean }[];
  pr?: { number: number; url: string };
};

export function ago(from: string, now: number): string {
  const minutes = Math.max(0, Math.floor((now - Date.parse(from)) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours < 24) return rest === 0 ? `${hours}h` : `${hours}h ${String(rest).padStart(2, "0")}m`;
  const days = Math.floor(hours / 24);
  return hours % 24 === 0 ? `${days}d` : `${days}d ${hours % 24}h`;
}

const PULL = /\/pull\/(\d+)/;

export function toPocket(job: PhoneJob, now: number): PocketJob {
  const over = job.ended_at !== undefined;
  const pull = job.pull_request === undefined ? undefined : PULL.exec(job.pull_request);
  return {
    id: job.id,
    title: job.title,
    status: job.status,
    repository: job.repository ?? "",
    reason: job.reason,
    asking: job.asking,
    step: job.step,
    age: ago(over ? (job.ended_at ?? job.created_at) : job.created_at, now),
    quiet: job.waiting_since === undefined ? undefined : ago(job.waiting_since, now),
    verdict: job.verdict === undefined ? undefined : { says: job.verdict, line: job.verdict === "pass" ? "Pass" : "Veto" },
    checks: job.checks,
    pr: job.pull_request === undefined || pull === null || pull === undefined
      ? undefined
      : { number: Number(pull[1]), url: job.pull_request.replace(/^https?:\/\//, "") },
  };
}

type Lists = { needs: PhoneJob[]; running: PhoneJob[]; done: PhoneJob[]; details: Record<string, PhoneJob>; error?: string; at: number };

let lists: Lists = { needs: [], running: [], done: [], details: {}, at: Date.now() };
const watchers = new Set<() => void>();
const set = (next: Partial<Lists>) => {
  lists = { ...lists, ...next };
  watchers.forEach((watch) => watch());
};
const watching = (watch: () => void) => {
  watchers.add(watch);
  return () => watchers.delete(watch);
};

const message = (why: unknown) => (why instanceof Refused ? why.message : "Armada is not answering on your Mac. Open it there, then try again.");

export async function loadLists(): Promise<void> {
  try {
    const [needs, running, done] = await Promise.all([
      getJson<NeedsBody>("/api/needs"),
      getJson<JobsBody>("/api/jobs?state=running"),
      getJson<JobsBody>("/api/jobs?state=done"),
    ]);
    set({ needs: needs.needs, running: running.jobs, done: done.jobs, error: undefined, at: Date.now() });
  } catch (why) {
    if (!(why instanceof Refused && why.status === 401)) set({ error: message(why) });
  }
}

export async function loadJob(id: string): Promise<void> {
  try {
    const job = await getJson<PhoneJob>(`/api/jobs/${encodeURIComponent(id)}`);
    set({ details: { ...lists.details, [id]: job } });
  } catch (why) {
    if (!(why instanceof Refused && why.status === 401)) set({ error: message(why) });
  }
}

let live = false;
let reload: ReturnType<typeof setTimeout> | undefined;
const changed = (job?: string) => {
  clearTimeout(reload);
  reload = setTimeout(() => {
    void loadLists();
    if (job !== undefined && job in lists.details) void loadJob(job);
  }, 250);
};

/** Holds /api/live open for as long as the app runs, reading the lists again on every change. */
export async function startLive(): Promise<void> {
  if (live) return;
  live = true;
  void loadLists();
  for (;;) {
    try {
      const response = await call("/api/live");
      if (response.body === null) throw new Refused(0, "");
      void loadLists();
      for await (const text of events(response.body)) {
        const change = JSON.parse(text) as LiveChange;
        if (change.change === "resync") void loadLists();
        else changed(change.job_id);
      }
    } catch (why) {
      if (why instanceof Refused && why.status === 401) break;
      set({ error: message(why) });
    }
    await new Promise((resolve) => setTimeout(resolve, 3000));
  }
  live = false;
}

function useLists(): Lists {
  return useSyncExternalStore(watching, () => lists);
}

/** The ages tick once a minute. */
const useNow = (): number => {
  useEffect(() => {
    const tick = setInterval(() => set({ at: Date.now() }), 60_000);
    return () => clearInterval(tick);
  }, []);
  return useLists().at;
};

export function useJobs(): { needs: PocketJob[]; running: PocketJob[]; done: PocketJob[]; error?: string } {
  const now = useNow();
  const current = useLists();
  const map = (jobs: PhoneJob[]) => jobs.map((job) => toPocket(job, now));
  return { needs: map(current.needs), running: map(current.running), done: map(current.done), error: current.error };
}

export function useJob(id: string | undefined): PocketJob | undefined {
  const now = useNow();
  const current = useLists();
  useEffect(() => {
    if (id !== undefined) void loadJob(id);
  }, [id]);
  if (id === undefined) return undefined;
  const found = current.details[id] ?? [...current.needs, ...current.running, ...current.done].find((job) => job.id === id);
  return found === undefined ? undefined : toPocket(found, now);
}
