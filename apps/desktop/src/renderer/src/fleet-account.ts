// The account over Fleet: the profiles this machine has joined and the sign-in in flight. Fleet pushes
// no event for it, so a read follows connecting, and while a flow is not idle another follows every
// `POLL_MS` until it is. Nothing is held for a Fleet that has no such route or is not connected, and
// nothing is drawn for it: the title menu offers no account where none is provided.

import type { Account, AccountFlow, BlankSetting, Profile, SignedOutNote, UnboundRepository } from "@armada/components";
import { openSettingsAt } from "@armada/settings";
import type { AccountFlow as WireFlow, Accounts, SignedOut, SignInEnded } from "@armada/protocol";

import type { AccountsActed } from "../../shared/api/accounts";
import type { BridgeState } from "../../shared/bridge";

/** How often a flow that is not idle is read again. */
export const POLL_MS = 2_000;

/** What the window holds of the account: what the title menu draws, and what Fleet said that nothing draws yet. */
export type AccountHeld = {
  account: Account;
  /** How the last sign-in ended, as Fleet holds it until the next starts. */
  ended?: SignInEnded;
  /** Fleet's own words when it would not begin a sign-in, such as the setting that is blank. */
  refused?: string;
};

export type AccountSource = {
  get(): AccountHeld | undefined;
  subscribe(on: () => void): () => void;
  /** Signs a profile out of this machine, and holds who left until the panel is put away. */
  signOut(accountId: number): Promise<AccountsActed<SignedOut>>;
};

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetAccounts = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getAccounts: () => Promise<AccountsActed>;
  startSignIn: () => Promise<AccountsActed>;
  signOut: (accountId: number) => Promise<AccountsActed<SignedOut>>;
  /** Binds the repository at `root` to the profile, or to none (`null`). */
  setRepositoryProfile: (root: string, accountId: number | null) => Promise<AccountsActed>;
  openLink: (address: string) => Promise<unknown>;
};

/** Where a pending read waits: `setTimeout` in the app, a hand-turned clock in a test. */
export type Later = (run: () => void, ms: number) => () => void;

const timer: Later = (run, ms) => {
  const id = setTimeout(run, ms);
  return () => clearTimeout(id);
};

/** The settings Fleet's refusal names as blank. Its sentence is all the wire carries of them. */
const blankIn = (said: string): BlankSetting[] => [
  ...(said.includes("sign-in app") ? (["account"] as const) : []),
  ...(said.includes("store address") ? (["store"] as const) : []),
];

const flowOf = (wire: WireFlow): AccountFlow =>
  wire.phase === "code"
    ? { phase: "code", code: wire.code, expiresIn: wire.expires_in_seconds }
    : wire.phase === "setup"
      ? { phase: "setup", login: wire.login, repository: wire.repository, steps: wire.steps }
      : { phase: "idle" };

export function createFleetAccount(fleet: FleetAccounts, later: Later = timer): AccountSource {
  let wire: Accounts | undefined;
  let refused: string | undefined;
  // Who signed out, until the panel is put away; and the ended sign-in the panel has been put away on.
  let left: SignedOutNote | undefined;
  let put: string | undefined;
  let shown = "";
  let held: AccountHeld | undefined;
  let wasConnected = false;
  let starting = false;
  let cancel: (() => void) | null = null;
  let stop: (() => void) | null = null;
  const listeners = new Set<() => void>();

  // Each request takes a number as it leaves, and an answer older than one already taken is dropped, so
  // a poll that left first cannot undo the act that left after it.
  let issued = 0;
  let taken = 0;

  const acts = {
    signIn: () => begin(),
    addProfile: () => begin(),
    openDevicePage: () => {
      const flow = wire?.flow;
      if (flow?.phase === "code" && flow.address.startsWith("https://")) void fleet.openLink(flow.address);
    },
    dismiss: () => {
      const ended = wire?.flow.phase === "idle" ? wire.flow.ended : undefined;
      const hide = ended === undefined ? undefined : JSON.stringify(ended);
      if (refused === undefined && left === undefined && hide === put) return;
      left = undefined;
      put = hide;
      publish(wire, undefined);
    },
    signOut: (login: string) => {
      const profile = wire?.profiles.find((one) => one.login === login);
      if (profile !== undefined) void leave(profile.account_id, login);
    },
    openSetting: (setting: BlankSetting) => openSettingsAt(setting, () => undefined),
    // A repository is bound to a profile and the machine has none that is active, so there is nothing to switch.
    switchTo: () => {},
    bind: (repository: string, login: string | null) => void bind(repository, login),
    // What syncs is not served yet.
    choose: () => {},
    closeSyncs: () => {},
    setKind: () => {},
    setScope: () => {},
  };

  const heldOf = (from: Accounts, said: string | undefined): AccountHeld => {
    // The composition still carries `active`; Fleet has no active profile, so none is marked.
    const profiles: Profile[] = from.profiles.map((one) => ({
      account_id: one.account_id,
      login: one.login,
      chosen: one.chosen,
      active: false,
      repositories: one.repositories.map((bound) => ({ name: bound.name, by: bound.how })),
    }));
    const unbound: UnboundRepository[] = from.unmatched.map((loose) => ({ name: loose.name, ...(loose.none_chosen ? { none: true } : {}) }));
    const ended = from.flow.phase === "idle" ? from.flow.ended : undefined;
    const blank = said === undefined ? [] : blankIn(said);
    // A refusal that names no blank setting is the forge declining to begin: it ends the sign-in, refused.
    const end =
      ended === undefined || ended.outcome === "signed_in" || JSON.stringify(ended) === put
        ? undefined
        : { outcome: ended.outcome, said: ended.said ?? "" };
    const account: Account = {
      profiles,
      flow: flowOf(from.flow),
      syncs: null,
      ...(unbound.length === 0 ? {} : { unbound }),
      ...(end === undefined ? {} : { ended: end }),
      ...(said === undefined ? {} : blank.length > 0 ? { unavailable: { said, blank } } : { ended: { outcome: "refused" as const, said } }),
      ...(left === undefined ? {} : { left }),
      ...acts,
    };
    return { account, ...(ended === undefined ? {} : { ended }), ...(said === undefined ? {} : { refused: said }) };
  };

  const publish = (to: Accounts | undefined, said: string | undefined): void => {
    const key = JSON.stringify([to, said, left, put]);
    const same = key === shown;
    wire = to;
    refused = said;
    shown = key;
    if (same) return;
    held = to === undefined ? undefined : heldOf(to, said);
    listeners.forEach((on) => on());
  };

  const poll = (): void => {
    cancel?.();
    cancel = null;
    if (wire === undefined || wire.flow.phase === "idle") return;
    cancel = later(() => {
      cancel = null;
      read();
    }, POLL_MS);
  };

  const took = (answer: AccountsActed, at: number): boolean => {
    if (at > taken && answer.ok) {
      taken = at;
      publish(answer.value, undefined);
    }
    // A read that failed leaves what stands, and a flow still in flight is read again.
    poll();
    return answer.ok;
  };

  const read = (): void => {
    const at = ++issued;
    void fleet.getAccounts().then((answer) => took(answer, at));
  };

  const begin = (): void => {
    if (starting || (wire !== undefined && wire.flow.phase !== "idle")) return;
    starting = true;
    left = undefined;
    put = undefined;
    const at = ++issued;
    void fleet
      .startSignIn()
      .then((answer) => {
        if (answer.ok) return took(answer, at);
        // Fleet's refusal is an answer: it names which setting is blank, and nothing else changes.
        if (answer.outcome.ok === false && answer.outcome.why === "refused" && wire !== undefined) publish(wire, answer.outcome.error.message);
        return false;
      })
      .finally(() => {
        starting = false;
      });
  };

  const leave = async (accountId: number, login: string): Promise<AccountsActed<SignedOut>> => {
    const at = ++issued;
    const answer = await fleet.signOut(accountId);
    if (answer.ok) {
      left = {
        login,
        storeRowRemains: answer.value.store_row_remains,
        ...(answer.value.said === undefined ? {} : { said: answer.value.said }),
      };
      took({ ok: true, value: answer.value.accounts }, at);
    }
    return answer;
  };

  // The composition names a repository by its folder name and a profile by its login; Fleet names them by
  // root and account id. Two repositories with one folder name are told apart by Fleet, not here.
  const bind = async (repository: string, login: string | null): Promise<void> => {
    const root = [...(wire?.profiles.flatMap((one) => one.repositories) ?? []), ...(wire?.unmatched ?? [])].find((one) => one.name === repository)?.root;
    const account = login === null ? null : wire?.profiles.find((one) => one.login === login)?.account_id;
    if (root === undefined || account === undefined) return;
    const at = ++issued;
    const answer = await fleet.setRepositoryProfile(root, account);
    // Fleet's refusal is shown as the page shows the others.
    if (!answer.ok && answer.outcome.ok === false && answer.outcome.why === "refused" && wire !== undefined) publish(wire, answer.outcome.error.message);
    else took(answer, at);
  };

  const changed = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    else if (!connected && wasConnected) {
      cancel?.();
      cancel = null;
      left = undefined;
      publish(undefined, undefined);
    }
    wasConnected = connected;
  };

  const start = (): void => {
    const off = fleet.subscribe(changed);
    void fleet.state().then(changed);
    stop = () => {
      off();
      cancel?.();
      cancel = null;
      wasConnected = false;
    };
  };

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) start();
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    signOut: (accountId) => leave(accountId, wire?.profiles.find((one) => one.account_id === accountId)?.login ?? ""),
  };
}
