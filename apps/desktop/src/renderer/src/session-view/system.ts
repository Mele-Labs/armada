// What a Job costs the machine, for the System layer: read off Fleet's `get_session_system` while the layer is
// open, and a process stopped with `stop_process`. The wire's bytes become the readings the layer draws.

import { useEffect, useState } from "react";

import type { SessionSystem } from "@armada/protocol";

export type Process = { pid: number; command: string; port?: number; state: "running" | "stopped" };
/** One reading of the machine as this Job uses it: `used` of `of`, both in `unit`. */
export type Reading = { used: number; of: number; unit: string };
export type LogFile = { name: string; bytes: number };
export type JobSystem = { processes: readonly Process[]; cpu?: Reading; memory?: Reading; disk?: Reading; logs: readonly LogFile[] };

const none: JobSystem = { processes: [], logs: [] };
/** How often an open layer reads again. */
const POLL_MS = 5000;
const GB = 1024 ** 3;

// A process Fleet has been told to stop stays in the list, as stopped, though the next reading no longer has it.
const stopped = new Map<string, Map<number, Process>>();
const nudges = new Set<() => void>();

/** The layer's readings from Fleet's, with the processes stopped here kept as stopped. */
export function systemOf(wire: SessionSystem | undefined, gone: ReadonlyMap<number, Process> = new Map()): JobSystem {
  if (wire === undefined) return none;
  const running = wire.processes.map((one): Process => {
    const ended = gone.get(one.pid);
    return {
      pid: one.pid,
      command: one.command,
      ...(one.port === undefined ? {} : { port: one.port }),
      state: ended === undefined ? one.state : "stopped",
    };
  });
  const left = [...gone.values()].filter((one) => !wire.processes.some((now) => now.pid === one.pid));
  const gb = (bytes: number) => bytes / GB;
  const cpu = wire.cpu_percent === undefined ? undefined : { used: Math.round(wire.cpu_percent), of: Math.max(100, Math.ceil(wire.cpu_percent / 100) * 100), unit: "%" };
  const memory = wire.memory_bytes === undefined || wire.memory_of_bytes === undefined ? undefined : { used: gb(wire.memory_bytes), of: Math.round(gb(wire.memory_of_bytes)), unit: "GB" };
  const disk = wire.disk_bytes === undefined || wire.disk_of_bytes === undefined ? undefined : { used: gb(wire.disk_bytes), of: Math.round(gb(wire.disk_of_bytes)), unit: "GB" };
  return {
    processes: [...running, ...left],
    ...(cpu === undefined ? {} : { cpu }),
    ...(memory === undefined ? {} : { memory }),
    ...(disk === undefined ? {} : { disk }),
    logs: wire.logs.map((one) => ({ name: one.name, bytes: one.bytes })),
  };
}

/** Stops one process of a Job. Fleet ends it only if the Job started it; the list is read again either way. */
export function stopProcess(sessionId: string, pid: number): void {
  const row = held.get(sessionId)?.processes.find((one) => one.pid === pid);
  void window.armada.stopProcess(sessionId, { pid }).then((done) => {
    if (done.ok) {
      const kept = stopped.get(sessionId) ?? new Map<number, Process>();
      kept.set(pid, { pid, command: row?.command ?? done.value.command, ...(row?.port === undefined ? {} : { port: row.port }), state: "stopped" });
      stopped.set(sessionId, kept);
    }
    nudges.forEach((one) => one());
  });
}

const held = new Map<string, JobSystem>();

/** A Job's system, read when the layer opens and every few seconds while it stays open. */
export function useSystem(sessionId: string): JobSystem {
  const [wire, setWire] = useState<SessionSystem | undefined>(undefined);
  const [, redraw] = useState(0);
  useEffect(() => {
    let open = true;
    const read = () =>
      void window.armada.getSessionSystem(sessionId).then((answer) => {
        if (open && answer.ok) setWire(answer.value);
      });
    const again = () => {
      redraw((n) => n + 1);
      read();
    };
    read();
    const timer = setInterval(read, POLL_MS);
    nudges.add(again);
    return () => {
      open = false;
      clearInterval(timer);
      nudges.delete(again);
    };
  }, [sessionId]);
  const now = systemOf(wire, stopped.get(sessionId));
  held.set(sessionId, now);
  return now;
}

/** A size as a person reads it: 1.4 MB, 820 KB. */
export function sizeOf(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
