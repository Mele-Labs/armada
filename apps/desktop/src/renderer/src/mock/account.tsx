// The GitHub account a machine is signed in to, as the mock holds it: signed out until the device
// flow finishes, then the profiles it has joined, and what each added profile syncs (`draft.account`).
// Fleet serves none of it yet. Pressing the link on the code card stands for typing the code at GitHub;
// Fleet's poll finishes a moment after, and the shared home appears only once the sign-in has set it up.
// A scenario can instead end each attempt in turn without signing in, refuse it for a blank setting,
// start already signed in so a profile can sign out, or set up no shared home at all.

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AccountProvider, SYNC_KINDS, WhatSyncsSheet } from "@armada/components";
import type {
  Account,
  AccountFlow,
  BlankSetting,
  EndedOutcome,
  Profile,
  SetUpState,
  SignedOutNote,
  SignInEnd,
  SignInUnavailable,
  SyncChoice,
  SyncKindWord,
} from "@armada/components";
import { openSettingsAt } from "@armada/settings";

import type { MockDraft } from "./moment";

const STEP_MS = 600;
const ADD_MS = 700;
const PULL_MS = 450;

const SignedIn = createContext(true);

/** Whether the window has signed in, for what is drawn only against an account. Signed in where there is no account to offer. */
export const useSignedIn = (): boolean => useContext(SignedIn);

type Offer = NonNullable<MockDraft["account"]>;

const choices = (offer: NonNullable<Offer["add"]>): SyncChoice[] =>
  SYNC_KINDS.map((kind) => ({ kind, on: false, pulling: false, items: offer.items[kind] ?? [] }));

/** Fleet's own sentence for each way a sign-in ends. */
const SAID: Record<EndedOutcome, string> = {
  denied: "The sign-in was turned down.",
  expired: "The code ran out before it was typed.",
  unreachable: "The forge could not be reached: the connection timed out.",
  refused: "The forge refused the sign-in: the app has the device flow off.",
};

const BLANK: Record<BlankSetting, string> = { account: "the sign-in app", store: "the store address" };

const unavailableFor = (blank: readonly BlankSetting[]): SignInUnavailable => ({
  said: `Sign-in is unavailable: ${blank.map((one) => BLANK[one]).join(" and ")} ${blank.length > 1 ? "are" : "is"} blank in Settings.`,
  blank,
});

function Held({ offer, children }: { offer: Offer; children: ReactNode }) {
  const [profiles, setProfiles] = useState<readonly Profile[]>(() =>
    (offer.joined ?? []).map((one, at) => ({ account_id: at + 1, login: one.login, active: at === 0, chosen: one.chosen })),
  );
  const [flow, setFlow] = useState<AccountFlow>({ phase: "idle" });
  const [adding, setAdding] = useState(false);
  const [authorized, setAuthorized] = useState(false);
  const [kinds, setKinds] = useState<Readonly<Record<string, readonly SyncChoice[]>>>(() => {
    const add = offer.add;
    return add === undefined ? {} : Object.fromEntries((offer.joined ?? []).filter((one) => one.chosen).map((one) => [one.login, choices(add)]));
  });
  const [open, setOpen] = useState<string | null>(null);
  const [ended, setEnded] = useState<SignInEnd | undefined>(undefined);
  const [unavailable, setUnavailable] = useState<SignInUnavailable | undefined>(undefined);
  const [left, setLeft] = useState<SignedOutNote | undefined>(undefined);
  // How many attempts have started, and how the one in flight will end.
  const tried = useRef(0);
  const planned = useRef<EndedOutcome | null>(null);

  const begin = useCallback((code: string, adds: boolean) => {
    const attempt = offer.attempts?.[tried.current];
    tried.current += 1;
    setEnded(undefined);
    setUnavailable(undefined);
    setLeft(undefined);
    if (attempt !== undefined && "blank" in attempt) return setUnavailable(unavailableFor(attempt.blank));
    planned.current = attempt === undefined ? null : attempt.ends;
    setAdding(adds);
    setFlow({ phase: "code", code });
  }, [offer]);

  // Fleet's poll finishing: the sign-in sets up row by row, an added profile joins at once.
  useEffect(() => {
    if (!authorized) return undefined;
    // An attempt that ends without signing in: the code was typed and nothing came of it.
    if (planned.current !== null) {
      const outcome = planned.current;
      planned.current = null;
      setFlow({ phase: "idle" });
      setEnded({ outcome, said: SAID[outcome] });
      setAdding(false);
      setAuthorized(false);
      return undefined;
    }
    const { login } = offer.signIn;
    const repository = offer.noHome === undefined ? offer.signIn.repository : "";
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
          setProfiles((was) => [...was, { account_id: 2, login: added.login, active: false, chosen: true }]);
          setKinds((was) => ({ ...was, [added.login]: choices(added) }));
          // With no shared home there is nothing to open What syncs on.
          if (offer.noHome === undefined) setOpen(added.login);
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
      if (offer.noHome !== undefined) {
        // Nothing makes the shared home: the third row stays where it was, and the profile joins without one.
        timers.push(
          window.setTimeout(() => {
            next(steps("done", "done", "waiting"));
            setProfiles([{ account_id: 1, login, active: true, chosen: false }]);
            setAuthorized(false);
          }, STEP_MS * 2),
        );
      } else {
        timers.push(window.setTimeout(() => next(steps("done", "done", "working")), STEP_MS * 2));
        timers.push(
          window.setTimeout(() => {
            next(steps("done", "done", "done"));
            setProfiles([{ account_id: 1, login, active: true, chosen: false }]);
            setAuthorized(false);
          }, STEP_MS * 3),
        );
      }
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
      ...(ended === undefined ? {} : { ended }),
      ...(unavailable === undefined ? {} : { unavailable }),
      ...(left === undefined ? {} : { left }),
      ...(offer.noHome === undefined ? {} : { home: false, syncsWithoutHome: offer.noHome }),
      signIn: () => {
        if (flow.phase === "idle") begin(offer.signIn.code, false);
      },
      openDevicePage: () => {
        if (flow.phase === "code") setAuthorized(true);
      },
      addProfile: () => {
        if (offer.add === undefined) return;
        begin(offer.add.code, true);
      },
      dismiss: () => {
        setFlow((was) => (was.phase === "setup" ? { phase: "idle" } : was));
        setEnded(undefined);
        setUnavailable(undefined);
        setLeft(undefined);
      },
      signOut: (login) => {
        const gone = profiles.find((one) => one.login === login);
        if (gone === undefined) return;
        setProfiles(profiles.filter((one) => one.login !== login).map((one, at) => ({ ...one, active: gone.active ? at === 0 : one.active })));
        setOpen((was) => (was === login ? null : was));
        const down = offer.storeDown?.includes(login) === true;
        setLeft({ login, storeRowRemains: down, ...(down ? { said: "The store could not be reached." } : {}) });
      },
      openSetting: (setting) => openSettingsAt(setting, () => undefined),
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
  }, [profiles, flow, kinds, open, offer, ended, unavailable, left, begin]);

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
