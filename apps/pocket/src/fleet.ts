// Every paired Mac's lists, kept live from one `/api/live` stream per Mac. The screens read the
// combined view through `useFleet` and `homeOf`; a Mac that answers 401 is marked not paired and
// the others carry on.

import { useEffect, useSyncExternalStore } from "react";

import { Refused, call, getJson, refusal, whenUnpaired } from "./client";
import type { LiveChange, PhoneSession, SessionsBody } from "./gateway";
import { keepMachine, machines, type Machine } from "./machines";
import { events } from "./sse";

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

export type Mac = {
  machine: Machine;
  /** False once the Mac answered 401; `why` is its sentence. */
  paired: boolean;
  why?: string;
  /** The Mac did not answer, or refused a read: its sentence. */
  error?: string;
  needs: PhoneSession[];
  live: PhoneSession[];
};

type Fleet = { macs: Mac[]; loaded: boolean; at: number };

let fleet: Fleet = { macs: [], loaded: false, at: Date.now() };
const watchers = new Set<() => void>();
const emit = () => watchers.forEach((watch) => watch());
const watching = (watch: () => void) => {
  watchers.add(watch);
  return () => watchers.delete(watch);
};

function setMac(origin: string, change: (mac: Mac) => Partial<Mac>) {
  fleet = { ...fleet, macs: fleet.macs.map((mac) => (mac.machine.origin === origin ? { ...mac, ...change(mac) } : mac)) };
  emit();
}

export const macOf = (origin: string): Mac | undefined => fleet.macs.find((mac) => mac.machine.origin === origin);

/** A change a Mac's stream reported, for an open Job to read itself again. `session_id` absent is a resync. */
const heard = new Set<(origin: string, sessionId?: string) => void>();
export function onChange(listen: (origin: string, sessionId?: string) => void): () => void {
  heard.add(listen);
  return () => heard.delete(listen);
}
const tell = (origin: string, sessionId?: string) => heard.forEach((listen) => listen(origin, sessionId));

export async function loadLists(origin: string): Promise<void> {
  const mac = macOf(origin);
  if (mac === undefined) return;
  const machine = mac.machine;
  try {
    const { sessions: live } = await getJson<SessionsBody>(machine, "/api/sessions");
    setMac(origin, () => ({ paired: true, why: undefined, error: undefined, needs: live.filter((session) => session.waiting), live }));
  } catch (why) {
    if (!(why instanceof Refused && why.status === 401)) setMac(origin, () => ({ error: refusal(why, machine) }));
  }
}

/** Learns the Mac's own name where it was paired before it had one to say. */
async function name(machine: Machine): Promise<void> {
  if (machine.name !== "") return;
  try {
    const { name: said } = await getJson<{ name: string }>(machine, "/api/machine");
    const named = { ...machine, name: said };
    await keepMachine(named);
    setMac(machine.origin, () => ({ machine: named }));
  } catch {
    // Unnamed, the Mac's rows go without its name until it answers.
  }
}

const streaming = new Set<string>();
/** The open connection of each streamed Mac, to abort when the app comes back on screen. */
const open = new Map<string, AbortController>();

/** A stream that yields no byte, not even a `: beat` comment, for this long is dead. */
const SILENT_MS = 40_000;
/** A stream that lasted less than this before it closed waits a second before the next. */
const BRIEF_MS = 5_000;

/**
 * Holds a Mac's /api/live open for as long as it is paired, reading its lists again on every change and
 * on every connect. A stream that opened and then ended, broke or was aborted reconnects at once, or a
 * second later if it lasted under 5 s; only a connect that failed is the Mac not answering, and waits 3 s.
 */
async function stream(origin: string): Promise<void> {
  if (streaming.has(origin)) return;
  streaming.add(origin);
  for (;;) {
    const mac = macOf(origin);
    if (mac === undefined || !mac.paired) break;
    const connection = new AbortController();
    open.set(origin, connection);
    let reload: ReturnType<typeof setTimeout> | undefined;
    let silence: ReturnType<typeof setTimeout> | undefined;
    const hear = () => {
      clearTimeout(silence);
      silence = setTimeout(() => connection.abort(), SILENT_MS);
    };
    let connected = false;
    let since = 0;
    try {
      const response = await call(mac.machine, "/api/live", { signal: connection.signal });
      if (response.body === null) throw new Refused(0, "");
      connected = true;
      since = Date.now();
      hear();
      void loadLists(origin);
      for await (const text of events(response.body, hear)) {
        const change = JSON.parse(text) as LiveChange;
        clearTimeout(reload);
        reload = setTimeout(() => void loadLists(origin), change.change === "resync" ? 0 : 250);
        tell(origin, change.change === "resync" ? undefined : change.session_id);
      }
    } catch (why) {
      if (why instanceof Refused && why.status === 401) break;
      if (!connected && !connection.signal.aborted) {
        setMac(origin, (one) => ({ error: refusal(why, one.machine) }));
        await new Promise((resolve) => setTimeout(resolve, 3000));
      }
    } finally {
      clearTimeout(silence);
      clearTimeout(reload);
      if (open.get(origin) === connection) open.delete(origin);
    }
    // A stream that closed on its own moments after opening would otherwise be opened again in a tight loop.
    if (connected && !connection.signal.aborted && Date.now() - since < BRIEF_MS) await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  streaming.delete(origin);
}

/** Every open stream is dropped and opened again, which reads its Mac's lists again. */
const reconnectAll = () => open.forEach((connection) => connection.abort());

function watch(machine: Machine) {
  void name(machine);
  void stream(machine.origin);
}

let started: Promise<void> | undefined;

/** Reads the paired Macs and starts each one's lists. Once per app. */
export function start(): Promise<void> {
  started ??= (async () => {
    whenUnpaired((origin, why) => setMac(origin, () => ({ paired: false, why })));
    document.addEventListener("visibilitychange", () => document.visibilityState === "visible" && reconnectAll());
    window.addEventListener("online", reconnectAll);
    const held = await machines();
    fleet = { ...fleet, loaded: true, macs: held.map((machine) => ({ machine, paired: true, needs: [], live: [] })) };
    emit();
    held.forEach(watch);
  })();
  return started;
}

/** A Mac just paired, or paired again: kept, and its lists started. */
export async function addMac(machine: Machine): Promise<void> {
  await keepMachine(machine);
  const was = macOf(machine.origin);
  if (was === undefined) fleet = { ...fleet, macs: [...fleet.macs, { machine, paired: true, needs: [], live: [] }] };
  else fleet = { ...fleet, macs: fleet.macs.map((mac) => (mac === was ? { ...mac, machine, paired: true, why: undefined, error: undefined } : mac)) };
  emit();
  watch(machine);
}

export function useFleet(): Fleet {
  // The ages tick once a minute.
  useEffect(() => {
    const tick = setInterval(() => {
      fleet = { ...fleet, at: Date.now() };
      emit();
    }, 60_000);
    return () => clearInterval(tick);
  }, []);
  return useSyncExternalStore(watching, () => fleet);
}

export type Item = { mac: Mac; session: PhoneSession };

export type Home = {
  /** More than one Mac is paired, so each row names its Mac. */
  several: boolean;
  needs: Item[];
  running: Item[];
  /** Macs that need the owner: not paired any more, or not answering. */
  troubled: Mac[];
};

const time = (at: string | undefined) => (at === undefined ? 0 : Date.parse(at));

export function homeOf({ macs }: Pick<Fleet, "macs">): Home {
  const paired = macs.filter((mac) => mac.paired);
  const all = (pick: (mac: Mac) => PhoneSession[]) => paired.flatMap((mac) => pick(mac).map((session) => ({ mac, session })));
  return {
    several: macs.length > 1,
    needs: all((mac) => mac.needs).sort((a, b) => time(a.session.waiting_since ?? a.session.started_at) - time(b.session.waiting_since ?? b.session.started_at)),
    running: all((mac) => mac.live.filter((session) => !session.waiting)).sort((a, b) => time(b.session.started_at) - time(a.session.started_at)),
    troubled: macs.filter((mac) => !mac.paired || mac.error !== undefined),
  };
}

/** The Mac that holds a Job a push named without saying where: the one listing it, else the first that can read it. */
export async function holderOf(sessionId: string): Promise<Mac | undefined> {
  await start();
  const listing = fleet.macs.find((mac) => [...mac.needs, ...mac.live].some((one) => one.id === sessionId));
  if (listing !== undefined) return listing;
  for (const mac of fleet.macs.filter((one) => one.paired)) {
    try {
      await call(mac.machine, `/api/sessions/one?session_id=${encodeURIComponent(sessionId)}`);
      return mac;
    } catch {
      // Not this Mac.
    }
  }
  return undefined;
}
