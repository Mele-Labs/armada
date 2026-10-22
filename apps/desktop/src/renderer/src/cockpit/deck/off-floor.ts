// What a Job's drones are doing when they are not writing code, read for its Field: the lights along its pad, and what each drone is
// doing now in the words its strip says. **Every state is read from what Fleet recorded**: a check from the thread's `check` rows, a walk
// from its `window` rows, a call from `session.called`, a read or write from the burst entries, and thinking from how long ago the last
// tool call was, and a message to or from another Job from `session.messaged`. Nothing here is drawn from what an agent said. Pure.
// `docs/concepts/cockpit.md`, *A Job open*.

import type { ServiceCall, TimelineEvent } from "@armada/protocol";

import { FRESH_MS } from "./doing";
import { FLAT, type DroneIn, type Layout, type P3, type Size } from "./layout";
import { WINDOW_UP, outpostAt } from "./pixi/scene";

/** How a check's light stands: unlit before it has started in this round, running, passed or failed. */
export type LightState = "idle" | "run" | "ok" | "bad";

/** One light along the pad: the check's name, how it stands, and the file its record names where it failed and names one. */
export type Light = { name: string; state: LightState; blame?: string };

/** A check the agent ran in its slot, as the thread's row records it. */
export type CheckRecord = { name: string; at: string; state: "running" | "passed" | "failed"; target?: string };

/** How long a working turn goes with no tool call before its drone is thinking: longer than the ordinary pause between two edits. */
export const THINK_MS = 12_000;

/** Whether a working turn has gone `ms` with no tool call by `now`. None recorded is no quiet spell. */
export const quietAt = (last: number | undefined, now: number, ms: number = THINK_MS): boolean => last !== undefined && now - last >= ms;

const STATE: Record<CheckRecord["state"], LightState> = { running: "run", passed: "ok", failed: "bad" };

/** A check's target, where it names one file: a path with a directory and an extension. A filter, a crate list or a test name names none. */
export function fileNamed(target: string | undefined): string | undefined {
  if (target === undefined || /[\s,]/.test(target)) return undefined;
  const last = target.split("/").pop() ?? "";
  return target.includes("/") && /\.[A-Za-z0-9]+$/.test(last) ? target : undefined;
}

/**
 * One light for each check the Job has run, in the order it first ran them. A round begins with the first check after the Job's last
 * edit, so a check not run again since is unlit until it is. A light takes the newest run of its check in the round.
 */
export function lightsOf(checks: readonly CheckRecord[], editedAt: number | undefined, fileOf: (target: string) => string | undefined = fileNamed): Light[] {
  const order: string[] = [];
  const newest = new Map<string, CheckRecord>();
  for (const one of checks) {
    if (!order.includes(one.name)) order.push(one.name);
    const at = Date.parse(one.at);
    // A row stamped before the last edit belongs to the round before.
    if (editedAt !== undefined && Number.isFinite(at) && at < editedAt) continue;
    newest.set(one.name, one);
  }
  return order.map((name): Light => {
    const one = newest.get(name);
    if (one === undefined) return { name, state: "idle" };
    const state = STATE[one.state];
    const blame = state === "bad" && one.target !== undefined ? fileOf(one.target) : undefined;
    return blame === undefined ? { name, state } : { name, state, blame };
  });
}

/** What a drone of the Job is doing now. */
export type Now =
  | { kind: "write" | "read"; file: string }
  | { kind: "run" }
  | { kind: "checks" }
  | { kind: "walk"; name: string }
  | { kind: "think" }
  | { kind: "call"; service: string; name: string }
  | { kind: "ask" }
  | { kind: "idle" }
  | { kind: "done" }
  | { kind: "working" };

const when = (event: TimelineEvent): number => Date.parse(event.at);

/** The newest of the Job's entries of these kinds up to `clock`. */
const newestOf = (events: readonly TimelineEvent[], kinds: readonly TimelineEvent["kind"][], clock: number): TimelineEvent | undefined => {
  let out: TimelineEvent | undefined;
  for (const one of events) if (kinds.includes(one.kind) && when(one) <= clock && (out === undefined || when(one) >= when(out))) out = one;
  return out;
};

/** When the Job last called a tool, as far as Fleet recorded it: a tool row, a check row, a burst's first call or a call to a service. */
export function lastToolAt(stamps: readonly string[], events: readonly TimelineEvent[]): number | undefined {
  let out: number | undefined;
  const see = (at: number) => {
    if (Number.isFinite(at) && (out === undefined || at > out)) out = at;
  };
  for (const at of stamps) see(Date.parse(at));
  for (const one of events) if (one.kind === "session.read" || one.kind === "session.edited" || one.kind === "session.ran" || one.kind === "session.called") see(when(one));
  return out;
}

/** The service a working Job is calling now: its newest call, while it is fresh. */
export function callingOf(events: readonly TimelineEvent[], clock: number): ServiceCall | undefined {
  const call = newestOf(events, ["session.called"], clock);
  return call?.call !== undefined && when(call) + FRESH_MS > clock ? call.call : undefined;
}

/** The read, write or command a working Job is at now: its newest burst, while it is fresh. */
export function burstOf(events: readonly TimelineEvent[], clock: number): { act: "read" | "write" | "run"; detail?: string } | undefined {
  const burst = newestOf(events, ["session.read", "session.edited", "session.ran"], clock);
  if (burst === undefined || when(burst) + FRESH_MS <= clock) return undefined;
  const act = burst.kind === "session.read" ? "read" : burst.kind === "session.edited" ? "write" : "run";
  return burst.detail === undefined ? { act } : { act, detail: burst.detail };
}

/**
 * The Job's own drone. A walk Fleet holds open comes first, then, while the turn works, a check running, a quiet spell, a fresh call, a
 * fresh burst and the file Fleet last saw it touch. With the turn over it waits on him where something is owed, and is done or idle.
 */
export function mainNow(input: {
  working: boolean;
  walk: string | undefined;
  checking: boolean;
  quiet: boolean;
  call: ServiceCall | undefined;
  burst: { act: "read" | "write" | "run"; file?: string } | undefined;
  file: string | undefined;
  owed: boolean;
  done: boolean;
}): Now {
  if (input.walk !== undefined) return { kind: "walk", name: input.walk };
  if (input.working) {
    if (input.checking) return { kind: "checks" };
    if (input.quiet) return { kind: "think" };
    if (input.call !== undefined) return { kind: "call", service: input.call.service, name: labelOf(input.call.name) };
    const burst = input.burst;
    if (burst?.act === "run") return { kind: "run" };
    if (burst !== undefined && burst.file !== undefined) return { kind: burst.act, file: burst.file };
    if (input.file !== undefined) return { kind: "write", file: input.file };
    return { kind: "working" };
  }
  if (input.owed) return { kind: "ask" };
  return input.done ? { kind: "done" } : { kind: "idle" };
}

/** A subagent: done once the ledger says so, else writing the file Fleet last saw it touch, else at work. Fleet records no more of one. */
export function subagentNow(done: boolean, file: string | undefined): Now {
  if (done) return { kind: "done" };
  return file === undefined ? { kind: "working" } : { kind: "write", file };
}

/** A service's name as a label, as an outpost's: `the web` is `Web`. */
const labelOf = (name: string): string => {
  const bare = name.replace(/^the /, "");
  return `${bare.charAt(0).toUpperCase()}${bare.slice(1)}`;
};

const base = (path: string): string => path.split("/").pop() || path;

/** What the strip says a drone is doing. */
export function wordOf(now: Now): string {
  switch (now.kind) {
    case "write":
      return `Writing ${base(now.file)}`;
    case "read":
      return `Reading ${base(now.file)}`;
    case "run":
      return "Running a command";
    case "checks":
      return "Running checks";
    case "walk":
      return "Walking the app";
    case "think":
      return "Thinking";
    case "call":
      return `Calling ${now.name}`;
    case "ask":
      return "Waiting on you";
    case "done":
      return "Done";
    case "idle":
      return "Idle";
    case "working":
      return "Working";
  }
}

/** A message between this Job and another while its entry is fresh: the other Job, whether this one sent it, what it said, and when. */
export type Message = { other: string; sent: boolean; quote: string; at: number; title?: string };

/** What a message said, without the kind Fleet puts before it. */
export const quoteOf = (says: string): string => says.replace(/^Messaged: /, "");

/**
 * The other Jobs this one is messaging or is messaged by at `clock`, newest first: one for each other Job, from its newest entry, while
 * that entry is fresh. `title` is the sender's, where the other Job sent it. Fleet writes an entry only for a Job it hosts.
 */
export function messagesOf(events: readonly TimelineEvent[], self: string, clock: number): Message[] {
  const out = new Map<string, Message>();
  const newest = events.filter((one) => one.kind === "session.messaged" && when(one) <= clock && when(one) + FRESH_MS > clock).sort((a, b) => when(b) - when(a) || b.id - a.id);
  for (const one of newest) {
    const from = one.subject.kind === "session" ? one.subject.id : undefined;
    const sent = from === self;
    const other = sent ? one.to : one.to === self ? from : undefined;
    if (other === undefined || other === self || out.has(other)) continue;
    out.set(other, { other, sent, quote: quoteOf(one.says), at: when(one), ...(sent || one.title === undefined ? {} : { title: one.title }) });
  }
  return [...out.values()];
}

/** What the strip says of a Job messaging another, or messaged by one. */
export const messageWord = (message: Pick<Message, "sent">, title: string): string => (message.sent ? `Messaging ${title}` : `Messaged by ${title}`);

/** A link the Field draws between two drones while one Job messages the other: from the sender's drone to the receiver's. */
export type LinkIn = { key: string; from: string; to: string; other: string; title: string; quote: string; sent: boolean };

/**
 * What an own camera frames besides the Job's drones while one is off the floor: on the pad or walking, the pad, its lights' names and the
 * window over it, and the file a failing check's line runs to; calling, the outpost it calls. Nothing while every drone is on the floor.
 */
export function offFloorMarks(i: { layout: Layout; drones: readonly DroneIn[]; lights: readonly Light[] | undefined; outposts: readonly { service: string }[]; world: Size }): P3[] {
  const marks: P3[] = [];
  const pad = i.layout.territories.find((one) => one.pad !== undefined)?.pad;
  const docked = i.drones.filter((one) => one.off === "pad" || one.off === "walk");
  if (pad !== undefined && docked.length > 0) {
    const up = docked.some((one) => one.off === "walk") ? WINDOW_UP + 30 : 30;
    for (const x of [pad.x, pad.x + pad.w]) for (const y of [pad.y, pad.y + pad.h + 16]) marks.push({ x, y: y * FLAT, z: 0 });
    marks.push({ x: pad.x + pad.w / 2, y: (pad.y + pad.h / 2) * FLAT, z: up });
    for (const light of i.lights ?? []) {
      const bar = light.state === "bad" && light.blame !== undefined ? i.layout.bars.get(light.blame) : undefined;
      if (bar !== undefined) marks.push({ x: bar.cx, y: bar.cy * FLAT, z: 20 });
    }
  }
  for (const drone of i.drones) {
    const k = drone.calling === undefined ? -1 : i.outposts.findIndex((one) => one.service === drone.calling);
    if (k < 0) continue;
    const at = outpostAt(i.world, k, i.outposts.length);
    marks.push({ x: at.x - 40, y: at.y * FLAT, z: 0 }, { x: at.x + 40, y: at.y * FLAT, z: 230 });
  }
  return marks;
}
