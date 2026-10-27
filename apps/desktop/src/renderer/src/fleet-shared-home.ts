// Kit against the shared home, over Fleet: the rows Kit and the Theme rows mark, and the acts that settle one. Fleet
// pushes no event for it, so it is read once the account names a profile, and every act answers with the read as it
// now stands. `docs/practices/protocol/the-shared-home.md`.
//
// **One home is drawn, the chosen profile's**: the login added on purpose beside the machine's own, whose What syncs
// is the person's choice. A machine with no chosen profile draws its one profile's home. Nothing is drawn while no
// profile is signed in or Fleet is not connected.

import type { Profile, SharedHome, SharedRow } from "@armada/components";
import type { HomeKind, HomeKinds, HomeRow, HomeRowAt, HomeSynced, SharedHomeRead as WireRead } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { AccountsActed } from "../../shared/api/accounts";
import type { AccountSource } from "./fleet-account";
import { askToTell } from "./tell";

/** The name Fleet gives every profile's home, `<login>/armada-home`, drawn before this machine holds a copy. */
export const HOME_REPOSITORY = "armada-home";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetHome = {
  getSharedHome: (login: string) => Promise<AccountsActed<WireRead>>;
  syncSharedHome: (login: string) => Promise<AccountsActed<HomeSynced>>;
  addToSharedHome: (row: HomeRowAt) => Promise<AccountsActed<WireRead>>;
  bringFromSharedHome: (row: HomeRowAt) => Promise<AccountsActed<WireRead>>;
  chooseSharedHomeSide: (row: HomeRowAt, side: "here" | "there") => Promise<AccountsActed<WireRead>>;
  setHomeRepository: (login: string, owner: string, name: string) => Promise<AccountsActed<WireRead>>;
  getHomeKinds: (login: string) => Promise<AccountsActed<HomeKinds>>;
  setHomeKind: (login: string, kind: HomeKind, on: boolean) => Promise<AccountsActed<HomeKinds>>;
  setHomeItem: (row: HomeRowAt, on: boolean) => Promise<AccountsActed<HomeKinds>>;
};

/** Told after a Sync that had nothing switched on to take. */
export const NOTHING_SYNCS = "Nothing syncs yet: no kind or item is switched on";

export type SharedHomeSource = {
  get(): SharedHome | undefined;
  subscribe(on: () => void): () => void;
  /** Read `login`'s home again, where it is the one drawn: What syncs turned a kind on or off. */
  readAgain(login: string): void;
};

/** The profile whose home is drawn. */
const profileOf = (account: AccountSource): Profile | undefined => {
  const profiles = account.get()?.account.profiles ?? [];
  return profiles.find((one) => one.chosen) ?? profiles[0];
};

/** The wire's row as Kit draws it. `key_here: false` is a server whose key is not here, which Kit marks as needing its key. */
const rowOf = (wire: HomeRow): SharedRow => ({
  kind: wire.kind,
  name: wire.name,
  state: wire.state,
  ...(wire.says === undefined ? {} : { says: wire.says }),
  ...(wire.source === undefined ? {} : { source: wire.source }),
  ...(wire.key_here === false ? { keyHere: true } : {}),
  ...(wire.mod_kind === undefined ? {} : { modKind: wire.mod_kind }),
  ...(wire.here === undefined ? {} : { here: wire.here }),
  ...(wire.there === undefined ? {} : { there: wire.there }),
});

export function createFleetSharedHome(fleet: FleetHome, account: AccountSource, tell: (sentence: string) => void = askToTell): SharedHomeSource {
  let login: string | undefined;
  let read: WireRead | undefined;
  // What syncs for the drawn login, read beside the rows.
  let switched: HomeKinds | undefined;
  let syncing = false;
  // A choice of owner and name pressed and not yet answered, drawn as if Fleet had taken it.
  let chosen: { owner: string; name: string } | undefined;
  let held: SharedHome | undefined;
  // Each row drawn, back to the wire row it came from: a memory note's act names its repository, which Kit's row does not carry.
  // Kept across reads, so a row a compare sheet still holds names itself as it was drawn.
  const wires = new WeakMap<SharedRow, HomeRow>();
  let off: (() => void) | null = null;
  const listeners = new Set<() => void>();

  // Each request takes a number as it leaves, and an answer older than one already taken is dropped.
  let issued = 0;
  let taken = 0;

  const atOf = (row: SharedRow, as: string): HomeRowAt => {
    const wire = wires.get(row);
    return { login: as, kind: (wire?.kind ?? row.kind) as HomeRowAt["kind"], name: wire?.name ?? row.name, ...(wire?.repository === undefined ? {} : { repository: wire.repository }) };
  };

  const build = (): SharedHome | undefined => {
    if (login === undefined || read === undefined) return undefined;
    const as = login;
    const rows = read.rows.map((wire) => {
      const row = rowOf(wire);
      wires.set(row, wire);
      return row;
    });
    // Where the home is: the copy here, else where Fleet plans it, else the login's default. Only a home with no copy here
    // and none made on another Mac can still be moved.
    const place = read.home ?? read.planned;
    const open = read.home === undefined && read.planned?.exists !== true;
    const owner = open ? chosen?.owner : undefined;
    return {
      account: owner ?? place?.account ?? as,
      repository: (open ? chosen?.name : undefined) ?? place?.repository ?? HOME_REPOSITORY,
      login: as,
      ...(open ? { made: false, owners: profileOf(account)?.orgs ?? [], setHome: (to: string, name: string) => void choose(as, to, name) } : {}),
      rows,
      syncing,
      ...(switched === undefined
        ? {}
        : {
            syncs: { kinds: switched.kinds.filter((one) => one.on).map((one) => one.kind), items: switched.items.map((one) => ({ kind: one.kind, name: one.name })) },
            setKind: (kind: string, on: boolean) => void switchTo(as, () => fleet.setHomeKind(as, kind as HomeKind, on)),
            setItem: (item: { kind: string; name: string }, on: boolean) => void switchTo(as, () => fleet.setHomeItem({ login: as, kind: item.kind as HomeKind, name: item.name }, on)),
          }),
      sync: () => void sync(as),
      add: (row) => void act(as, () => fleet.addToSharedHome(atOf(row, as))),
      bring: (row) => void act(as, () => fleet.bringFromSharedHome(atOf(row, as))),
      choose: (row, side) => void act(as, () => fleet.chooseSharedHomeSide(atOf(row, as), side)),
    };
  };

  const publish = (): void => {
    held = build();
    listeners.forEach((on) => on());
  };

  /** Take an answer for `as`, unless the drawn profile changed or a later answer was taken, and say whether it was taken. A refusal is told, as a press that failed. */
  const took = (as: string, at: number, answer: AccountsActed<WireRead>, told: boolean): boolean => {
    if (as !== login || at <= taken) return false;
    if (!answer.ok) {
      if (told) tell(refusalWords(answer.outcome));
      return false;
    }
    taken = at;
    read = answer.value;
    publish();
    return true;
  };

  const kindsOf = (as: string, answer: AccountsActed<HomeKinds>, told: boolean): boolean => {
    if (as !== login) return false;
    if (!answer.ok) {
      if (told) tell(refusalWords(answer.outcome));
      return false;
    }
    switched = answer.value;
    publish();
    return true;
  };

  const load = (as: string): void => {
    const at = ++issued;
    void fleet.getSharedHome(as).then((answer) => took(as, at, answer, false));
    void fleet.getHomeKinds(as).then((answer) => kindsOf(as, answer, false));
  };

  /** Switch a kind or an item, then read the rows again: what is drawn follows what syncs. */
  const switchTo = async (as: string, ask: () => Promise<AccountsActed<HomeKinds>>): Promise<void> => {
    if (kindsOf(as, await ask(), true)) {
      const at = ++issued;
      took(as, at, await fleet.getSharedHome(as), false);
    }
  };

  const act = async (as: string, ask: () => Promise<AccountsActed<WireRead>>): Promise<void> => {
    const at = ++issued;
    took(as, at, await ask(), true);
  };

  const choose = async (as: string, owner: string, name: string): Promise<void> => {
    const mine = { owner, name };
    chosen = mine;
    publish();
    const at = ++issued;
    const answer = await fleet.setHomeRepository(as, owner, name);
    if (chosen === mine) chosen = undefined;
    // A refusal is told, and the place it stood at is drawn again.
    if (took(as, at, answer, true)) account.refresh?.();
    else if (as === login) publish();
  };

  const sync = async (as: string): Promise<void> => {
    if (syncing) return;
    syncing = true;
    publish();
    const at = ++issued;
    const answer = await fleet.syncSharedHome(as);
    syncing = false;
    // A row the sync could not add stays on this machine, told one by one.
    if (took(as, at, answer, true) && answer.ok) {
      answer.value.left.forEach((one) => tell(one.why));
      if (switched !== undefined && !switched.kinds.some((one) => one.on) && switched.items.length === 0) tell(NOTHING_SYNCS);
      // The Account page names the home it made.
      account.refresh?.();
    }
    // A refused or overtaken sync still stops turning.
    if (held?.syncing === true) publish();
  };

  const follow = (): void => {
    const now = profileOf(account)?.login;
    if (now === login) return;
    login = now;
    read = undefined;
    switched = undefined;
    chosen = undefined;
    syncing = false;
    publish();
    if (now !== undefined) load(now);
  };

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      if (off === null) {
        off = account.subscribe(follow);
        follow();
      }
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && off !== null) {
          off();
          off = null;
        }
      };
    },
    readAgain: (as) => {
      if (as === login) load(as);
    },
  };
}
