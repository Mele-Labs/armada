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
  BoundRepository,
  Profile,
  SetUpState,
  SignedOutNote,
  SignInEnd,
  SignInUnavailable,
  SyncChoice,
  SyncKindWord,
  UnboundRepository,
} from "@armada/components";
import { openSettingsAt } from "@armada/settings";

import type { MockDraft } from "./moment";

const STEP_MS = 600;
const ADD_MS = 700;
const PULL_MS = 450;
const HOLD_MS = 8000;
/** What a code has left when it is shown. */
const EXPIRES_S = 899;

const SignedIn = createContext(true);

/** Whether the window has signed in, for what is drawn only against an account. Signed in where there is no account to offer. */
export const useSignedIn = (): boolean => useContext(SignedIn);

/** The kinds each added profile has turned on in What syncs, by login. A login with no entry chose nothing, so nothing is held back. */
const Synced = createContext<Readonly<Record<string, readonly SyncKindWord[]>>>({});

/** The kinds `login` syncs on this machine, or `undefined` where it has no What syncs to choose in. */
export const useSyncedKinds = (login: string): readonly SyncKindWord[] | undefined => useContext(Synced)[login];

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
    (offer.joined ?? []).map((one, at) => ({
      account_id: at + 1,
      login: one.login,
      active: at === 0,
      chosen: one.chosen,
      ...(offer.orgs?.[one.login] === undefined ? {} : { orgs: offer.orgs[one.login] }),
      ...(offer.noHome === undefined ? { repository: offer.signIn.repository } : {}),
    })),
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
  // What the person chose for a repository, over what matching said: a login, or `null` for None.
  const [picked, setPicked] = useState<Readonly<Record<string, string | null>>>({});
  // How many attempts have started, and how the one in flight will end.
  const tried = useRef(0);
  const planned = useRef<EndedOutcome | null>(null);

  const begin = useCallback((code: string, adds: boolean) => {
    const attempt = offer.attempts?.[tried.current];
    tried.current += 1;
    setEnded(undefined);
    setUnavailable(undefined);
    setLeft(undefined);
    if (typeof attempt === "object" && "blank" in attempt) return setUnavailable(unavailableFor(attempt.blank));
    planned.current = typeof attempt === "object" && "ends" in attempt ? attempt.ends : null;
    setAdding(adds);
    setFlow({ phase: "code", code, expiresIn: EXPIRES_S });
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
          setProfiles((was) => [
            ...was,
            { account_id: 2, login: added.login, active: false, chosen: true, ...(offer.noHome === undefined ? { repository: offer.signIn.repository } : {}) },
          ]);
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
            setProfiles([{ account_id: 1, login, active: true, chosen: false, ...(offer.orgs?.[login] === undefined ? {} : { orgs: offer.orgs[login] }) }]);
            setAuthorized(false);
          }, STEP_MS * 2),
        );
      } else {
        timers.push(window.setTimeout(() => next(steps("done", "done", "working")), STEP_MS * 2));
        timers.push(
          window.setTimeout(() => {
            next(steps("done", "done", "done"));
            setProfiles([{ account_id: 1, login, active: true, chosen: false, repository, ...(offer.orgs?.[login] === undefined ? {} : { orgs: offer.orgs[login] }) }]);
            setAuthorized(false);
          }, STEP_MS * 3),
        );
      }
    }
    return () => timers.forEach((one) => window.clearTimeout(one));
  }, [authorized, adding, offer]);

  // What signing in set up is read, then put away: Fleet's flow goes idle once the profile is in.
  useEffect(() => {
    if (flow.phase !== "setup" || profiles.length === 0) return undefined;
    const timer = window.setTimeout(() => setFlow((was) => (was.phase === "setup" ? { phase: "idle" } : was)), HOLD_MS);
    return () => window.clearTimeout(timer);
  }, [flow.phase, profiles.length]);

  const pulling = Object.values(kinds).some((one) => one.some((kind) => kind.pulling));
  useEffect(() => {
    if (!pulling) return undefined;
    const timer = window.setTimeout(
      () => setKinds((was) => Object.fromEntries(Object.entries(was).map(([login, one]) => [login, one.map((kind) => ({ ...kind, pulling: false }))]))),
      PULL_MS,
    );
    return () => window.clearTimeout(timer);
  }, [pulling]);

  // A mock repository is named once, so its name stands for its root.
  // Each repository is bound to one profile: by its owner being the login or one of its organisations, or by choice.
  // One no profile matches, or two both do, waits unbound until it is chosen.
  const { bound, unbound } = useMemo(() => {
    const bound = new Map<string, BoundRepository[]>();
    const unbound: UnboundRepository[] = [];
    if (profiles.length === 0) return { bound, unbound };
    const owns = (login: string, name: string) => {
      const owner = name.split("/")[0] ?? "";
      return owner === login || (offer.orgs?.[login] ?? []).includes(owner);
    };
    const put = (login: string, row: BoundRepository) => bound.set(login, [...(bound.get(login) ?? []), row]);
    for (const name of offer.repositories ?? []) {
      const choice = picked[name];
      if (choice === null) unbound.push({ root: name, name, none: true });
      else if (choice !== undefined && profiles.some((one) => one.login === choice)) put(choice, { root: name, name, by: "chosen" });
      else {
        const matches = profiles.filter((one) => owns(one.login, name));
        if (matches.length === 1 && matches[0] !== undefined) put(matches[0].login, { root: name, name, by: "matched" });
        else unbound.push({ root: name, name });
      }
    }
    return { bound, unbound };
  }, [profiles, picked, offer]);

  const value = useMemo<Account>(() => {
    const change = (login: string, kind: SyncKindWord, edit: (one: SyncChoice) => SyncChoice) =>
      setKinds((was) => ({ ...was, [login]: (was[login] ?? []).map((one) => (one.kind === kind ? edit(one) : one)) }));
    return {
      profiles: offer.repositories === undefined ? profiles : profiles.map((one) => ({ ...one, repositories: bound.get(one.login) ?? [] })),
      flow,
      syncs: open === null ? null : { login: open, kinds: kinds[open] ?? [], home: { repository: "armada-home", made: (kinds[open] ?? []).some((one) => one.on) } },
      ...(ended === undefined ? {} : { ended }),
      ...(unavailable === undefined ? {} : { unavailable }),
      ...(left === undefined ? {} : { left }),
      ...(offer.noHome === undefined ? {} : { home: false }),
      ...(unbound.length === 0 ? {} : { unbound }),
      ...(offer.repositories === undefined ? {} : { bind: (root: string, login: string | null) => setPicked((was) => ({ ...was, [root]: login })) }),
      ...(offer.machine === undefined ? {} : { machine: offer.machine }),
      cancel: () => {
        planned.current = null;
        setAdding(false);
        setAuthorized(false);
        setFlow((was) => (was.phase === "code" ? { phase: "idle" } : was));
      },
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
        setPicked((was) => Object.fromEntries(Object.entries(was).filter(([, to]) => to !== login)));
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
  }, [profiles, flow, kinds, open, offer, ended, unavailable, left, begin, bound, unbound]);

  const synced = useMemo(() => Object.fromEntries(Object.entries(kinds).map(([login, one]) => [login, one.filter((kind) => kind.on).map((kind) => kind.kind)])), [kinds]);

  return (
    <AccountProvider value={value}>
      <SignedIn.Provider value={profiles.length > 0}>
        <Synced.Provider value={synced}>
          {children}
          <WhatSyncsSheet />
        </Synced.Provider>
      </SignedIn.Provider>
    </AccountProvider>
  );
}

/** Hands the window the account a scenario carries. Nothing carried, nothing is drawn. */
export function AccountFrom({ account, children }: { account: MockDraft["account"]; children: ReactNode }) {
  return account === undefined ? <>{children}</> : <Held offer={account}>{children}</Held>;
}
