// A bay as the flight deck says it: free, leased, stranded or closed, who is behind it, and who held it before. The state
// and a stranded slot's facts are the pool's own (`WorktreeSlot`, `PoolSlots`' `stateOf`). The history is **only what
// Fleet's `slot.leased` and `slot.released` timeline events say**, who took the slot and when it was given back, with the
// current `WorktreeSlot` and a stranded slot's `last_holder` completing it. Nothing says why a lease ended or which branch
// it was on, so a past row carries neither, and a bay with no slot events has no history. What the events do not reach
// is `unknown`, never `free`. Nothing here draws. #2301.

import { useMemo } from "react";
import { CircleHelp, DoorClosedLocked, DoorOpen, Ghost, KeyRound, LoaderCircle, SquareDashed, type LucideIcon } from "lucide-react";
import type { TimelineEvent, WorktreeSlot } from "@armada/protocol";
import { stateOf as poolStateOf } from "@armada/components/src/compositions/PoolSlots/PoolTile";

/** Who held a bay. A past holder can be a Job Fleet still serves; only Sessions are drawn as holding one now. */
export type Holder = { kind: "session"; id: string; title?: string | undefined };

/** One holder's time on a bay. A start is absent where only the slot's own record names the holder. */
export type Lease = { holder: Holder; from?: number; to?: number };

/** A bay's leases, newest first, and the earliest instant the events or the slot reach back to. */
export type Ledger = { leases: readonly Lease[]; first: number | undefined };

export type BayState = "free" | "leased" | "stranded" | "closed" | "busy" | "ghost" | "unknown";

export type BayView = {
  key: string;
  slot: WorktreeSlot;
  state: BayState;
  word: string;
  /** The state in full, where it says more than its word: why a bay is stranded, what a kept bay keeps. */
  said?: string;
  Glyph: LucideIcon;
  holder?: Holder;
  /** The branch the slot is on now. A past lease does not carry one. */
  branch?: string;
  /** When the holder took it, and for how long it has held it, at the instant the view is of. */
  since?: number;
  heldFor?: number;
  /** A stranded bay: who held it last, what is left on it, and when the holder was told. */
  last?: (Holder & { open: boolean }) | undefined;
  dirty?: number;
  unpushed?: number;
  told?: number;
  /** A free or closed bay: how long ago it was given back, where an event says. */
  freed?: number;
  warm: boolean;
};

const MIN = 60_000;

/** `41m`, `2h 05m`, as the Board says a span. */
export function spanOf(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / MIN));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** An id cut to what a person can tell it by. */
export const shortOf = (id: string): string => (id.length > 10 ? `${id.slice(0, 4)}…${id.slice(-4)}` : id);

export const slotKey = (slot: Pick<WorktreeSlot, "manifest_id" | "slot">) => `slot:${slot.manifest_id}/${slot.slot}`;

/** The holder a bay has now, as the pool says it. */
export function holderOf(slot: WorktreeSlot): Holder | undefined {
  const held = slot.held;
  if (held.state === "hosted_session") return { kind: "session", id: held.session_id, title: held.session_title };
  if (held.state === "session") return { kind: "session", id: held.session_id ?? held.holder, title: held.session_title ?? held.holder };
  return undefined;
}

/** The Session a bay is lit with in the list: its holder, or a stranded bay's last holder, by the key the list's rows carry. */
export function holderKeyOf(view: BayView): string | undefined {
  const holder = view.holder ?? view.last;
  return holder === undefined ? undefined : `${holder.kind}:${holder.id}`;
}

/** The slot each hosted Session holds now, by the subject key the drones carry. A drone's pad, and so its arc, comes from here. */
export function holdersOf(slots: readonly WorktreeSlot[]): Map<string, WorktreeSlot> {
  return new Map(
    slots.flatMap((slot) => {
      const held = slot.held;
      const key = held.state === "hosted_session" ? `session:${held.session_id}` : undefined;
      return key === undefined ? [] : [[key, slot] as const];
    }),
  );
}

const same = (a: Holder, b: Holder) => a.kind === b.kind && a.id === b.id;

/** The Session a slot event is about. Events about a pull request or main are not a holder. */
function holderOfEvent(event: TimelineEvent): Holder | undefined {
  const { subject } = event;
  return subject.kind === "session" ? { kind: subject.kind, id: subject.id, title: event.title } : undefined;
}

/**
 * The bay's leases from the slot events that name it. A lease opens at `slot.leased` and closes at that holder's
 * `slot.released`, or when another holder takes the slot. A release whose lease the events do not show is not a row, but
 * it still fixes when the events begin. One that never closed is kept only for the holder the slot has now, and for a
 * stranded slot's last holder. **No slot events for this bay is no history.**
 */
export function ledgerOf(slot: WorktreeSlot, events: readonly TimelineEvent[]): Ledger {
  const mine = events
    .filter((event) => (event.kind === "slot.leased" || event.kind === "slot.released") && event.detail === `slot-${slot.slot}` && event.manifest_id === slot.manifest_id)
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id);
  if (mine.length === 0) return { leases: [], first: undefined };

  const leases: Lease[] = [];
  let open: Lease | undefined;
  for (const event of mine) {
    const holder = holderOfEvent(event);
    const at = Date.parse(event.at);
    if (holder === undefined) continue;
    if (event.kind === "slot.leased") {
      if (open !== undefined && same(open.holder, holder)) continue;
      // The slot has one holder, so a new one means the last one is over.
      if (open !== undefined) leases.push({ ...open, to: at });
      open = { holder, from: at };
    } else if (open !== undefined && same(open.holder, holder)) {
      leases.push({ ...open, to: at });
      open = undefined;
    }
  }

  const now = holderOf(slot);
  const last = slot.stranded?.last_holder;
  if (open !== undefined && ((now !== undefined && same(open.holder, now)) || (last !== undefined && same(open.holder, last)))) leases.push(open);
  if (now !== undefined && !leases.some((one) => one.to === undefined && same(one.holder, now))) {
    leases.push({ holder: now, ...(slot.since === undefined ? {} : { from: Date.parse(slot.since) }) });
  }
  if (last !== undefined && !leases.some((one) => same(one.holder, last))) leases.push({ holder: { kind: last.kind, id: last.id, title: last.title } });

  const sorted = leases.sort((a, b) => (b.from ?? Infinity) - (a.from ?? Infinity));
  const starts = [Date.parse(mine[0]!.at), ...sorted.flatMap((one) => (one.from === undefined ? [] : [one.from]))];
  return { leases: sorted, first: Math.min(...starts) };
}

/** The lease that covers an instant, where one does. */
export const leaseAt = (ledger: Ledger, at: number): Lease | undefined => ledger.leases.find((one) => one.from !== undefined && one.from <= at && (one.to === undefined || at < one.to));

/**
 * The bay at an instant: now, or the playhead of a replay. At the playhead the events say who held it; **before the
 * first thing they say, the bay is unknown**, not free. Where they say nobody held it, the bay is free, unless it stands
 * stranded or closed now and the last lease has ended by then.
 */
export function bayViewOf(slot: WorktreeSlot, key: string, ledger: Ledger, at: number | undefined, now: number): BayView {
  const live = poolStateOf({ slot } as never);
  const closed = slot.closed === true;
  const instant = at ?? now;
  const base = { key, slot, warm: slot.warm } as const;
  if (at !== undefined) {
    if (ledger.first === undefined || at < ledger.first) return { ...base, state: "unknown", word: "Unknown", Glyph: CircleHelp };
    const lease = leaseAt(ledger, at);
    if (lease !== undefined) return { ...base, state: "leased", word: "Held", Glyph: KeyRound, holder: lease.holder, ...(lease.from === undefined ? {} : { since: lease.from, heldFor: at - lease.from }) };
    const before = ledger.leases.filter((one) => one.to !== undefined && one.to <= at).sort((a, b) => b.to! - a.to!)[0];
    const lastTo = ledger.leases.reduce((m, one) => Math.max(m, one.to ?? Infinity), 0);
    if (at >= lastTo && !closed && live.bay === "stranded") return strandedOf(base, slot, instant);
    const shut = closed && at >= lastTo;
    return { ...base, state: shut ? "closed" : "free", word: shut ? "Closed" : "Free", Glyph: shut ? DoorClosedLocked : DoorOpen, ...(before?.to === undefined ? {} : { freed: instant - before.to }) };
  }
  if (live.bay === "stranded") return strandedOf(base, slot, instant);
  const holder = holderOf(slot);
  const current = ledger.leases.find((one) => one.to === undefined && one.from !== undefined);
  const since = current?.from ?? (slot.since === undefined ? undefined : Date.parse(slot.since));
  if (holder !== undefined && live.bay === "held") {
    return {
      ...base,
      state: "leased",
      word: live.word,
      Glyph: KeyRound,
      said: live.said,
      holder,
      ...(slot.branch === undefined ? {} : { branch: slot.branch }),
      ...(since === undefined ? {} : { since, heldFor: instant - since }),
    };
  }
  const freed = ledger.leases.find((one) => one.to !== undefined)?.to;
  if (closed) return { ...base, state: "closed", word: "Closed", Glyph: DoorClosedLocked, ...(freed === undefined ? {} : { freed: instant - freed }) };
  if (live.bay === "busy") return { ...base, state: "busy", word: live.word, Glyph: LoaderCircle };
  if (live.bay === "ghost") return { ...base, state: "ghost", word: live.word, Glyph: SquareDashed };
  return { ...base, state: "free", word: "Free", Glyph: DoorOpen, ...(freed === undefined ? {} : { freed: instant - freed }) };
}

function strandedOf(base: { key: string; slot: WorktreeSlot; warm: boolean }, slot: WorktreeSlot, instant: number): BayView {
  const left = slot.stranded;
  return {
    ...base,
    state: "stranded",
    word: "Stranded",
    said: slot.held.state === "stranded" ? `Stranded: ${slot.held.why}` : "Stranded",
    Glyph: Ghost,
    ...(slot.branch === undefined ? {} : { branch: slot.branch }),
    last: left?.last_holder === undefined ? undefined : { ...left.last_holder },
    dirty: left?.uncommitted.length ?? 0,
    unpushed: left?.unpushed ?? 0,
    ...(left?.told === undefined ? {} : { told: Date.parse(left.told) }),
    ...(slot.since === undefined ? {} : { since: Date.parse(slot.since), heldFor: instant - Date.parse(slot.since) }),
  };
}

/** Every bay of the pool as a view, with its ledger: now, or as of a replay's playhead. */
export function useBays(slots: readonly WorktreeSlot[], events: readonly TimelineEvent[], at: number | undefined, now: number): { view: BayView; ledger: Ledger }[] {
  return useMemo(
    () =>
      slots.map((slot) => {
        const ledger = ledgerOf(slot, events);
        return { view: bayViewOf(slot, slotKey(slot), ledger, at, now), ledger };
      }),
    [slots, events, at, now],
  );
}

/** How long ago `since` was, in the Board's words: 41m, 2h 05m. */
export function heldFor(since: string | undefined, now: number): string | undefined {
  const at = since === undefined ? NaN : Date.parse(since);
  if (Number.isNaN(at)) return undefined;
  const minutes = Math.max(0, Math.round((now - at) / 60_000));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}
