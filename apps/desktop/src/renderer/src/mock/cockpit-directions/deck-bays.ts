// A bay as the flight deck says it: free, leased, stranded or closed, who is behind it, and who held it before. The state and the
// stranded slot's facts are the pool's own (`WorktreeSlot`, `PoolSlots`' `stateOf`); the history is what Fleet's slot ledger would
// say (`who_owns`, `armada worktree --status`), made up here from the bay and the Jobs and Sessions the ship holds, and kept the same
// from one draw to the next. Nothing here draws. Mock only. proto/cockpit-flight-deck.

import { useMemo } from "react";
import { DoorClosedLocked, DoorOpen, Ghost, KeyRound, LoaderCircle, SquareDashed, type LucideIcon } from "lucide-react";
import type { WorktreeSlot } from "@armada/protocol";
import { stateOf as poolStateOf } from "@armada/components/src/compositions/PoolSlots/PoolTile";

import { rand } from "./deck-field";
import { slotKey } from "./deck-model";

export type Holder = { kind: "job" | "session"; id: string; title?: string | undefined };

/** How a lease ended: still held, given back clean, given back with its branch kept, stranded, or taken by the owner. */
export type Ending = "held" | "clean" | "kept" | "stranded" | "taken";

export type Lease = { holder: Holder; branch: string; from: number; to?: number; ended: Ending };

export type BayState = "free" | "leased" | "stranded" | "closed" | "busy" | "ghost";

export type BayView = {
  key: string;
  slot: WorktreeSlot;
  state: BayState;
  word: string;
  /** The state in full, where it says more than its word: why a bay is stranded, what a kept bay keeps. */
  said?: string;
  Glyph: LucideIcon;
  holder?: Holder;
  /** Why a Job's lease is kept where it has ended. */
  kept?: string;
  branch?: string;
  /** When the holder took it, and for how long it has held it, at the instant the view is of. */
  since?: number;
  heldFor?: number;
  /** A stranded bay: who held it last, what is left on it, and when the holder was told. */
  last?: (Holder & { open: boolean }) | undefined;
  dirty?: number;
  unpushed?: number;
  told?: number;
  /** A free or closed bay: when it was given back. */
  freed?: number;
  warm: boolean;
};

/** The time a ledger is made against: the draw it was first asked for in. */
const T0 = Date.now();
const MIN = 60_000;

/** `41m`, `2h 05m`, as the Board says a span. */
export function spanOf(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / MIN));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** A time of day, `10:02`, in local time. */
export const clockOf = (at: number): string => new Date(at).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });

/** An id cut to what a person can tell it by. */
export const shortOf = (id: string): string => (id.length > 10 ? `${id.slice(0, 4)}…${id.slice(-4)}` : id);

const slug = (title: string) => title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 28);

/** The holder a bay has now, as the pool says it. */
export function holderOf(slot: WorktreeSlot): Holder | undefined {
  const held = slot.held;
  if (held.state === "job") return { kind: "job", id: held.job_id, title: held.job_title };
  if (held.state === "hosted_session") return { kind: "session", id: held.session_id, title: held.session_title };
  if (held.state === "session") return { kind: "session", id: held.session_id ?? held.holder, title: held.session_title ?? held.holder };
  return undefined;
}

const ledgers = new Map<string, Lease[]>();

/**
 * The bay's recent holders, newest first: the one holding it now (or the one it stranded under), then a few before that. Some are
 * the Jobs and Sessions the ship holds, and the rest are gone, known by their ids alone. One of the bays' histories has a stranding in it.
 */
export function ledgerOf(slot: WorktreeSlot, key: string, field: readonly Holder[]): Lease[] {
  const now = holderOf(slot);
  const id = `${key}:${now?.id ?? ""}:${slot.since ?? ""}`;
  const kept = ledgers.get(id);
  if (kept !== undefined) return kept;
  const out: Lease[] = [];
  let edge: number;
  const closed = slot.closed === true;
  if (now !== undefined) {
    const from = slot.since === undefined ? T0 - 20 * MIN : Date.parse(slot.since);
    out.push({ holder: now, branch: slot.branch ?? `${slug(now.title ?? now.id)}`, from, ended: "held" });
    edge = from;
  } else if (slot.stranded?.last_holder !== undefined) {
    const last = slot.stranded.last_holder;
    const to = T0 - Math.round(40 + rand(key, 11) * 50) * MIN;
    const from = to - Math.round(50 + rand(key, 12) * 80) * MIN;
    out.push({ holder: { kind: last.kind, id: last.id, title: last.title }, branch: slot.branch ?? slug(last.title ?? last.id), from, to, ended: "stranded" });
    edge = from;
  } else {
    edge = T0 - Math.round(25 + rand(key, 13) * 170) * MIN;
  }
  const given = now === undefined && out.length === 0;
  const total = 3 + Math.floor(rand(key, 14) * 4);
  const strands = rand(key, 15) > 0.55 || key.endsWith("/3");
  const used = new Set(out.map((one) => one.holder.id));
  for (let i = 0; out.length < total; i += 1) {
    const to = given && i === 0 ? edge : edge - Math.round(4 + rand(key, 20 + i) * 30) * MIN;
    const from = to - Math.round(20 + rand(key, 40 + i) * 120) * MIN;
    const pool = field.filter((one) => !used.has(one.id));
    // Half the holders are ones the ship still holds; the rest are gone.
    const real = pool.length > 0 && rand(key, 60 + i) > 0.45 ? pool[Math.floor(rand(key, 80 + i) * pool.length)] : undefined;
    const gone: Holder = { kind: rand(key, 100 + i) > 0.5 ? "job" : "session", id: `01M2C1${Math.floor(rand(key, 120 + i) * 1e9).toString(36).toUpperCase().padStart(6, "0")}${Math.floor(rand(key, 140 + i) * 1e9).toString(36).toUpperCase().padStart(6, "0")}` };
    const holder = real ?? gone;
    used.add(holder.id);
    const roll = rand(key, 160 + i);
    const ended: Ending = closed && given && i === 0 ? "taken" : strands && i === 1 ? "stranded" : roll > 0.9 ? "taken" : roll > 0.72 ? "kept" : "clean";
    out.push({ holder, branch: `${holder.kind === "job" ? "armada" : "session"}/${slug(holder.title ?? holder.id.slice(-8).toLowerCase())}`, from, to, ended });
    edge = from;
  }
  ledgers.set(id, out);
  return out;
}

/** The lease that covers an instant, where one does. */
export const leaseAt = (ledger: readonly Lease[], at: number): Lease | undefined => ledger.find((one) => one.from <= at && (one.to === undefined || at < one.to));

/**
 * The bay at an instant: now, or the playhead of a replay. At the playhead the ledger says who held it; where it says nobody did, the bay is
 * free, unless it stands stranded or closed now and the last lease has ended by then.
 */
export function bayViewOf(slot: WorktreeSlot, key: string, ledger: readonly Lease[], at: number | undefined, now: number): BayView {
  const live = poolStateOf({ slot } as never);
  const closed = slot.closed === true;
  const instant = at ?? now;
  const base = { key, slot, warm: slot.warm } as const;
  if (at !== undefined) {
    const lease = leaseAt(ledger, at);
    if (lease !== undefined) return { ...base, state: "leased", word: "Held", Glyph: KeyRound, holder: lease.holder, branch: lease.branch, since: lease.from, heldFor: at - lease.from };
    const before = ledger.filter((one) => one.to !== undefined && one.to <= at).sort((a, b) => b.to! - a.to!)[0];
    const lastTo = ledger.reduce((m, one) => Math.max(m, one.to ?? Infinity), 0);
    if (at >= lastTo && !closed && live.bay === "stranded") return strandedOf(base, slot, instant);
    return { ...base, state: closed && at >= lastTo ? "closed" : "free", word: closed && at >= lastTo ? "Closed" : "Free", Glyph: closed && at >= lastTo ? DoorClosedLocked : DoorOpen, ...(before?.to === undefined ? {} : { freed: instant - before.to }) };
  }
  if (live.bay === "stranded") return strandedOf(base, slot, instant);
  const holder = holderOf(slot);
  const lease = ledger.find((one) => one.ended === "held");
  const since = lease?.from ?? (slot.since === undefined ? undefined : Date.parse(slot.since));
  if (holder !== undefined && (live.bay === "held" || live.bay === "kept")) {
    return {
      ...base,
      state: "leased",
      word: live.word,
      Glyph: live.bay === "kept" ? live.Glyph : KeyRound,
      said: live.said,
      holder,
      ...(slot.held.state === "job" && slot.held.kept !== undefined ? { kept: slot.held.kept } : {}),
      ...(slot.branch === undefined ? {} : { branch: slot.branch }),
      ...(since === undefined ? {} : { since, heldFor: instant - since }),
    };
  }
  const freed = ledger.find((one) => one.to !== undefined)?.to;
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

/** How a lease ended, in words. */
export const ENDING: Record<Ending, string> = {
  held: "Held now",
  clean: "Given back clean",
  kept: "Given back, its branch kept",
  stranded: "Stranded",
  taken: "Taken by the owner",
};

/** Every bay of the pool as a view, with its ledger: now, or as of a replay's playhead. `field` is the Jobs and Sessions the ship holds. */
export function useBays(slots: readonly WorktreeSlot[], rootOf: (manifest: string) => string | undefined, subjects: readonly { kind: string; id: string; title: string; root: string }[], at: number | undefined, now: number): { view: BayView; ledger: Lease[] }[] {
  return useMemo(
    () =>
      slots.map((slot) => {
        const key = slotKey(slot);
        const root = rootOf(slot.manifest_id) ?? slot.manifest_id;
        const field = subjects.filter((one) => one.kind !== "pr" && one.root === root).map((one): Holder => ({ kind: one.kind === "job" ? "job" : "session", id: one.id, title: one.title }));
        const ledger = ledgerOf(slot, key, field);
        return { view: bayViewOf(slot, key, ledger, at, now), ledger };
      }),
    [slots, subjects, at, now],
  );
}
