// The account this machine is signed in to: the profiles it has joined and the sign-in in flight.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Accounts, Outcome, SignedOut } from "@armada/protocol";

/** What an account act came to: the thing Fleet answered with, or the refusal to word. */
export type AccountsActed<T = Accounts> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type AccountsApi = {
  /** The profiles and the sign-in in flight. Read again while a flow is not idle: Fleet pushes no event for it. */
  getAccounts: () => Promise<AccountsActed>;
  /** Begins the device flow. Answers with the code to type; Fleet refuses, naming the setting, where sign-in is not set up. */
  startSignIn: () => Promise<AccountsActed>;
  /** Signs a profile out of this machine and the store. The profile is gone either way. */
  signOut: (accountId: number) => Promise<AccountsActed<SignedOut>>;
  /** Binds the repository at `root` to a profile, or to none (`null`): what it claims and syncs goes to that profile's store. */
  setRepositoryProfile: (root: string, accountId: number | null) => Promise<AccountsActed>;
};

export const ACCOUNTS_CHANNELS = {
  getAccounts: "bridge:get-accounts",
  startSignIn: "bridge:start-sign-in",
  signOut: "bridge:sign-out",
  setRepositoryProfile: "bridge:set-repository-profile",
} as const;
