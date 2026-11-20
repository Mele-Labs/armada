// @vitest-environment happy-dom
// Where the phone keeps its Macs and its store sign-in: IndexedDB, so a non-extractable key survives; and in the fixture, memory,
// so a walk on macOS WebKit never reaches for the keychain's "WebCrypto Master Key". IndexedDB is a fake that records what opens it.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Machine, StoreSignIn } from "./machines";

type Fake = {
  opened: number;
  /** What each object store holds, by its key. */
  stores: Map<string, Map<unknown, unknown>>;
};

const key = {} as CryptoKey;
const STUDIO: Machine = {
  origin: "https://studio.tail0.ts.net",
  name: "studio",
  deviceId: "phone-on-studio",
  key,
};
const LAPTOP: Machine = {
  origin: "https://laptop.tail0.ts.net",
  name: "laptop",
  deviceId: "phone-on-laptop",
  key,
};
const SIGN_IN: StoreSignIn = {
  store: "https://store.example",
  deviceId: "phone-on-store",
  key,
};

let fake: Fake;

/** An IndexedDB with the four calls `machines.ts` makes, each answering on a later tick as a request does. */
function install() {
  fake = {
    opened: 0,
    stores: new Map(),
  };
  const after = <T>(settle: () => T) => {
    const request: {
      result?: T;
      onsuccess?: () => void;
    } = {};
    queueMicrotask(() => {
      request.result = settle();
      request.onsuccess?.();
    });
    return request;
  };
  const storeOf = (name: string) => {
    const held = fake.stores.get(name) ?? new Map<unknown, unknown>();
    fake.stores.set(name, held);
    return {
      get: (at: unknown) => after(() => held.get(at)),
      getAll: () =>
        after(() => [
          ...held.values(),
        ]),
      put: (value: unknown, at?: unknown) =>
        after(() => {
          const where = at ?? (value as Record<string, unknown>)["origin"];
          held.set(where, value);
          return where;
        }),
      delete: (at: unknown) => after(() => held.delete(at)),
      clear: () => after(() => held.clear()),
    };
  };
  vi.stubGlobal("indexedDB", {
    open: () => {
      fake.opened += 1;
      const request: {
        result: unknown;
        onupgradeneeded?: () => void;
        onsuccess?: () => void;
      } = {
        result: undefined,
      };
      const names = new Set<string>();
      request.result = {
        objectStoreNames: {
          contains: (name: string) => names.has(name),
        },
        createObjectStore: (name: string) => void names.add(name),
        transaction: () => ({
          objectStore: storeOf,
        }),
        close: () => undefined,
      };
      queueMicrotask(() => {
        request.onupgradeneeded?.();
        request.onsuccess?.();
      });
      return request;
    },
  });
}

/** `machines.ts` read afresh, because it decides where it keeps things once, as it loads. */
async function loaded(mode: string) {
  vi.resetModules();
  vi.stubEnv("MODE", mode);
  return import("./machines");
}

beforeEach(install);

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("where the phone keeps its Macs", () => {
  it("keeps them in memory in the fixture, in the order they were paired, and never opens IndexedDB", async () => {
    const kept = await loaded("fixture");
    expect(await kept.machines()).toEqual([]);
    await kept.keepMachine(STUDIO);
    await kept.keepMachine(LAPTOP);
    expect((await kept.machines()).map((one) => one.name)).toEqual([
      "studio",
      "laptop",
    ]);
    // Pairing a Mac again replaces it and keeps its place.
    await kept.keepMachine({
      ...STUDIO,
      name: "studio-2",
    });
    expect((await kept.machines()).map((one) => one.name)).toEqual([
      "studio-2",
      "laptop",
    ]);
    await kept.forgetMachine(STUDIO.origin);
    expect((await kept.machines()).map((one) => one.name)).toEqual([
      "laptop",
    ]);
    expect(fake.opened).toBe(0);
  });

  it("keeps the store sign-in in memory in the fixture, and forgets it", async () => {
    const kept = await loaded("fixture");
    expect(await kept.storeSignIn()).toBeUndefined();
    await kept.keepStoreSignIn(SIGN_IN);
    expect(await kept.storeSignIn()).toEqual(SIGN_IN);
    await kept.forgetStoreSignIn();
    expect(await kept.storeSignIn()).toBeUndefined();
    expect(fake.opened).toBe(0);
  });

  it("does not carry the fixture's Macs into a phone that keeps them", async () => {
    const fixture = await loaded("fixture");
    await fixture.keepMachine(STUDIO);
    const phone = await loaded("production");
    expect(await phone.machines()).toEqual([]);
  });

  it("keeps them in IndexedDB otherwise, in the machines store, and the sign-in in its own", async () => {
    const kept = await loaded("production");
    await kept.keepMachine(STUDIO);
    await kept.keepMachine(LAPTOP);
    await kept.keepStoreSignIn(SIGN_IN);
    expect(fake.opened).toBeGreaterThan(0);
    expect([
      ...(fake.stores.get("machines")?.keys() ?? []),
    ]).toEqual([
      STUDIO.origin,
      LAPTOP.origin,
    ]);
    expect(fake.stores.get("store_sign_in")?.get("kept")).toEqual(SIGN_IN);
    expect((await kept.machines()).map((one) => one.name)).toEqual([
      "studio",
      "laptop",
    ]);
    expect(await kept.storeSignIn()).toEqual(SIGN_IN);
    await kept.forgetMachine(STUDIO.origin);
    await kept.forgetStoreSignIn();
    expect((await kept.machines()).map((one) => one.name)).toEqual([
      "laptop",
    ]);
    expect(await kept.storeSignIn()).toBeUndefined();
  });
});
