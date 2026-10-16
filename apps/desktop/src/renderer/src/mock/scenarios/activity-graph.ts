// The flight deck's Activity as a graph: the whole ship at rest, with the last hour filled in so the columns have something to say.
// Events are spread over six Jobs and Sessions in every tone, with one busy stretch about half an hour ago (27 to 30 minutes back,
// the ninth column from the right of twenty three-minute ones) that holds nine events from five of them. Every age is nominal and
// shifts together as the window stays open, so a stretch's neighbours move with it. The walk `activity-graph` plays it.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as base } from "./cockpit-ship";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const held = base.draft?.events ?? [];

type Who = { subject: TimelineSubject; title: string };
/** A Job or Session the ship already has, found by the title its own events carry. */
function who(title: string): Who {
  const one = held.find((event) => event.title === title && (event.subject.kind === "job" || event.subject.kind === "session"));
  if (one === undefined) throw new Error(`activity-graph: nothing on the ship is called ${title}`);
  return { subject: one.subject, title };
}

const cache = who("Cache the manifest read between dispatches");
const reconnect = who("Shorten the reconnect wait");
const clock = who("Pin the store clock");
const notes = who("Migration notes");
const gate = who("Gate policy on every run");
const fix = who("Fix the reconnect wait");

let count = 6000;
function event(minutes: number, one: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, detail?: string, items?: string[]): TimelineEvent {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: one.subject, title: one.title, tone, says, manifest_id: "armada", live: tone === "run", ...(detail === undefined ? {} : { detail }), ...(items === undefined ? {} : { items }) };
}

const MORE: TimelineEvent[] = [
  // The busy stretch: five of them at once.
  event(27.2, clock, "session.edited", "run", "Edited 4 files", "crates/store/src/clock.rs", ["crates/store/src/clock.rs", "crates/store/src/pool.rs", "crates/store/tests/clock.rs", "crates/store/tests/pool.rs"]),
  event(27.5, notes, "session.subagent", "ok", "Subagent finished", "Check the order of migrations 0042 to 0047"),
  event(27.7, reconnect, "job.step", "bad", "store failed", "store::reconnect::waits_five_seconds"),
  event(27.9, gate, "session.ran", "run", "Ran 3 commands", "cargo test -p fleet gate", ["cargo test -p fleet gate", "cargo clippy -p fleet", "git diff --stat main"]),
  event(28, cache, "job.step", "ask", "Awaiting review"),
  event(28.2, clock, "session.read", "run", "Read 6 files", "crates/store/src/pool.rs", ["crates/store/src/pool.rs", "crates/store/src/lib.rs", "crates/store/src/reads.rs", "crates/store/src/writes.rs", "crates/store/Cargo.toml", "Cargo.lock"]),
  event(28.4, fix, "session.turn", "run", "Reading the reconnect tests"),
  event(28.6, notes, "session.read", "run", "Read 3 files", "migrations/0046_drop_legacy.sql", ["migrations/0046_drop_legacy.sql", "migrations/0045_rename_ledger.sql", "docs/migrations.md"]),
  event(28.9, gate, "session.edited", "run", "Edited 2 files", "crates/fleet/src/gate.rs", ["crates/fleet/src/gate.rs", "crates/gate/src/policy.rs"]),

  // The rest of the hour, from the oldest to the newest.
  event(58.4, clock, "session.turn", "run", "Reading the store tests"),
  event(55.1, notes, "session.read", "run", "Read 3 files", "migrations/0042_add_index.sql", ["migrations/0042_add_index.sql", "migrations/0043_split_runs.sql", "docs/migrations.md"]),
  event(55.6, notes, "session.edited", "run", "Edited 1 file", "docs/migrations.md", ["docs/migrations.md"]),
  event(49.3, cache, "job.step", "run", "Fix started"),
  event(46.8, gate, "session.turn", "idle", "Waiting on the store Check"),
  event(43.2, reconnect, "job.step", "ok", "Scope passed"),
  event(43.9, reconnect, "job.step", "run", "Fix started"),
  event(43.7, cache, "job.files", "run", "Changed 2 files", "crates/fleet/src/manifest.rs"),
  event(37.4, fix, "session.edited", "run", "Edited 1 file", "apps/desktop/src/renderer/src/cockpit/reconnect.ts", ["apps/desktop/src/renderer/src/cockpit/reconnect.ts"]),
  event(34.1, clock, "session.ran", "run", "Ran 2 commands", "cargo test -p store", ["cargo test -p store", "cargo clippy -p store"]),
  event(24.6, notes, "session.waiting", "ask", "Waiting: drop the legacy migration?", "Decision"),
  event(21.2, reconnect, "job.step", "bad", "store failed", "store::reconnect::waits_five_seconds"),
  event(21.8, reconnect, "job.step", "run", "Fix restarted"),
  event(15.3, gate, "session.ran", "run", "Ran 1 command", "cargo test -p fleet", ["cargo test -p fleet"]),
  event(15.7, clock, "session.turn", "ok", "Tests pass"),
  event(10.4, cache, "job.step", "ok", "Fix passed"),
  event(10.9, notes, "session.resolved", "ok", "Resolved: drop the legacy migration?", "Decision"),
  event(4.3, fix, "session.turn", "run", "Waiting on the Check"),
  event(1.6, gate, "session.turn", "ask", "Replied"),
];

export const s252ActivityGraph: Scenario = {
  ...base,
  name: "activity-graph",
  says: "The flight deck with the last hour of events across six Jobs and Sessions, for the Activity graph",
  draft: { ...base.draft, events: [...held, ...MORE] },
};
