// A bay's history as Fleet will serve it: every lease and every branch checkout, newest first. The shape is the query's.
// Nothing serves it yet, so a window with no source draws the history the timeline's slot events give (`historyOfLedger`),
// which has leases and no checkouts; a mock scenario hands its own list to `BayHistoriesFrom`. Wiring the real query is a
// source passed to the same provider. #2301.

import { createContext, useContext, type ReactNode } from "react";

import type { Ledger } from "./bays";

export type BayHolder = { kind: "job" | "session" | "agent"; id: string; title?: string };

export type BayHistoryEntry =
  | { kind: "leased"; at: string; holder: BayHolder; branch?: string }
  | { kind: "released"; at: string; holder: BayHolder }
  | { kind: "checkout"; at: string; from?: string; to: string; sha: string };

/** `entries` are newest first. `cut` is set where Fleet kept more than it sent, so the first entry is not the bay's first. */
export type BayHistory = { manifest_id: string; slot: number; entries: BayHistoryEntry[]; cut: boolean };

/** One holder's time on the bay. A start is absent where the history is cut before it; an end is absent while it holds. */
export type Stay = { holder: BayHolder; from?: number; to?: number; branch?: string };

/** What the history draws: a stay of one holder, or a checkout inside one. */
export type HistoryRow = { kind: "stay"; at: number; stay: Stay } | { kind: "checkout"; at: number; from?: string; to: string; sha: string };

const same = (a: BayHolder, b: BayHolder) => a.kind === b.kind && a.id === b.id;

/**
 * The entries as rows, newest first. A lease and the release that ends it are one row with a span; a lease another holder
 * took over ends there, and a release with no lease before it (the history is cut) is a row with an end only. **A lease with
 * no release is still held.** A checkout is a row of its own where it happened.
 */
export function rowsOf(history: BayHistory): HistoryRow[] {
  const oldest = history.entries.map((entry, index) => ({ entry, index })).reverse();
  const made: { row: HistoryRow; order: number }[] = [];
  let open: { stay: Stay; order: number } | undefined;
  const close = (to: number) => {
    if (open === undefined) return;
    made.push({ row: { kind: "stay", at: open.stay.from ?? to, stay: { ...open.stay, to } }, order: open.order });
    open = undefined;
  };
  for (const { entry, index } of oldest) {
    const at = Date.parse(entry.at);
    if (entry.kind === "leased") {
      if (open !== undefined && same(open.stay.holder, entry.holder)) continue;
      close(at);
      open = { stay: { holder: entry.holder, from: at, ...(entry.branch === undefined ? {} : { branch: entry.branch }) }, order: index };
    } else if (entry.kind === "released") {
      if (open !== undefined && same(open.stay.holder, entry.holder)) close(at);
      else made.push({ row: { kind: "stay", at, stay: { holder: entry.holder, to: at } }, order: index });
    } else {
      made.push({ row: { kind: "checkout", at, ...(entry.from === undefined ? {} : { from: entry.from }), to: entry.to, sha: entry.sha }, order: index });
    }
  }
  if (open !== undefined) made.push({ row: { kind: "stay", at: open.stay.from!, stay: open.stay }, order: open.order });
  return made.sort((a, b) => b.row.at - a.row.at || a.order - b.order).map((one) => one.row);
}

/** The history the timeline's slot events carry: leases and where they ended, no branches and no checkouts. */
export function historyOfLedger(manifestId: string, slot: number, ledger: Ledger): BayHistory {
  const entries: BayHistoryEntry[] = [];
  for (const lease of ledger.leases) {
    if (lease.from === undefined) continue;
    const holder: BayHolder = { kind: lease.holder.kind, id: lease.holder.id, ...(lease.holder.title === undefined ? {} : { title: lease.holder.title }) };
    if (lease.to !== undefined) entries.push({ kind: "released", at: new Date(lease.to).toISOString(), holder });
    entries.push({ kind: "leased", at: new Date(lease.from).toISOString(), holder });
  }
  entries.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
  return { manifest_id: manifestId, slot, entries, cut: entries.length > 0 };
}

/** What a window reads a bay's history from: `undefined` where nothing serves one. */
export type BayHistorySource = (manifestId: string, slot: number) => BayHistory | undefined;

const NONE: BayHistorySource = () => undefined;
const Held = createContext<BayHistorySource>(NONE);

/** Hand the window the histories a source serves. The mock's mount is the one caller until Fleet serves the query. */
export function BayHistoriesFrom({ histories, children }: { histories: readonly BayHistory[] | undefined; children: ReactNode }) {
  const source: BayHistorySource = histories === undefined ? NONE : (manifestId, slot) => histories.find((one) => one.manifest_id === manifestId && one.slot === slot);
  return <Held.Provider value={source}>{children}</Held.Provider>;
}

/** A bay's history as served, or the ledger's where nothing serves one. Nothing where neither has an entry. */
export function useBayHistory(manifestId: string, slot: number, ledger: Ledger): BayHistory | undefined {
  const served = useContext(Held)(manifestId, slot);
  if (served !== undefined) return served;
  const derived = historyOfLedger(manifestId, slot, ledger);
  return derived.entries.length === 0 ? undefined : derived;
}
