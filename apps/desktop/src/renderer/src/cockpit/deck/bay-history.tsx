// A bay's history as Fleet serves it: every lease and every branch checkout, newest first, asked of `getSlotHistory` when a
// Bay pane is open and asked again when the timeline brings a slot event for the bay. While the answer is pending or refused
// the window draws the history the timeline's slot events give (`historyOfLedger`), which has leases and no checkouts; a
// mock scenario hands its own list to `BayHistoriesFrom`. #2301.

import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import type { BayHistory, BayHistoryEntry, BayHolder } from "@armada/protocol";

import type { BayHistoryRead } from "../../../../shared/api/bay-history";
import type { Ledger } from "./bays";

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

/** What a window reads a bay's history from, ahead of asking Fleet: `undefined` where nothing is handed one. */
export type BayHistorySource = (manifestId: string, slot: number) => BayHistory | undefined;

const NONE: BayHistorySource = () => undefined;
const Held = createContext<BayHistorySource>(NONE);

/** Hand the window the histories a source serves. The mock's mount is the one caller: it never asks Fleet. */
export function BayHistoriesFrom({ histories, children }: { histories: readonly BayHistory[] | undefined; children: ReactNode }) {
  const source: BayHistorySource = histories === undefined ? NONE : (manifestId, slot) => histories.find((one) => one.manifest_id === manifestId && one.slot === slot);
  return <Held.Provider value={source}>{children}</Held.Provider>;
}

const askFleet = (manifestId: string, slot: number): Promise<BayHistoryRead> => window.armada.getSlotHistory(manifestId, slot);

/**
 * A bay's history as handed to the window, else Fleet's answer, else the ledger's. Fleet is asked when the bay is first drawn
 * and again whenever a lease or release changes the ledger; the last answer stays up while the next is read. Nothing where
 * none of the three has an entry.
 */
export function useBayHistory(manifestId: string, slot: number, ledger: Ledger, read: (manifestId: string, slot: number) => Promise<BayHistoryRead> = askFleet): BayHistory | undefined {
  const handed = useContext(Held)(manifestId, slot);
  const [fetched, setFetched] = useState<BayHistory>();
  const moved = ledger.leases.map((one) => `${one.from ?? ""}-${one.to ?? ""}`).join("|");
  useEffect(() => {
    if (handed !== undefined) return;
    let live = true;
    read(manifestId, slot).then(
      (answer) => {
        if (live && answer.ok) setFetched(answer.value);
      },
      () => {},
    );
    return () => {
      live = false;
    };
    // `moved` stands for the ledger: a slot event for this bay is the cue to ask again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [manifestId, slot, moved, handed === undefined]);
  if (handed !== undefined) return handed;
  if (fetched !== undefined && fetched.manifest_id === manifestId && fetched.slot === slot) return fetched;
  const derived = historyOfLedger(manifestId, slot, ledger);
  return derived.entries.length === 0 ? undefined : derived;
}
