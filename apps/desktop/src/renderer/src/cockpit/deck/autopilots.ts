// Every repository's Autopilot as this window knows it, read off Fleet: listed once and kept current by
// `autopilot.changed`, which main sends again after a reconnect. Held here and not in a component, so the
// switch on the Map's lip, the setup over the field and the question for whoever is back all read one record.
// Nothing is kept past the last reader: a window mounted again asks Fleet again.

import { useMemo, useSyncExternalStore } from "react";
import type { Autopilot } from "@armada/protocol";

let held: ReadonlyMap<string, Autopilot> = new Map();
let stop: (() => void) | undefined;
const heard = new Set<() => void>();

const set = (next: ReadonlyMap<string, Autopilot>) => {
  held = next;
  heard.forEach((one) => one());
};
/** One repository's Autopilot, whole, as Fleet answered or published it. */
export const fold = (autopilot: Autopilot) => set(new Map(held).set(autopilot.manifest_id, autopilot));

/** A window whose Fleet serves no Autopilot (a fake without the slice) has none to read. */
const served = () => typeof window.armada?.listAutopilots === "function";

function begin(): () => void {
  if (!served()) return () => undefined;
  let current = true;
  const off = window.armada.onAutopilotChanged(fold);
  void window.armada.listAutopilots().then((read) => current && read.ok && read.value.autopilots.forEach(fold));
  return () => {
    current = false;
    off();
  };
}

function subscribe(hear: () => void): () => void {
  heard.add(hear);
  if (heard.size === 1) stop = begin();
  return () => {
    heard.delete(hear);
    if (heard.size > 0) return;
    stop?.();
    stop = undefined;
    held = new Map();
  };
}

const runningIn = (all: ReadonlyMap<string, Autopilot>): Autopilot[] => [...all.values()].filter((one) => one.state === "running");

/** The Autopilots that are running now. Paused and stopped ones are not. */
export function useRunning(): Autopilot[] {
  const all = useSyncExternalStore(subscribe, () => held);
  return useMemo(() => runningIn(all), [all]);
}

/** The Autopilots that have not ended: running, or paused with picks still to settle. */
export function useAlive(): Autopilot[] {
  const all = useSyncExternalStore(subscribe, () => held);
  return useMemo(() => [...all.values()].filter((one) => one.state !== "stopped"), [all]);
}

/** Whether any Autopilot is running, read where a component is not. */
export const isRunning = (): boolean => runningIn(held).length > 0;

/** Stops every running Autopilot: the jobs they started carry on. */
export async function stopRunning(): Promise<void> {
  if (!served()) return;
  await Promise.all(
    runningIn(held).map(async (one) => {
      const stopped = await window.armada.stopAutopilot(one.manifest_id);
      if (stopped.ok) fold(stopped.value);
    }),
  );
}
