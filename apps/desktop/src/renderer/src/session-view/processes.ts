// The processes a Job started, for the Processes section of the Artifacts layer. `Process` is the shape the wire will fill; the store below is the
// mock's only source and is what the wire task replaces. A scenario seeds it; Stop flips a row in it.

import { useSyncExternalStore } from "react";

export type Process = { pid: number; command: string; port?: number; state: "running" | "stopped" };

const held = new Map<string, readonly Process[]>();
const listeners = new Set<() => void>();
const none: readonly Process[] = [];

const emit = () => listeners.forEach((one) => one());

/** Sets what a Job's processes are. */
export function seedProcesses(sessionId: string, processes: readonly Process[]): void {
  held.set(sessionId, processes);
  emit();
}

/** Stops one process. */
export function stopProcess(sessionId: string, pid: number): void {
  held.set(sessionId, (held.get(sessionId) ?? none).map((one) => (one.pid === pid ? { ...one, state: "stopped" } : one)));
  emit();
}

/** A Job's processes, as they stand. */
export function useProcesses(sessionId: string): readonly Process[] {
  return useSyncExternalStore(
    (listen) => {
      listeners.add(listen);
      return () => void listeners.delete(listen);
    },
    () => held.get(sessionId) ?? none,
  );
}
