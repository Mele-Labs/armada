// The shared home, as the mock holds it: a scenario's rows, settled by the acts on them. Fleet serves
// none of it yet, so a scenario carries its rows (`draft.sharedHome`) and this keeps them. A row that
// differs is only ever settled by a side the person picks; Sync adds every row only on this machine, bar the
// ones the scenario says stay, each told in a toast of its own, and settles nothing that differs. Where the home's
// login chooses in What syncs, a kind it has not turned on has no rows, so Kit draws no marks for it. A scenario that
// carries `syncs` switches kinds and single items from Kit instead, and a row is drawn where its kind or it is on.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { SharedHomeProvider, useAccount } from "@armada/components";
import type { SharedHome, SharedRow, Syncs } from "@armada/components";

import { NOTHING_SYNCS } from "../fleet-shared-home";
import { askToTell } from "../tell";
import { useSignedIn, useSyncedKinds } from "./account";
import type { MockDraft } from "./moment";
import { installBroughtMod } from "./system-mods";

const SYNCING_MS = 1400;

function Held({ home, children }: { home: NonNullable<MockDraft["sharedHome"]>; children: ReactNode }) {
  // Against an account that has not signed in yet there is no home to draw.
  const signedIn = useSignedIn();
  const synced = useSyncedKinds(home.account);
  const account = useAccount();
  const profile = account?.profiles.find((one) => one.login === home.account);
  const [rows, setRows] = useState<readonly SharedRow[]>(home.rows);
  const [syncing, setSyncing] = useState(false);
  const [syncs, setSyncs] = useState<Syncs | undefined>(home.syncs);
  // A row Kit draws and Sync takes: its kind or it switched on in Kit, else its kind on in What syncs.
  const takes = useCallback(
    (row: SharedRow) =>
      syncs !== undefined
        ? syncs.kinds.includes(row.kind) || syncs.items.some((one) => one.kind === row.kind && one.name === row.name)
        : synced === undefined || (synced as readonly string[]).includes(row.kind),
    [syncs, synced],
  );
  useEffect(() => {
    if (!syncing) return undefined;
    const timer = window.setTimeout(() => {
      const stays = (row: SharedRow) => home.staysHere?.find((one) => one.kind === row.kind && one.name === row.name);
      // A kind switched off in What syncs is not synced.
      const goes = (row: SharedRow) => row.state === "here" && takes(row);
      const left = rows.flatMap((row) => (goes(row) ? (stays(row) ?? []) : []));
      setRows((was) => was.map((row) => (goes(row) && stays(row) === undefined ? { ...row, state: "same" } : row)));
      setSyncing(false);
      left.forEach((one) => askToTell(one.why));
      if (syncs !== undefined && syncs.kinds.length === 0 && syncs.items.length === 0) askToTell(NOTHING_SYNCS);
    }, SYNCING_MS);
    return () => window.clearTimeout(timer);
  }, [syncing, rows, home, syncs, takes]);

  const value = useMemo<SharedHome>(() => {
    const settle = (row: SharedRow, change: Partial<SharedRow>) =>
      setRows((was) => was.map((one) => (one.kind === row.kind && one.name === row.name ? { ...one, ...change } : one)));
    return {
      account: home.account,
      repository: home.repository,
      rows: rows.filter(takes),
      syncing,
      ...(syncs === undefined
        ? {}
        : {
            syncs,
            setKind: (kind: string, on: boolean) =>
              setSyncs((was) => was && { ...was, kinds: on ? [...was.kinds, kind] : was.kinds.filter((one) => one !== kind) }),
            setItem: (item: { kind: string; name: string }, on: boolean) =>
              setSyncs((was) => was && { ...was, items: on ? [...was.items, item] : was.items.filter((one) => one.kind !== item.kind || one.name !== item.name) }),
          }),
      ...(account?.home === false ? { made: false, owners: profile?.orgs ?? [] } : {}),
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
  }, [home, rows, syncing, syncs, takes, account?.home, profile?.orgs]);

  return <SharedHomeProvider value={signedIn ? value : undefined}>{children}</SharedHomeProvider>;
}

/** Hands the window the home a scenario carries. Nothing carried, nothing is drawn. */
export function SharedHomeFrom({ home, children }: { home: MockDraft["sharedHome"]; children: ReactNode }) {
  return home === undefined ? <>{children}</> : <Held home={home}>{children}</Held>;
}
