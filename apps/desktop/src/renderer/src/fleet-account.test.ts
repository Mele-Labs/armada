// The Fleet-backed account source, against a fake Fleet and a clock turned by hand: signing in from
// nothing to a profile by reading Fleet while the flow is not idle, how a sign-in that did not finish
// is held, Fleet's refusal to begin one, signing out, and binding a repository to a profile. No profile
// is active, so switching asks Fleet nothing. What syncs reads a profile's kinds and switches one. What the
// code card and the set-up rows look like is the composition's (`Account.stories.tsx`).

import { describe, expect, it, vi } from "vitest";

import { openSettingsAt } from "@armada/settings";

import type { AccountFlow, Accounts, HomeKind, HomeKinds, Outcome, Profile, SignedOut } from "@armada/protocol";

import type { AccountsActed } from "../../shared/api/accounts";
import { createFleetAccount, POLL_MS, type FleetAccounts, type Later } from "./fleet-account";

vi.mock("@armada/settings", () => ({ openSettingsAt: vi.fn() }));

const settled = () => new Promise((done) => setTimeout(done, 0));

const ADDRESS = "https://forge.example/login/device";
const work: Profile = { account_id: 1001, login: "mele-work", chosen: false, orgs: [], home: { account: "mele-work", repository: "armada-home", made: false }, repositories: [] };
const personal: Profile = { account_id: 1002, login: "nickmele", chosen: true, orgs: ["mele-labs"], home: { account: "mele-labs", repository: "team-home", made: true }, repositories: [] };
/** `work` as the composition is handed it: no profile is marked active. */
const workDrawn = { account_id: 1001, login: "mele-work", chosen: false, active: false, orgs: [], home: { account: "mele-work", repository: "armada-home", made: false }, repositories: [] };
const CODE: AccountFlow = { phase: "code", code: "WDJB-MJHT", address: ADDRESS, expires_in_seconds: 899 };
const SETUP: AccountFlow = { phase: "setup", login: "mele-work", repository: "armada-home", steps: { identity: "working", store: "waiting", home: "waiting" } };
const IDLE: AccountFlow = { phase: "idle" };
const UNAVAILABLE = "Sign-in is not set up: the sign-in app is blank.";
const refusal = (message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code: "fleet.sign_in_unavailable", message, run_id: "r1", fields: {}, chain: [] },
});

/** A clock whose waits run when the test says, and a record of how long each was asked to be. */
function clock() {
  const waiting: { run: () => void; ms: number }[] = [];
  const later: Later = (run, ms) => {
    const one = { run, ms };
    waiting.push(one);
    return () => void waiting.splice(waiting.indexOf(one), 1);
  };
  return {
    later,
    pending: () => waiting.length,
    waited: () => waiting.map((one) => one.ms),
    tick: async () => {
      waiting.shift()?.run();
      await settled();
    },
  };
}

/** A Fleet holding the accounts, a record of what Bridge asked of it, and the connection it reports. */
function fake(start: Accounts, over: Partial<FleetAccounts> = {}) {
  let held = start;
  let state: "connected" | "closed" = "connected";
  let hear: (facts: { connection: { state: string } }) => void = () => undefined;
  const asked: string[] = [];
  const opened: string[] = [];
  let starts: () => AccountsActed = () => ({ ok: true, value: (held = { ...held, flow: CODE }) });
  // What syncs, by login: every kind off until one is switched on.
  const on = new Map<string, Set<HomeKind>>();
  const kindsOf = (login: string): HomeKinds => ({ login, kinds: KINDS.map((kind) => ({ kind, on: on.get(login)?.has(kind) === true })) });
  const fleet: FleetAccounts = {
    state: async () => ({ connection: { state } }) as never,
    subscribe: (on) => ((hear = on as never), () => undefined),
    getAccounts: async () => (asked.push("get"), { ok: true, value: held }),
    startSignIn: async () => (asked.push("start"), starts()),
    signOut: async (id) => {
      asked.push(`sign out ${id}`);
      held = { ...held, profiles: held.profiles.filter((one) => one.account_id !== id) };
      return { ok: true, value: { accounts: held, store_row_remains: true, said: "The store did not answer." } satisfies SignedOut };
    },
    setRepositoryProfile: async (root, accountId) => {
      asked.push(`bind ${root} ${accountId}`);
      const found = [...held.profiles.flatMap((one) => one.repositories), ...held.unmatched].find((one) => one.root === root);
      if (found === undefined) return { ok: false, outcome: refusal("Fleet does not serve " + root) };
      if (accountId !== null && !held.profiles.some((one) => one.account_id === accountId)) return { ok: false, outcome: refusal("no such account") };
      const { root: at, name } = found;
      held = {
        ...held,
        profiles: held.profiles.map((one) => ({
          ...one,
          repositories: [
            ...one.repositories.filter((bound) => bound.root !== root),
            ...(one.account_id === accountId ? [{ root: at, name, how: "chosen" as const }] : []),
          ],
        })),
        unmatched: [
          ...held.unmatched.filter((loose) => loose.root !== root),
          ...(accountId === null ? [{ root: at, name, candidates: [], none_chosen: true }] : []),
        ],
      };
      return { ok: true, value: held };
    },
    openLink: async (address) => void opened.push(address),
    getSharedHome: async (login) => (asked.push(`home ${login}`), { ok: true, value: login === "nickmele" ? { home: { account: login, repository: "armada-home" }, rows: [], unreadable: [] } : { planned: { account: "mele-labs", repository: "team-home", exists: false }, rows: [], unreadable: [] } }),
    getHomeKinds: async (login) => (asked.push(`kinds ${login}`), { ok: true, value: kindsOf(login) }),
    setHomeKind: async (login, kind, turned) => {
      asked.push(`set ${login} ${kind} ${turned}`);
      const kinds = on.get(login) ?? new Set<HomeKind>();
      if (turned) kinds.add(kind);
      else kinds.delete(kind);
      on.set(login, kinds);
      return { ok: true, value: kindsOf(login) };
    },
    ...over,
  };
  return {
    fleet,
    asked,
    opened,
    /** What Fleet moves to by itself, between two reads. */
    become: (to: Accounts) => void (held = to),
    start: (answer: () => AccountsActed) => void (starts = answer),
    connection: (to: "connected" | "closed") => {
      state = to;
      hear({ connection: { state } });
    },
  };
}

const SIGNED_OUT: Accounts = { profiles: [], unmatched: [], flow: IDLE };
const KINDS: readonly HomeKind[] = ["skills", "plugins", "global_instructions", "sub_agents", "commands", "mcp_servers", "allowlist", "workflows", "memory", "mods"];

describe("signing in from the title menu", () => {
  it("draws nothing before Fleet answers, then offers an account that is signed out", async () => {
    const fleet = fake(SIGNED_OUT);
    const source = createFleetAccount(fleet.fleet, clock().later);
    expect(source.get()).toBeUndefined();
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.profiles).toEqual([]);
    expect(source.get()?.account.flow).toEqual({ phase: "idle" });
    expect(fleet.asked).toEqual(["get"]);
  });

  it("goes from signed out to a profile: the code, the set-up rows, then the profile, reading every two seconds and then no more", async () => {
    const fleet = fake(SIGNED_OUT);
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    source.subscribe(() => undefined);
    await settled();
    expect(time.pending()).toBe(0);

    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT", expiresIn: 899 });
    expect(time.waited()).toEqual([POLL_MS]);

    // Pressing the link opens the address Fleet gave, in the browser.
    source.get()?.account.openDevicePage();
    expect(fleet.opened).toEqual([ADDRESS]);

    // The code is typed; Fleet reads who signed in.
    fleet.become({ profiles: [], unmatched: [], flow: SETUP });
    await time.tick();
    expect(source.get()?.account.flow).toEqual({ phase: "setup", login: "mele-work", repository: "armada-home", steps: SETUP.phase === "setup" ? SETUP.steps : undefined });
    expect(time.pending()).toBe(1);

    fleet.become({ profiles: [work], unmatched: [], flow: { phase: "idle", ended: { outcome: "signed_in" } } });
    await time.tick();
    expect(source.get()?.account.profiles).toEqual([workDrawn]);
    expect(source.get()?.account.flow).toEqual({ phase: "idle" });
    expect(source.get()?.ended).toEqual({ outcome: "signed_in" });
    expect(time.pending()).toBe(0);
    expect(fleet.asked).toEqual(["get", "start", "get", "get"]);
  });

  it("holds a sign-in turned down or run out as how it ended, and stops reading", async () => {
    for (const ended of [
      { outcome: "denied", said: "It was turned down at the forge." },
      { outcome: "expired", said: "The code ran out before it was typed." },
    ] as const) {
      const fleet = fake(SIGNED_OUT);
      const time = clock();
      const source = createFleetAccount(fleet.fleet, time.later);
      source.subscribe(() => undefined);
      await settled();
      source.get()?.account.signIn();
      await settled();

      fleet.become({ profiles: [], unmatched: [], flow: { phase: "idle", ended } });
      await time.tick();
      expect(source.get()?.ended).toEqual(ended);
      expect(source.get()?.account.profiles).toEqual([]);
      expect(source.get()?.account.flow).toEqual({ phase: "idle" });
      expect(time.pending()).toBe(0);
    }
  });

  it("holds Fleet's own words when sign-in is not set up, changes nothing else, and lets a second press try again", async () => {
    const fleet = fake(SIGNED_OUT);
    fleet.start(() => ({ ok: false, outcome: refusal(UNAVAILABLE) }));
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    source.subscribe(() => undefined);
    await settled();

    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.refused).toBe(UNAVAILABLE);
    expect(source.get()?.account.flow).toEqual({ phase: "idle" });
    expect(time.pending()).toBe(0);

    source.get()?.account.dismiss();
    expect(source.get()?.refused).toBeUndefined();

    fleet.start(() => ({ ok: true, value: { profiles: [], unmatched: [], flow: CODE } }));
    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.refused).toBeUndefined();
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT", expiresIn: 899 });
  });

  it("does not begin a second sign-in while one is in flight, or open an address that is not https", async () => {
    const fleet = fake({ profiles: [], unmatched: [], flow: { phase: "code", code: "ABCD-EFGH", address: "http://forge.example/device", expires_in_seconds: 5 } });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    source.get()?.account.signIn();
    source.get()?.account.addProfile();
    await settled();
    expect(fleet.asked).toEqual(["get"]);
    source.get()?.account.openDevicePage();
    expect(fleet.opened).toEqual([]);
  });
});

describe("the profiles", () => {
  it("signs a profile out by its account id, and the profile leaves the list", async () => {
    const fleet = fake({ profiles: [work, personal], unmatched: [], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.profiles).toHaveLength(2);

    const out = await source.signOut(1001);
    expect(fleet.asked).toContain("sign out 1001");
    expect(source.get()?.account.profiles.map((one) => one.login)).toEqual(["nickmele"]);
    expect(out.ok && out.value.store_row_remains).toBe(true);
  });

  it("marks no profile active and asks Fleet nothing when one is switched to, since a repository is bound to a profile and the machine has none that acts", async () => {
    const fleet = fake({ profiles: [work, personal], unmatched: [], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.profiles.map((one) => [one.login, one.active])).toEqual([
      ["mele-work", false],
      ["nickmele", false],
    ]);

    source.get()?.account.switchTo("nickmele");
    await settled();
    expect(fleet.asked).toEqual(["get"]);
  });

  it("hands the composition each profile's repositories and the ones bound to none, with how each came to be", async () => {
    const armada = { root: "/r/armada", name: "armada", how: "matched" as const };
    const site = { root: "/r/site", name: "site", how: "chosen" as const };
    const fleet = fake({
      profiles: [{ ...work, repositories: [armada] }, { ...personal, repositories: [site] }],
      unmatched: [
        { root: "/r/shared", name: "shared", candidates: [1001, 1002], none_chosen: false },
        { root: "/r/scratch", name: "scratch", candidates: [], none_chosen: true },
      ],
      flow: IDLE,
    });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    const account = source.get()?.account;
    expect(account?.profiles.map((one) => [one.login, one.repositories])).toEqual([
      ["mele-work", [{ root: "/r/armada", name: "armada", by: "matched" }]],
      ["nickmele", [{ root: "/r/site", name: "site", by: "chosen" }]],
    ]);
    expect(account?.unbound).toEqual([
      { root: "/r/shared", name: "shared" },
      { root: "/r/scratch", name: "scratch", none: true },
    ]);
  });

  it("draws no unbound repositories when every one is bound", async () => {
    const fleet = fake({ profiles: [work], unmatched: [], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.unbound).toBeUndefined();
  });

  it("binds a repository by its root and a profile by its login, and moves it from one profile to another", async () => {
    const fleet = fake({
      profiles: [{ ...work, repositories: [{ root: "/r/armada", name: "armada", how: "matched" }] }, personal],
      unmatched: [{ root: "/r/shared", name: "shared", candidates: [1001, 1002], none_chosen: false }],
      flow: IDLE,
    });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();

    source.get()?.account.bind?.("/r/shared", "nickmele");
    await settled();
    expect(fleet.asked).toEqual(["get", "bind /r/shared 1002"]);
    expect(source.get()?.account.unbound).toBeUndefined();
    expect(source.get()?.account.profiles.map((one) => one.repositories?.map((bound) => [bound.name, bound.by]))).toEqual([
      [["armada", "matched"]],
      [["shared", "chosen"]],
    ]);

    source.get()?.account.bind?.("/r/armada", "nickmele");
    await settled();
    expect(fleet.asked.at(-1)).toBe("bind /r/armada 1002");
    expect(source.get()?.account.profiles.map((one) => one.repositories?.map((bound) => [bound.name, bound.by]))).toEqual([
      [],
      [["shared", "chosen"], ["armada", "chosen"]],
    ]);
  });

  it("binds a repository to none, and it is then drawn as chosen none", async () => {
    const fleet = fake({
      profiles: [{ ...work, repositories: [{ root: "/r/armada", name: "armada", how: "matched" }] }],
      unmatched: [],
      flow: IDLE,
    });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();

    source.get()?.account.bind?.("/r/armada", null);
    await settled();
    expect(fleet.asked).toEqual(["get", "bind /r/armada null"]);
    expect(source.get()?.account.profiles[0]?.repositories).toEqual([]);
    expect(source.get()?.account.unbound).toEqual([{ root: "/r/armada", name: "armada", none: true }]);
  });

  it("moves the checkout it is told to when two share a folder name", async () => {
    const mine = { root: "/home/a/armada", name: "armada", how: "matched" as const };
    const other = { root: "/home/b/armada", name: "armada", how: "matched" as const };
    const fleet = fake({ profiles: [{ ...work, repositories: [mine, other] }, personal], unmatched: [], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.profiles[0]?.repositories?.map((one) => one.root)).toEqual(["/home/a/armada", "/home/b/armada"]);

    source.get()?.account.bind?.("/home/b/armada", "nickmele");
    await settled();
    expect(fleet.asked).toEqual(["get", "bind /home/b/armada 1002"]);
    expect(source.get()?.account.profiles.map((one) => one.repositories?.map((bound) => bound.root))).toEqual([
      ["/home/a/armada"],
      ["/home/b/armada"],
    ]);

    source.get()?.account.bind?.("/home/a/armada", null);
    await settled();
    expect(source.get()?.account.unbound).toEqual([{ root: "/home/a/armada", name: "armada", none: true }]);
    expect(source.get()?.account.profiles[1]?.repositories?.map((bound) => bound.root)).toEqual(["/home/b/armada"]);
  });

  it("asks Fleet nothing for a login it does not hold, and shows a refusal as it shows the others", async () => {
    const fleet = fake({ profiles: [work], unmatched: [{ root: "/r/shared", name: "shared", candidates: [], none_chosen: false }], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();

    source.get()?.account.bind?.("/r/shared", "nobody");
    await settled();
    expect(fleet.asked).toEqual(["get"]);

    const refused = fake(
      { profiles: [work], unmatched: [{ root: "/r/shared", name: "shared", candidates: [], none_chosen: false }], flow: IDLE },
      { setRepositoryProfile: async () => ({ ok: false, outcome: refusal("This machine has not joined account 1001.") }) },
    );
    const again = createFleetAccount(refused.fleet, clock().later);
    again.subscribe(() => undefined);
    await settled();
    again.get()?.account.bind?.("/r/shared", "mele-work");
    await settled();
    expect(again.get()?.refused).toBe("This machine has not joined account 1001.");
    expect(again.get()?.account.unbound).toEqual([{ root: "/r/shared", name: "shared" }]);
  });

  it("adds a profile by the same sign-in, and shows the profile list again once it is done", async () => {
    const fleet = fake({ profiles: [work], unmatched: [], flow: IDLE });
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    source.subscribe(() => undefined);
    await settled();
    source.get()?.account.addProfile();
    await settled();
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT", expiresIn: 899 });
    fleet.become({ profiles: [work, personal], unmatched: [], flow: IDLE });
    await time.tick();
    expect(source.get()?.account.profiles).toHaveLength(2);
    expect(source.get()?.account.flow).toEqual({ phase: "idle" });
  });
});

describe("when Fleet is not there", () => {
  it("offers no account while Fleet cannot be read", async () => {
    const fleet = fake(SIGNED_OUT, { getAccounts: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()).toBeUndefined();
  });

  it("puts the account away when the connection drops, stops reading, and reads again when it returns", async () => {
    const fleet = fake({ profiles: [], unmatched: [], flow: CODE });
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    let heard = 0;
    source.subscribe(() => void heard++);
    await settled();
    expect(source.get()?.account.flow.phase).toBe("code");
    expect(time.pending()).toBe(1);

    fleet.connection("closed");
    expect(source.get()).toBeUndefined();
    expect(time.pending()).toBe(0);

    fleet.connection("connected");
    await settled();
    expect(source.get()?.account.flow.phase).toBe("code");
    expect(heard).toBe(3);
  });

  it("stops reading when the last window listener leaves", async () => {
    const fleet = fake({ profiles: [], unmatched: [], flow: CODE });
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    const off = source.subscribe(() => undefined);
    await settled();
    expect(time.pending()).toBe(1);
    off();
    expect(time.pending()).toBe(0);
  });

  it("does not let a read that left before an act undo it", async () => {
    const before: Accounts = { profiles: [work, personal], unmatched: [], flow: CODE };
    const fleet = fake(before);
    let release: () => void = () => undefined;
    let reads = 0;
    const time = clock();
    const source = createFleetAccount(
      {
        ...fleet.fleet,
        // The second read, the poll, is slow and answers with the profiles as they were.
        getAccounts: async () => {
          if (++reads === 1) return { ok: true, value: before };
          await new Promise<void>((done) => (release = done));
          return { ok: true, value: before };
        },
      },
      time.later,
    );
    source.subscribe(() => undefined);
    await settled();

    await time.tick();
    await source.signOut(1002);
    expect(source.get()?.account.profiles.map((one) => one.login)).toEqual(["mele-work"]);

    release();
    await settled();
    expect(source.get()?.account.profiles.map((one) => one.login)).toEqual(["mele-work"]);
  });
});

describe("what the account page is told", () => {
  const BLANK_BOTH = "Sign-in is unavailable: the sign-in app and the store address are blank in Settings.";

  /** A source over `start`, subscribed and settled. */
  async function opened(start: Accounts, over: Partial<FleetAccounts> = {}) {
    const fleet = fake(start, over);
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    source.subscribe(() => undefined);
    await settled();
    return { fleet, time, source };
  }

  it("says how a sign-in ended, and puts it away on dismiss until the next one ends differently", async () => {
    const { fleet, time, source } = await opened(SIGNED_OUT);
    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.account.ended).toBeUndefined();

    fleet.become({ profiles: [], unmatched: [], flow: { phase: "idle", ended: { outcome: "denied", said: "It was turned down at the forge." } } });
    await time.tick();
    expect(source.get()?.account.ended).toEqual({ outcome: "denied", said: "It was turned down at the forge." });

    source.get()?.account.dismiss();
    expect(source.get()?.account.ended).toBeUndefined();

    fleet.become({ profiles: [], unmatched: [], flow: { phase: "idle", ended: { outcome: "expired", said: "The code ran out before it was typed." } } });
    source.get()?.account.signIn();
    await settled();
    fleet.become({ profiles: [], unmatched: [], flow: { phase: "idle", ended: { outcome: "unreachable", said: "The forge could not be reached." } } });
    await time.tick();
    expect(source.get()?.account.ended).toEqual({ outcome: "unreachable", said: "The forge could not be reached." });
  });

  it("says nothing of a sign-in that signed in", async () => {
    const { fleet, time, source } = await opened(SIGNED_OUT);
    source.get()?.account.signIn();
    await settled();
    fleet.become({ profiles: [work], unmatched: [], flow: { phase: "idle", ended: { outcome: "signed_in" } } });
    await time.tick();
    expect(source.get()?.account.ended).toBeUndefined();
    expect(source.get()?.account.profiles).toEqual([workDrawn]);
  });

  it("names each blank setting when sign-in is refused for one, and calls any other refusal a refused sign-in", async () => {
    for (const [said, blank] of [
      [BLANK_BOTH, ["account", "store"]],
      ["Sign-in is unavailable: the store address is blank in Settings.", ["store"]],
      ["Sign-in is unavailable: the sign-in app is blank in Settings.", ["account"]],
    ] as const) {
      const fleet = fake(SIGNED_OUT);
      fleet.start(() => ({ ok: false, outcome: refusal(said) }));
      const source = createFleetAccount(fleet.fleet, clock().later);
      source.subscribe(() => undefined);
      await settled();
      source.get()?.account.signIn();
      await settled();
      expect(source.get()?.account.unavailable).toEqual({ said, blank });
      expect(source.get()?.account.ended).toBeUndefined();
    }

    const fleet = fake(SIGNED_OUT);
    fleet.start(() => ({ ok: false, outcome: refusal("The forge would not begin a sign-in.") }));
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.account.unavailable).toBeUndefined();
    expect(source.get()?.account.ended).toEqual({ outcome: "refused", said: "The forge would not begin a sign-in." });
  });

  it("opens Settings at the setting a blank one names", async () => {
    const { source } = await opened(SIGNED_OUT);
    source.get()?.account.openSetting("store");
    expect(openSettingsAt).toHaveBeenLastCalledWith("store", expect.any(Function));
    source.get()?.account.openSetting("account");
    expect(openSettingsAt).toHaveBeenLastCalledWith("account", expect.any(Function));
  });

  it("holds who signed out, and that the store kept its row, until it is put away or the next sign-in begins", async () => {
    const { fleet, source } = await opened({ profiles: [work, personal], unmatched: [], flow: IDLE });
    source.get()?.account.signOut("mele-work");
    await settled();
    expect(fleet.asked).toContain("sign out 1001");
    expect(source.get()?.account.profiles.map((one) => one.login)).toEqual(["nickmele"]);
    expect(source.get()?.account.left).toEqual({ login: "mele-work", storeRowRemains: true, said: "The store did not answer." });

    source.get()?.account.dismiss();
    expect(source.get()?.account.left).toBeUndefined();

    source.get()?.account.signOut("nickmele");
    await settled();
    expect(source.get()?.account.left?.login).toBe("nickmele");
    source.get()?.account.addProfile();
    await settled();
    expect(source.get()?.account.left).toBeUndefined();
  });

  it("says no more than the store's row when the store forgot the device", async () => {
    const { source } = await opened({ profiles: [work], unmatched: [], flow: IDLE }, {
      signOut: async () => ({ ok: true, value: { accounts: { profiles: [], unmatched: [], flow: IDLE }, store_row_remains: false } satisfies SignedOut }),
    });
    source.get()?.account.signOut("mele-work");
    await settled();
    expect(source.get()?.account.left).toEqual({ login: "mele-work", storeRowRemains: false });
  });
});

describe("what syncs", () => {
  const signedIn = async (over: Partial<FleetAccounts> = {}, hooks: Parameters<typeof createFleetAccount>[2] = {}) => {
    const fleet = fake({ profiles: [work, personal], unmatched: [], flow: IDLE }, over);
    const source = createFleetAccount(fleet.fleet, clock().later, hooks);
    source.subscribe(() => undefined);
    await settled();
    return { fleet, source };
  };

  it("opens on a profile's kinds as Fleet keeps them, every one off, and its home made where this machine holds a copy", async () => {
    const { fleet, source } = await signedIn();
    expect(source.get()?.account.syncs).toBeNull();
    source.get()?.account.choose("nickmele");
    await settled();
    const syncs = source.get()?.account.syncs;
    expect(fleet.asked).toEqual(["get", "kinds nickmele", "home nickmele"]);
    expect(syncs?.login).toBe("nickmele");
    expect(syncs?.kinds.map((one) => one.kind)).toEqual(KINDS);
    expect(syncs?.kinds.every((one) => !one.on && !one.pulling && one.items.length === 0)).toBe(true);
    expect(syncs?.home).toEqual({ account: "nickmele", repository: "armada-home", made: true });

    source.get()?.account.closeSyncs();
    expect(source.get()?.account.syncs).toBeNull();
    source.get()?.account.choose("mele-work");
    await settled();
    expect(source.get()?.account.syncs?.home).toEqual({ account: "mele-labs", repository: "team-home", made: false });
  });

  it("names each profile's organisations and home as Fleet does, the repository once a copy is here, and whether the drawn home is made", async () => {
    const { source } = await signedIn();
    const [mine, theirs] = source.get()?.account.profiles ?? [];
    // `signedIn` lists the work profile first and the chosen personal one second.
    expect([mine?.login, mine?.orgs, mine?.home, mine?.repository]).toEqual(["mele-work", [], { account: "mele-work", repository: "armada-home", made: false }, undefined]);
    expect([theirs?.login, theirs?.orgs, theirs?.home, theirs?.repository]).toEqual(["nickmele", ["mele-labs"], { account: "mele-labs", repository: "team-home", made: true }, "team-home"]);
    expect(source.get()?.account.home).toBe(true);
  });

  it("draws the chosen profile's home as not made while it has no copy here", async () => {
    const fleet = fake({ profiles: [work], unmatched: [], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.home).toBe(false);
  });

  it("reads the profiles again when asked, since a home was chosen or made", async () => {
    const { fleet, source } = await signedIn();
    source.refresh?.();
    await settled();
    expect(fleet.asked.filter((one) => one === "get")).toHaveLength(2);
  });

  it("switches a kind for the profile it is open on, draws it at once, and says so to the home", async () => {
    const switched: string[] = [];
    let answer: (() => void) | undefined;
    const { fleet, source } = await signedIn({}, { kindSwitched: (login) => switched.push(login) });
    const real = fleet.fleet.setHomeKind;
    fleet.fleet.setHomeKind = (login, kind, on) => new Promise((done) => (answer = () => void real(login, kind, on).then(done)));
    source.get()?.account.choose("nickmele");
    await settled();
    source.get()?.account.setKind("mods", true);
    expect(source.get()?.account.syncs?.kinds.find((one) => one.kind === "mods")?.on).toBe(true);
    expect(switched).toEqual([]);
    answer?.();
    await settled();
    expect(fleet.asked).toContain("set nickmele mods true");
    expect(source.get()?.account.syncs?.kinds.find((one) => one.kind === "mods")?.on).toBe(true);
    expect(switched).toEqual(["nickmele"]);
  });

  it("puts a switch Fleet refused back where it stood, and says why as the page says the others", async () => {
    const { source } = await signedIn({ setHomeKind: async () => ({ ok: false, outcome: refusal("What syncs could not be read: the store is locked") }) });
    source.get()?.account.choose("nickmele");
    await settled();
    source.get()?.account.setKind("skills", true);
    await settled();
    expect(source.get()?.account.syncs?.kinds.find((one) => one.kind === "skills")?.on).toBe(false);
    expect(source.get()?.account.ended).toEqual({ outcome: "refused", said: "What syncs could not be read: the store is locked" });
  });
});
