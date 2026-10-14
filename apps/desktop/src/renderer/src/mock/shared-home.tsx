// The shared home, as the mock holds it: a scenario's rows, settled by the acts on them. Fleet serves
// none of it yet, so a scenario carries its rows (`draft.sharedHome`) and this keeps them. A row that
// differs is only ever settled by a side the person picks; Sync pulls and settles nothing by itself.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { SharedHomeProvider } from "@armada/components";
import type { SharedHome, SharedRow } from "@armada/components";

import type { MockDraft } from "./moment";

const SYNCING_MS = 1400;

function Held({ home, children }: { home: NonNullable<MockDraft["sharedHome"]>; children: ReactNode }) {
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
      rows: [...rows],
      syncing,
      choose: (row, side) => {
        const kept = side === "here" ? row.here : row.there;
        settle(row, { state: "same", ...(kept === undefined ? {} : { here: kept, there: kept }) });
      },
      add: (row) => settle(row, { state: "same" }),
      bring: (row) => settle(row, { state: "same" }),
      sync: () => setSyncing(true),
    };
  }, [home, rows, syncing]);

  return <SharedHomeProvider value={value}>{children}</SharedHomeProvider>;
}

/** Hands the window the home a scenario carries. Nothing carried, nothing is drawn. */
export function SharedHomeFrom({ home, children }: { home: MockDraft["sharedHome"]; children: ReactNode }) {
  return home === undefined ? <>{children}</> : <Held home={home}>{children}</Held>;
}
