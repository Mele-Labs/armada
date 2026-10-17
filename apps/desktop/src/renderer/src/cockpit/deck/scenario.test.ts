// The re-authored ship scenario is in the wire shapes Fleet serves, and the deck's readings take it as they take Fleet: the
// scenario holds nothing the readings would have to be told, so whatever the walk draws, the real Cockpit reads the same way.

import type { TimelineEvent, WorktreeSlot } from "@armada/protocol";
import { expect, test } from "vitest";

import { s252CockpitShipFlightdeck as ship } from "../../mock/scenarios/cockpit-ship";
import { mockAreas } from "../../mock/cockpit-areas";
import { bayViewOf, ledgerOf, slotKey } from "./bays";
import { clashesOf, touchesFromEvents } from "./touches";

const events = ship.draft?.events ?? [];
const slots: readonly WorktreeSlot[] = ship.held?.slots ?? [];
const slotAt = (manifest: string, number: number) => slots.find((one) => one.manifest_id === manifest && one.slot === number)!;

/** What the Cockpit will do: a drone's checkouts are the paths of the bays it holds. */
const holders = new Map<string, string[]>();
for (const slot of slots) {
  const holder = slot.held.state === "hosted_session" ? `session:${slot.held.session_id}` : undefined;
  if (holder !== undefined) holders.set(holder, [...(holders.get(holder) ?? []), slot.path]);
}
const keyOf = (event: TimelineEvent) => (event.subject.kind === "session" ? `session:${event.subject.id}` : undefined);

test("every tool burst carries its items, newest first", () => {
  const bursts = events.filter((one) => ["session.read", "session.edited", "session.ran"].includes(one.kind));
  expect(bursts.length).toBeGreaterThan(0);
  for (const one of bursts) {
    expect(one.items?.length, one.kind).toBeGreaterThan(0);
    expect(one.detail).toBe(one.items![0]);
    expect(one.items!.length).toBeLessThanOrEqual(25);
  }
});

test("what a drone touches comes from its edits, in the spellings Fleet gives them, and a drone that edited nothing touches nothing", () => {
  const touched = touchesFromEvents(events, { keyOf, rootsOf: (key) => holders.get(key) ?? [] });
  const areas = (key: string) => touched.get(key)?.map((one) => one.path);

  // Under its bay, absolute: the store tests.
  expect(areas("session:r1")).toEqual(["crates/store"]);
  // From the home directory with `~`: three files in two areas.
  expect(areas("session:p1")).toEqual(["crates/gate", "crates/fleet"]);
  // A Session's paths that were already relative: two files in two areas.
  expect(areas("session:f1")).toEqual(["apps/desktop", "crates/fleet"]);
  // Pocket's Session wrote under its own bay, in another repository.
  expect(areas("session:k1")).toEqual(["src/sw"]);
  // Migration notes only read.
  expect(touched.has("session:w1")).toBe(false);
});

test("the write clashes are the shape /clashes returns and name areas the deck draws", () => {
  const found = clashesOf(ship.draft?.clashes ?? [], {
    keyOf: (kind, id) => (kind === "main" ? undefined : `${kind}:${id}`),
    rootOf: () => "/Users/user/armada",
  });

  expect(found.map((one) => one.path).sort()).toEqual(["apps/desktop", "crates/store"]);
  const names = mockAreas("/Users/user/armada").flatMap((area) => area.kids.map(([kid]) => `${area.dir}/${kid}`));
  for (const one of found) expect(names).toContain(one.path);
});

test("the bays' history is the slot events: two rows where a holder was given back, none for a bay no event names", () => {
  const now = Date.now();
  const history = (manifest: string, number: number) => ledgerOf(slotAt(manifest, number), events).leases.map((one) => [one.holder.id, one.to === undefined ? "held" : "given back"]);

  const armada = slots[0]!.manifest_id;
  expect(history(armada, 1)).toHaveLength(2);
  expect(history(armada, 1)[1]).toEqual(["01SESSIONRELEASENOTES0001", "given back"]);
  // The asking Session holds bay 2, leased and not given back.
  expect(history(armada, 2)).toEqual([["q1", "held"]]);
  // Free, and closed: no event, no history, and before now they are unknown rather than free.
  expect(history(armada, 5)).toEqual([]);
  const free = slotAt(armada, 5);
  expect(bayViewOf(free, slotKey(free), ledgerOf(free, events), now - 60 * 60_000, now).state).toBe("unknown");
  // The stranded bay shows the holder it stranded under.
  expect(history(armada, 6)).toEqual([["01SESSIONCLOSEDFRAME0000", "held"]]);
  const stranded = slotAt(armada, 6);
  expect(bayViewOf(stranded, slotKey(stranded), ledgerOf(stranded, events), undefined, now).state).toBe("stranded");
});
