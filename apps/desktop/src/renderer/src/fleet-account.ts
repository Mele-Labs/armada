// The account over Fleet: the profiles this machine has joined and the sign-in in flight. Fleet pushes
// no event for it, so a read follows connecting, and while a flow is not idle another follows every
// `POLL_MS` until it is. Nothing is held for a Fleet that has no such route or is not connected, and
// nothing is drawn for it: the title menu offers no account where none is provided.

import type { Account, AccountFlow } from "@armada/components";
import type { AccountFlow as WireFlow, Accounts, SignedOut, SignInEnded } from "@armada/protocol";

import type { AccountsActed } from "../../shared/api/accounts";
import type { BridgeState } from "../../shared/bridge";

/** How often a flow that is not idle is read again. */
export const POLL_MS = 2_000;

/** What the window holds of the account: what the title menu draws, and what Fleet said that nothing draws yet. */
export type AccountHeld = {
  account: Account;
  /** How the last sign-in ended, as Fleet holds it until the next starts. The composition has no place for it. */
  ended?: SignInEnded;
  /** Fleet's own words when it would not begin a sign-in, such as the setting that is blank. The composition has no place for it. */
  refused?: string;
};

export type AccountSource = {
  get(): AccountHeld | undefined;
  subscribe(on: () => void): () => void;
  /** Signs a profile out of this machine. The composition has no act for it yet, so nothing draws it. */
  signOut(accountId: number): Promise<AccountsActed<SignedOut>>;
};

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetAccounts = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getAccounts: () => Promise<AccountsActed>;
  startSignIn: () => Promise<AccountsActed>;
  signOut: (accountId: number) => Promise<AccountsActed<SignedOut>>;
  openLink: (address: string) => Promise<unknown>;
};

/** Where a pending read waits: `setTimeout` in the app, a hand-turned clock in a test. */
export type Later = (run: () => void, ms: number) => () => void;

const timer: Later = (run, ms) => {
  const id = setTimeout(run, ms);
  return () => clearTimeout(id);
};

const flowOf = (wire: WireFlow): AccountFlow =>
  wire.phase === "code"
    ? { phase: "code", code: wire.code }
    : wire.phase === "setup"
      ? { phase: "setup", login: wire.login, repository: wire.repository, steps: wire.steps }
      : { phase: "idle" };

export function createFleetAccount(fleet: FleetAccounts, later: Later = timer): AccountSource {
  let wire: Accounts | undefined;
  let refused: string | undefined;
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
      if (refused !== undefined) publish(wire, undefined);
    },
    // A repository is bound to a profile and the machine has none that is active, so there is nothing to
    // switch; what syncs is not served yet either.
    switchTo: () => {},
    choose: () => {},
    closeSyncs: () => {},
    setKind: () => {},
    setScope: () => {},
  };

  const heldOf = (from: Accounts, said: string | undefined): AccountHeld => {
    // The composition still draws one profile as active; Fleet has none, so none is marked.
    const profiles = from.profiles.map((one) => ({ account_id: one.account_id, login: one.login, chosen: one.chosen, active: false }));
    const account: Account = { profiles, flow: flowOf(from.flow), syncs: null, ...acts };
    const ended = from.flow.phase === "idle" ? from.flow.ended : undefined;
    return { account, ...(ended === undefined ? {} : { ended }), ...(said === undefined ? {} : { refused: said }) };
  };

  const publish = (to: Accounts | undefined, said: string | undefined): void => {
    const same = JSON.stringify([to, said]) === JSON.stringify([wire, refused]);
    wire = to;
    refused = said;
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

  const changed = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    else if (!connected && wasConnected) {
      cancel?.();
      cancel = null;
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
    signOut: async (accountId) => {
      const at = ++issued;
      const answer = await fleet.signOut(accountId);
      if (answer.ok) took({ ok: true, value: answer.value.accounts }, at);
      return answer;
    },
  };
}
