// The account this machine is signed in to: the profiles it has joined and the sign-in in flight, and each
// profile's shared home: Kit's rows against it, the acts that settle a row, and What syncs.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Accounts, HomeKind, HomeKinds, HomeRowAt, HomeSide, HomeSynced, Outcome, SharedHomeRead, SignedOut } from "@armada/protocol";

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
  /** Kit's rows against `login`'s shared home. Fleet pushes no event for it, so it is read again after an act. */
  getSharedHome: (login: string) => Promise<AccountsActed<SharedHomeRead>>;
  /** Makes the home where it does not exist, clones or pulls it, and answers the rows as they now stand. */
  syncSharedHome: (login: string) => Promise<AccountsActed<HomeSynced>>;
  /** Puts a row only this machine has into the home. */
  addToSharedHome: (row: HomeRowAt) => Promise<AccountsActed<SharedHomeRead>>;
  /** Brings a row only the home has to this machine. */
  bringFromSharedHome: (row: HomeRowAt) => Promise<AccountsActed<SharedHomeRead>>;
  /** Keeps one side of a row that differs. */
  chooseSharedHomeSide: (row: HomeRowAt, side: HomeSide) => Promise<AccountsActed<SharedHomeRead>>;
  /** What syncs for `login`: every kind, off until it is turned on. */
  getHomeKinds: (login: string) => Promise<AccountsActed<HomeKinds>>;
  setHomeKind: (login: string, kind: HomeKind, on: boolean) => Promise<AccountsActed<HomeKinds>>;
  /** Where `login`'s shared home is made: the owner (the login or one of its organisations) and the repository name. Answers with the home read again. */
  setHomeRepository: (login: string, owner: string, name: string) => Promise<AccountsActed<SharedHomeRead>>;
};

export const ACCOUNTS_CHANNELS = {
  getAccounts: "bridge:get-accounts",
  startSignIn: "bridge:start-sign-in",
  signOut: "bridge:sign-out",
  setRepositoryProfile: "bridge:set-repository-profile",
  getSharedHome: "bridge:get-shared-home",
  syncSharedHome: "bridge:sync-shared-home",
  addToSharedHome: "bridge:add-to-shared-home",
  bringFromSharedHome: "bridge:bring-from-shared-home",
  chooseSharedHomeSide: "bridge:choose-shared-home-side",
  getHomeKinds: "bridge:get-home-kinds",
  setHomeKind: "bridge:set-home-kind",
  setHomeRepository: "bridge:set-home-repository",
} as const;
