// What a Job costs the machine, for the System layer. These shapes are what the wire will fill; the store below is the mock's only source
// and is what the wire task replaces. A scenario seeds it; Stop flips a process in it.

import { useSyncExternalStore } from "react";

export type Process = { pid: number; command: string; port?: number; state: "running" | "stopped" };
/** One reading of the machine as this Job uses it: `used` of `of`, both in `unit`. */
export type Reading = { used: number; of: number; unit: string };
export type LogFile = { name: string; bytes: number };
export type JobSystem = { processes: readonly Process[]; cpu?: Reading; memory?: Reading; disk?: Reading; logs: readonly LogFile[] };

const held = new Map<string, JobSystem>();
const listeners = new Set<() => void>();
const none: JobSystem = { processes: [], logs: [] };

const emit = () => listeners.forEach((one) => one());

/** Sets what a Job's system is. */
export function seedSystem(sessionId: string, system: JobSystem): void {
  held.set(sessionId, system);
  emit();
}

/** Stops one process. */
export function stopProcess(sessionId: string, pid: number): void {
  const now = held.get(sessionId) ?? none;
  held.set(sessionId, { ...now, processes: now.processes.map((one) => (one.pid === pid ? { ...one, state: "stopped" } : one)) });
  emit();
}

/** A Job's system, as it stands. */
export function useSystem(sessionId: string): JobSystem {
  return useSyncExternalStore(
    (listen) => {
      listeners.add(listen);
      return () => void listeners.delete(listen);
    },
    () => held.get(sessionId) ?? none,
  );
}

/** A size as a person reads it: 1.4 MB, 820 KB. */
export function sizeOf(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
