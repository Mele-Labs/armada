// The flight deck's Activity beeswarm: the whole ship at rest, with the last hour filled in. Nine Jobs and Sessions work through it in
// stretches, some loud (turns and steps) and some quiet (reads, edits and commands), a single event lands in every minute so any
// point of the hour has something near it, and the ship's own events add what needs him and what failed. Ages are fixed when the page
// loads and the chart slides on from there, so the walks point at the newest event and not at a named one. The walk `activity-beeswarm` plays it.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as base } from "./cockpit-ship";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const held = base.draft?.events ?? [];

type Who = { subject: TimelineSubject; title: string; kind: "job" | "session" };
/** A Job or Session the ship already has, found by the title its own events carry. */
function who(title: string): Who {
  const one = held.find((event) => event.title === title && (event.subject.kind === "job" || event.subject.kind === "session"));
  if (one === undefined || (one.subject.kind !== "job" && one.subject.kind !== "session")) throw new Error(`activity-graph: nothing on the ship is called ${title}`);
  return { subject: one.subject, title, kind: one.subject.kind };
}

const clock = who("Pin the store clock");
const notes = who("Migration notes");
const gate = who("Gate policy on every run");
const fix = who("Fix the reconnect wait");
const cacheKey = who("Cache key");
const audit = who("Service worker audit");
const cache = who("Cache the manifest read between dispatches");
const reconnect = who("Shorten the reconnect wait");
const pause = who("Add the pause marker column");

// The same hour every time: a small generator, seeded.
let seed = 7;
const random = () => {
  seed = (seed * 48271) % 2147483647;
  return seed / 2147483647;
};

let count = 6000;
function entry(minutes: number, one: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, detail?: string): TimelineEvent {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: one.subject, title: one.title, tone, says, manifest_id: "armada", live: tone === "run", ...(detail === undefined ? {} : { detail }) };
}

const TURNS = ["Reading the tests", "Editing the module", "Writing the test", "Checking the diff", "Reading the notes"];
const STEPS = ["Fix started", "Changed 2 files", "Changed 1 file", "Fix restarted"];
const FILES = ["crates/store/src/clock.rs", "crates/fleet/src/gate.rs", "migrations/0046_drop_legacy.sql", "apps/desktop/src/renderer/src/cockpit/reconnect.ts"];
const COMMANDS = ["cargo test -p store", "cargo clippy -p fleet", "git diff --stat main"];

/** `n` events across the minutes from `from` to `to` ago, `quiet` of them as a share the coalesced reads, edits and commands. */
function stretch(one: Who, from: number, to: number, n: number, quiet: number): TimelineEvent[] {
  return Array.from({ length: n }, (_, i) => {
    const minutes = from + (to - from) * ((i + random()) / n);
    const pick = <T>(list: readonly T[]) => list[Math.floor(random() * list.length)]!;
    if (i < Math.round(n * quiet)) {
      const act = Math.floor(random() * 3);
      const kind = one.kind === "session" ? (["session.read", "session.edited", "session.ran"] as const)[act]! : (["job.read", "job.ran", "job.ran"] as const)[act]!;
      const says = act === 0 ? "Read 3 files" : act === 1 && one.kind === "session" ? "Edited 2 files" : "Ran 1 command";
      return entry(minutes, one, kind, "run", says, act === 2 ? pick(COMMANDS) : pick(FILES));
    }
    return one.kind === "session" ? entry(minutes, one, "session.turn", "run", pick(TURNS)) : entry(minutes, one, "job.step", "run", pick(STEPS));
  });
}

const MORE: TimelineEvent[] = [
  ...stretch(clock, 0, 9, 16, 0.5),
  ...stretch(clock, 18, 30, 7, 0.8),
  ...stretch(clock, 40, 58, 4, 0.5),
  ...stretch(notes, 2, 14, 11, 0.7),
  ...stretch(notes, 26, 44, 18, 0.4),
  ...stretch(gate, 0, 6, 6, 0.2),
  ...stretch(gate, 12, 24, 14, 0.8),
  ...stretch(gate, 48, 58, 3, 0.3),
  ...stretch(fix, 0, 4, 3, 0.3),
  ...stretch(fix, 30, 40, 9, 0.6),
  ...stretch(cache, 3, 16, 8, 0.4),
  ...stretch(cache, 34, 52, 10, 0.7),
  ...stretch(reconnect, 0, 3, 4, 0.5),
  ...stretch(reconnect, 16, 28, 13, 0.3),
  ...stretch(pause, 20, 50, 7, 0.5),
  ...stretch(cacheKey, 0, 8, 7, 0.6),
  ...stretch(cacheKey, 28, 36, 8, 0.8),
  ...stretch(audit, 44, 58, 5, 0.4),
  // What is owed him, and what failed.
  entry(24.6, notes, "session.waiting", "ask", "Waiting: drop the legacy migration?", "Decision"),
  entry(21.2, reconnect, "job.step", "bad", "store failed", "store::reconnect::waits_five_seconds"),
];

const EVERY = [clock, notes, gate, fix, cacheKey, audit, cache, reconnect, pause];
// One event in each minute of the hour, so a point of the chart is never empty.
for (let minute = 0; minute < 60; minute += 1) MORE.push(...stretch(EVERY[minute % EVERY.length]!, minute, minute + 1, 1, minute % 3 === 0 ? 1 : 0));

export const s252ActivityBeeswarm: Scenario = {
  ...base,
  name: "activity-beeswarm",
  says: "The flight deck with the last hour of events across nine Jobs and Sessions, for the Activity beeswarm",
  draft: { ...base.draft, events: [...held, ...MORE] },
};
