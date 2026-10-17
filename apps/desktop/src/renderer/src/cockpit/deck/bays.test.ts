// A bay's history is what Fleet's slot events say, completed by the slot as it stands now. It never says why a lease ended
// or which branch it was on, a bay with no events has no history, and a bay before its first event is unknown, not free.

import type { TimelineEvent, WorktreeSlot } from "@armada/protocol";
import { expect, test } from "vitest";

import { bayViewOf, holderOf, holdersOf, ledgerOf, slotKey } from "./bays";

const at = (minute: number) => Date.parse("2026-10-14T10:00:00Z") + minute * 60_000;
const NOW = at(120);

let count = 0;
const event = (kind: "slot.leased" | "slot.released", holder: { kind: "session"; id: string }, minute: number, slot = 3, extra: Partial<TimelineEvent> = {}): TimelineEvent => {
  count += 1;
  return { id: count, at: new Date(at(minute)).toISOString(), kind, subject: holder, tone: "idle", says: kind, detail: `slot-${slot}`, manifest_id: "m1", ...extra };
};
const s1 = { kind: "session", id: "s1" } as const;
const s3 = { kind: "session", id: "s3" } as const;

const slot = (extra: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: "m1", slot: 3, path: "/p/slot-3", held: { state: "free" }, base: "main", warm: true, ...extra });
const heldBy = (id: string, since?: string): Partial<WorktreeSlot> => ({ held: { state: "hosted_session", session_id: id, session_title: id }, branch: "feature/now", ...(since === undefined ? {} : { since }) });

test("the history is the leases the slot events name, newest first, with no end reason and no branch", () => {
  const events = [event("slot.leased", s1, 10), event("slot.released", s1, 40), event("slot.leased", s3, 45), event("slot.released", s3, 70)];
  const { leases } = ledgerOf(slot(), events);

  expect(leases.map((one) => [one.holder.kind, one.holder.id, one.from, one.to])).toEqual([
    ["session", "s3", at(45), at(70)],
    ["session", "s1", at(10), at(40)],
  ]);
  for (const lease of leases) expect(Object.keys(lease).sort()).toEqual(["from", "holder", "to"]);
});

test("a bay with no slot events has no history, whoever holds it and whatever the events say of other bays", () => {
  expect(ledgerOf(slot(), []).leases).toEqual([]);
  expect(ledgerOf(slot(heldBy("s1", new Date(at(5)).toISOString())), []).leases).toEqual([]);
  // Another slot's events, another repository's, and events that are not about a slot.
  const others = [event("slot.leased", s1, 10, 4), event("slot.leased", s1, 11, 3, { manifest_id: "m2" }), event("slot.leased", s1, 12, 3, { kind: "session.started" })];
  expect(ledgerOf(slot(), others).leases).toEqual([]);
});

test("the slot completes the history: the holder it has now, from its own record, and a stranded slot's last holder", () => {
  const held = ledgerOf(slot(heldBy("s2", new Date(at(80)).toISOString())), [event("slot.leased", s1, 10), event("slot.released", s1, 40)]);
  expect(held.leases.map((one) => [one.holder.id, one.from, one.to])).toEqual([
    ["s2", at(80), undefined],
    ["s1", at(10), at(40)],
  ]);

  const stranded = ledgerOf(
    slot({ held: { state: "stranded", why: "uncommitted work" }, stranded: { uncommitted: ["a.rs"], commits: [], unpushed: 0, last_holder: { kind: "session", id: "s9", title: "Nine", open: false } } }),
    [event("slot.leased", s1, 10), event("slot.released", s1, 40)],
  );
  // Its last holder has no time the events know, so it is a row without one, first as the newest.
  expect(stranded.leases[0]).toEqual({ holder: { kind: "session", id: "s9", title: "Nine" } });
  expect(stranded.leases).toHaveLength(2);
});

test("a lease the events opened and never closed is kept only for the holder the slot has now", () => {
  const open = [event("slot.leased", s1, 10)];
  expect(ledgerOf(slot(heldBy("s1")), open).leases.map((one) => [one.holder.id, one.from, one.to])).toEqual([["s1", at(10), undefined]]);
  // The slot is free now and no release was recorded: when it ended is not known, so the row is not made up.
  expect(ledgerOf(slot(), open).leases).toEqual([]);
  // A new holder means the last one is over.
  expect(ledgerOf(slot(heldBy("s3")), [event("slot.leased", s1, 10), event("slot.leased", s3, 30)]).leases.map((one) => [one.holder.id, one.to])).toEqual([
    ["s3", undefined],
    ["s1", at(30)],
  ]);
});

test("before its first event a bay is unknown, not free; between leases it is free; during one it is held", () => {
  const events = [event("slot.leased", s1, 10), event("slot.released", s1, 40)];
  const ledger = ledgerOf(slot(), events);
  const view = (minute: number) => bayViewOf(slot(), slotKey(slot()), ledger, at(minute), NOW);

  expect(view(5).state).toBe("unknown");
  expect(view(5).holder).toBeUndefined();
  expect(view(20).state).toBe("leased");
  expect(view(20).holder?.id).toBe("s1");
  expect(view(60).state).toBe("free");
  // A bay no event ever named is unknown at every instant before now.
  expect(bayViewOf(slot(), slotKey(slot()), ledgerOf(slot(), []), at(60), NOW).state).toBe("unknown");
});

test("now is the pool's own reading, with the branch the slot is on and no ledger needed", () => {
  const live = slot(heldBy("s1", new Date(at(30)).toISOString()));
  const view = bayViewOf(live, slotKey(live), ledgerOf(live, []), undefined, NOW);

  expect(view.state).toBe("leased");
  expect(view.holder).toEqual(holderOf(live));
  expect(view.branch).toBe("feature/now");
  expect(view.heldFor).toBe(NOW - at(30));
  expect(bayViewOf(slot(), "k", ledgerOf(slot(), []), undefined, NOW).state).toBe("free");
});

test("a slot a Session takes after the deck is drawn gets its drone, and a released slot loses it", () => {
  const drawn = [slot({ slot: 1, held: { state: "busy" } }), slot({ slot: 4, held: { state: "free" } })];
  // A slot being taken or released draws no holder: only hosted Sessions hold a bay.
  expect([...holdersOf(drawn).keys()]).toEqual([]);

  // Slot 4 as `list_worktrees` returns it once the Session's first write has leased it.
  const leased = drawn.map((one) => (one.slot === 4 ? slot({ slot: 4, ...heldBy("d02bc3e3-94ce-4972-896c-9df4056ecf28", "2026-10-15T12:05:12Z") }) : one));
  const holders = holdersOf(leased);
  expect(holders.get("session:d02bc3e3-94ce-4972-896c-9df4056ecf28")).toBe(leased[1]);
  expect(slotKey(holders.get("session:d02bc3e3-94ce-4972-896c-9df4056ecf28")!)).toBe("slot:m1/4");

  const released = leased.map((one) => (one.slot === 4 ? slot({ slot: 4, held: { state: "free" } }) : one));
  expect(holdersOf(released).has("session:d02bc3e3-94ce-4972-896c-9df4056ecf28")).toBe(false);
});
