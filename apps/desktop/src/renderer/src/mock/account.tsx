// The GitHub account a machine is signed in to, as the mock holds it: signed out until the device
// flow finishes, then the profiles it has joined, and what each added profile syncs (`draft.account`).
// Fleet serves none of it yet. Pressing the link on the code card stands for typing the code at GitHub;
// Fleet's poll finishes a moment after, and the shared home appears only once the sign-in has set it up.

import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { AccountProvider, SYNC_KINDS, WhatSyncsSheet } from "@armada/components";
import type { Account, AccountFlow, Profile, SetUpState, SyncChoice, SyncKindWord } from "@armada/components";

import type { MockDraft } from "./moment";

const STEP_MS = 600;
const ADD_MS = 700;
const PULL_MS = 1000;

const SignedIn = createContext(true);

/** Whether the window has signed in, for what is drawn only against an account. Signed in where there is no account to offer. */
export const useSignedIn = (): boolean => useContext(SignedIn);

type Offer = NonNullable<MockDraft["account"]>;

const choices = (offer: NonNullable<Offer["add"]>): SyncChoice[] =>
  SYNC_KINDS.map((kind) => ({ kind, on: false, pulling: false, items: offer.items[kind] ?? [] }));

function Held({ offer, children }: { offer: Offer; children: ReactNode }) {
  const [profiles, setProfiles] = useState<readonly Profile[]>([]);
  const [flow, setFlow] = useState<AccountFlow>({ phase: "idle" });
  const [adding, setAdding] = useState(false);
  const [authorized, setAuthorized] = useState(false);
  const [kinds, setKinds] = useState<Readonly<Record<string, readonly SyncChoice[]>>>({});
  const [open, setOpen] = useState<string | null>(null);

  // Fleet's poll finishing: the sign-in sets up row by row, an added profile joins at once.
  useEffect(() => {
    if (!authorized) return undefined;
    const { login, repository } = offer.signIn;
    const timers: number[] = [];
    const steps = (identity: SetUpState, store: SetUpState, home: SetUpState): AccountFlow => ({
      phase: "setup",
      login,
      repository,
      steps: { identity, store, home },
    });
    if (adding && offer.add !== undefined) {
      const added = offer.add;
      timers.push(
        window.setTimeout(() => {
          setProfiles((was) => [...was, { login: added.login, active: false, chosen: true }]);
          setKinds((was) => ({ ...was, [added.login]: choices(added) }));
          setOpen(added.login);
          setFlow({ phase: "idle" });
          setAdding(false);
          setAuthorized(false);
        }, ADD_MS),
      );
    } else {
      setFlow(steps("working", "waiting", "waiting"));
      // A panel put away mid-way leaves the flow idle, and the rows stop being drawn.
      const next = (to: AccountFlow) => setFlow((was) => (was.phase === "setup" ? to : was));
      timers.push(window.setTimeout(() => next(steps("done", "working", "waiting")), STEP_MS));
      timers.push(window.setTimeout(() => next(steps("done", "done", "working")), STEP_MS * 2));
      timers.push(
        window.setTimeout(() => {
          next(steps("done", "done", "done"));
          setProfiles([{ login, active: true, chosen: false }]);
          setAuthorized(false);
        }, STEP_MS * 3),
      );
    }
    return () => timers.forEach((one) => window.clearTimeout(one));
  }, [authorized, adding, offer]);

  const pulling = Object.values(kinds).some((one) => one.some((kind) => kind.pulling));
  useEffect(() => {
    if (!pulling) return undefined;
    const timer = window.setTimeout(
      () => setKinds((was) => Object.fromEntries(Object.entries(was).map(([login, one]) => [login, one.map((kind) => ({ ...kind, pulling: false }))]))),
      PULL_MS,
    );
    return () => window.clearTimeout(timer);
  }, [pulling]);

  const value = useMemo<Account>(() => {
    const change = (login: string, kind: SyncKindWord, edit: (one: SyncChoice) => SyncChoice) =>
      setKinds((was) => ({ ...was, [login]: (was[login] ?? []).map((one) => (one.kind === kind ? edit(one) : one)) }));
    return {
      profiles,
      flow,
      syncs: open === null ? null : { login: open, kinds: kinds[open] ?? [] },
      signIn: () => {
        if (flow.phase === "idle") setFlow({ phase: "code", code: offer.signIn.code });
      },
      openDevicePage: () => {
        if (flow.phase === "code") setAuthorized(true);
      },
      addProfile: () => {
        if (offer.add === undefined) return;
        setAdding(true);
        setFlow({ phase: "code", code: offer.add.code });
      },
      dismiss: () => setFlow((was) => (was.phase === "setup" ? { phase: "idle" } : was)),
      switchTo: (login) => setProfiles((was) => was.map((one) => ({ ...one, active: one.login === login }))),
      choose: (login) => setOpen(login),
      closeSyncs: () => setOpen(null),
      setKind: (kind, on) => {
        if (open !== null) change(open, kind, (one) => ({ ...one, on, pulling: on }));
      },
      setScope: (kind, name, scope) => {
        if (open !== null) change(open, kind, (one) => ({ ...one, items: one.items.map((item) => (item.name === name ? { ...item, scope } : item)) }));
      },
    };
  }, [profiles, flow, kinds, open, offer]);

  return (
    <AccountProvider value={value}>
      <SignedIn.Provider value={profiles.length > 0}>
        {children}
        <WhatSyncsSheet />
      </SignedIn.Provider>
    </AccountProvider>
  );
}

/** Hands the window the account a scenario carries. Nothing carried, nothing is drawn. */
export function AccountFrom({ account, children }: { account: MockDraft["account"]; children: ReactNode }) {
  return account === undefined ? <>{children}</> : <Held offer={account}>{children}</Held>;
}
