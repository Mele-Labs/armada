// The Fleet-backed account source, against a fake Fleet and a clock turned by hand: signing in from
// nothing to a profile by reading Fleet while the flow is not idle, how a sign-in that did not finish
// is held, Fleet's refusal to begin one, signing out and switching the active profile. What the code
// card and the set-up rows look like is the composition's (`Account.stories.tsx`).

import { describe, expect, it } from "vitest";

import type { AccountFlow, Accounts, Outcome, Profile, SignedOut } from "@armada/protocol";

import type { AccountsActed } from "../../shared/api/accounts";
import { createFleetAccount, POLL_MS, type FleetAccounts, type Later } from "./fleet-account";

const settled = () => new Promise((done) => setTimeout(done, 0));

const ADDRESS = "https://forge.example/login/device";
const work: Profile = { account_id: 1001, login: "mele-work", active: true, chosen: false };
const personal: Profile = { account_id: 1002, login: "nickmele", active: false, chosen: true };
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
  const fleet: FleetAccounts = {
    state: async () => ({ connection: { state } }) as never,
    subscribe: (on) => ((hear = on as never), () => undefined),
    getAccounts: async () => (asked.push("get"), { ok: true, value: held }),
    startSignIn: async () => (asked.push("start"), starts()),
    signOut: async (id) => {
      asked.push(`sign out ${id}`);
      held = { ...held, profiles: held.profiles.filter((one) => one.account_id !== id).map((one, at) => ({ ...one, active: at === 0 })) };
      return { ok: true, value: { accounts: held, store_row_remains: true, said: "The store did not answer." } satisfies SignedOut };
    },
    setActiveAccount: async (id) => {
      asked.push(`active ${id}`);
      return { ok: true, value: (held = { ...held, profiles: held.profiles.map((one) => ({ ...one, active: one.account_id === id })) }) };
    },
    openLink: async (address) => void opened.push(address),
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

const SIGNED_OUT: Accounts = { profiles: [], flow: IDLE };

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
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT" });
    expect(time.waited()).toEqual([POLL_MS]);

    // Pressing the link opens the address Fleet gave, in the browser.
    source.get()?.account.openDevicePage();
    expect(fleet.opened).toEqual([ADDRESS]);

    // The code is typed; Fleet reads who signed in.
    fleet.become({ profiles: [], flow: SETUP });
    await time.tick();
    expect(source.get()?.account.flow).toEqual({ phase: "setup", login: "mele-work", repository: "armada-home", steps: SETUP.phase === "setup" ? SETUP.steps : undefined });
    expect(time.pending()).toBe(1);

    fleet.become({ profiles: [work], flow: { phase: "idle", ended: { outcome: "signed_in" } } });
    await time.tick();
    expect(source.get()?.account.profiles).toEqual([work]);
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

      fleet.become({ profiles: [], flow: { phase: "idle", ended } });
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

    fleet.start(() => ({ ok: true, value: { profiles: [], flow: CODE } }));
    source.get()?.account.signIn();
    await settled();
    expect(source.get()?.refused).toBeUndefined();
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT" });
  });

  it("does not begin a second sign-in while one is in flight, or open an address that is not https", async () => {
    const fleet = fake({ profiles: [], flow: { phase: "code", code: "ABCD-EFGH", address: "http://forge.example/device", expires_in_seconds: 5 } });
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
    const fleet = fake({ profiles: [work, personal], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();
    expect(source.get()?.account.profiles).toHaveLength(2);

    const out = await source.signOut(1001);
    expect(fleet.asked).toContain("sign out 1001");
    expect(source.get()?.account.profiles.map((one) => one.login)).toEqual(["nickmele"]);
    // The first one left becomes active, as Fleet answers.
    expect(source.get()?.account.profiles[0]?.active).toBe(true);
    expect(out.ok && out.value.store_row_remains).toBe(true);
  });

  it("switches the active profile through its account id, and does nothing for the one that is active", async () => {
    const fleet = fake({ profiles: [work, personal], flow: IDLE });
    const source = createFleetAccount(fleet.fleet, clock().later);
    source.subscribe(() => undefined);
    await settled();

    source.get()?.account.switchTo("mele-work");
    await settled();
    expect(fleet.asked).toEqual(["get"]);

    source.get()?.account.switchTo("nickmele");
    await settled();
    expect(fleet.asked).toEqual(["get", "active 1002"]);
    expect(source.get()?.account.profiles.map((one) => [one.login, one.active])).toEqual([
      ["mele-work", false],
      ["nickmele", true],
    ]);
  });

  it("adds a profile by the same sign-in, and shows the profile list again once it is done", async () => {
    const fleet = fake({ profiles: [work], flow: IDLE });
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    source.subscribe(() => undefined);
    await settled();
    source.get()?.account.addProfile();
    await settled();
    expect(source.get()?.account.flow).toEqual({ phase: "code", code: "WDJB-MJHT" });
    fleet.become({ profiles: [work, personal], flow: IDLE });
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
    const fleet = fake({ profiles: [], flow: CODE });
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
    const fleet = fake({ profiles: [], flow: CODE });
    const time = clock();
    const source = createFleetAccount(fleet.fleet, time.later);
    const off = source.subscribe(() => undefined);
    await settled();
    expect(time.pending()).toBe(1);
    off();
    expect(time.pending()).toBe(0);
  });

  it("does not let a read that left before an act undo it", async () => {
    const before: Accounts = { profiles: [work, personal], flow: CODE };
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
    source.get()?.account.switchTo("nickmele");
    await settled();
    expect(source.get()?.account.profiles.find((one) => one.active)?.login).toBe("nickmele");

    release();
    await settled();
    expect(source.get()?.account.profiles.find((one) => one.active)?.login).toBe("nickmele");
  });
});
