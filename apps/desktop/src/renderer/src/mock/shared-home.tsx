// The shared home, as the mock holds it: a scenario's rows, settled by the acts on them. Fleet serves
// none of it yet, so a scenario carries its rows (`draft.sharedHome`) and this keeps them. A row that
// differs is only ever settled by a side the person picks; Sync pulls and settles nothing by itself. Where the home's
// login chooses in What syncs, a kind it has not turned on has no rows, so Kit draws no marks for it.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { SharedHomeProvider } from "@armada/components";
import type { SharedHome, SharedRow } from "@armada/components";

import { useSignedIn, useSyncedKinds } from "./account";
import type { MockDraft } from "./moment";
import { installBroughtMod } from "./system-mods";

const SYNCING_MS = 1400;

function Held({ home, children }: { home: NonNullable<MockDraft["sharedHome"]>; children: ReactNode }) {
  // Against an account that has not signed in yet there is no home to draw.
  const signedIn = useSignedIn();
  const synced = useSyncedKinds(home.account);
  const [rows, setRows] = useState<readonly SharedRow[]>(home.rows);
  const [syncing, setSyncing] = useState(false);
  useEffect(() => {
    if (!syncing) return undefined;
    const timer = window.setTimeout(() => setSyncing(false), SYNCING_MS);
    return () => window.clearTimeout(timer);
  }, [syncing]);

  const value = useMemo<SharedHome>(() => {
    const settle = (row: SharedRow, change: Partial<SharedRow>) =>
      setRows((was) => was.map((one) => (one.kind === row.kind && one.name === row.name ? { ...one, ...change } : one)));
    return {
      account: home.account,
      repository: home.repository,
      rows: synced === undefined ? [...rows] : rows.filter((row) => (synced as readonly string[]).includes(row.kind)),
      syncing,
      choose: (row, side) => {
        const kept = side === "here" ? row.here : row.there;
        settle(row, { state: "same", ...(kept === undefined ? {} : { here: kept, there: kept }) });
      },
      add: (row) => settle(row, { state: "same" }),
      bring: (row) => {
        settle(row, { state: "same" });
        const as = row.modKind ?? "theme";
        if (row.kind === "mods" && (as === "theme" || as === "layout")) installBroughtMod(as, row.name);
      },
      sync: () => setSyncing(true),
    };
  }, [home, rows, syncing, synced]);

  return <SharedHomeProvider value={signedIn ? value : undefined}>{children}</SharedHomeProvider>;
}

/** Hands the window the home a scenario carries. Nothing carried, nothing is drawn. */
export function SharedHomeFrom({ home, children }: { home: MockDraft["sharedHome"]; children: ReactNode }) {
  return home === undefined ? <>{children}</> : <Held home={home}>{children}</Held>;
}
