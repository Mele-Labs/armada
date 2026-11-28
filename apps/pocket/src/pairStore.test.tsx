// @vitest-environment happy-dom
// Pairing through the owner's claims store, as the Pair screen plays it: Sign in with GitHub, his Macs, one press to pair.
// The store, GitHub's code and the Macs' `/pair/vouched` are a fetch stub that records what the phone sends; the phone's own
// store key is a real one, so a signature is checked by verifying it, never by comparing it.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Machine, StoreSignIn } from "./machines";

const STORE = "https://store.example";
const CLIENT = "Iv1.test";
const MINI = "https://mini.tail0.ts.net";
const STUDIO = "https://studio.tail0.ts.net";
const CODE = {
  device_code: "dc-1",
  user_code: "WDJB-MJHT",
  verification_uri: "https://github.com/login/device",
  interval: 5,
  expires_in: 900,
};

const peer = (machine: string, address: string, kind = "mac") => ({
  device: `7001.${machine}`,
  machine,
  kind,
  address,
  public_key: "AAAA",
});
const PEERS = [
  peer("studio", STUDIO),
  peer("mini", MINI),
  peer("attic", ""),
  peer("tablet", "https://tablet.tail0.ts.net", "phone"),
];

type Sent = {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string;
};
type Reply = {
  status: number;
  body?: unknown;
};

// What the stub answers, and what the screen holds beside it; reset in `beforeEach`.
const held = vi.hoisted(() => ({
  signed: undefined as StoreSignIn | undefined,
  kept: [] as StoreSignIn[],
  paired: [] as Machine[],
  added: [] as Machine[],
  pushed: [] as Machine[][],
}));
let sent: Sent[];
let theStore: Reply;
let signIn: Reply;
let polls: Reply[];
let peers: Reply;
let vouches: Reply[];
let phone: CryptoKeyPair;
let root: Root | undefined;
let host: HTMLElement;
const realTimeout = globalThis.setTimeout;
const realSleep = (ms: number) => new Promise<void>((resolve) => realTimeout(resolve, ms));

vi.mock("./machines", () => ({
  machines: async () => [],
  keepMachine: async () => undefined,
  forgetMachine: async () => undefined,
  storeSignIn: async () => held.signed,
  keepStoreSignIn: async (kept: StoreSignIn) => void held.kept.push(kept),
  forgetStoreSignIn: async () => {
    held.signed = undefined;
  },
}));
vi.mock("./fleet", () => ({
  macOf: () => undefined,
  useFleet: () => ({
    macs: held.paired.map((machine) => ({
      machine,
      paired: true,
      needs: [],
      live: [],
    })),
  }),
  addMac: async (machine: Machine) => void held.added.push(machine),
}));
vi.mock("./push", () => ({
  subscribe: async (all: Machine[]) => void held.pushed.push(all),
}));

const answered = ({ status, body }: Reply) =>
  new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json",
    },
  });

async function world() {
  vi.resetModules();
  held.signed = undefined;
  held.kept = [];
  held.paired = [];
  held.added = [];
  held.pushed = [];
  sent = [];
  theStore = {
    status: 200,
    body: {
      store: `${STORE}/`,
      client_id: CLIENT,
    },
  };
  signIn = {
    status: 200,
    body: CODE,
  };
  polls = [];
  peers = {
    status: 200,
    body: {
      devices: PEERS,
    },
  };
  vouches = [];
  phone = (await crypto.subtle.generateKey(
    {
      name: "ECDSA",
      namedCurve: "P-256",
    },
    false,
    [
      "sign",
      "verify",
    ],
  )) as CryptoKeyPair;
  vi.stubGlobal(
    "fetch",
    async (
      url: string,
      init: {
        method?: string;
        body?: string;
        headers?: Record<string, string>;
      } = {},
    ) => {
      const at = new URL(url);
      const method = init.method ?? "GET";
      sent.push({
        method,
        url,
        headers: init.headers ?? {},
        body: init.body ?? "",
      });
      if (at.pathname === "/pair/store" && at.origin === window.location.origin)
        return answered(theStore);
      if (at.origin === STORE && at.pathname === "/v1/sign_in") return answered(signIn);
      if (at.origin === STORE && at.pathname === "/v1/sign_in/poll")
        return answered(
          polls.shift() ?? {
            status: 202,
            body: {
              waiting: "pending",
            },
          },
        );
      if (at.origin === STORE && at.pathname === "/v1/peers") return answered(peers);
      if (at.origin === MINI && at.pathname === "/pair/vouched")
        return answered(
          vouches.shift() ?? {
            status: 200,
            body: {
              device_id: "fixture-mini",
            },
          },
        );
      if (at.origin === MINI && at.pathname === "/api/machine")
        return answered({
          status: 200,
          body: {
            name: "mini",
          },
        });
      throw new TypeError("Failed to fetch");
    },
  );
}

async function openPair() {
  window.history.replaceState(null, "", "/pair");
  const { Pair } = await import("./Pair");
  host = document.createElement("div");
  document.body.replaceChildren(host);
  await act(async () => {
    root = createRoot(host);
    root.render(<Pair code="" />);
  });
}

/** Signs the phone in as it is after a sign in, before the screen opens. */
function signedIn() {
  held.signed = {
    store: STORE,
    deviceId: "7001.phone",
    key: phone.privateKey,
  };
}

const text = () => host.textContent ?? "";
const button = (name: string) =>
  Array.from(host.querySelectorAll("button")).find(
    (one) => one.textContent?.trim() === name || one.getAttribute("aria-label") === name,
  );
const press = async (element: Element | null | undefined) => {
  expect(element).toBeTruthy();
  await act(async () => (element as HTMLElement).click());
};
const until = async (ok: () => boolean) => {
  for (let waited = 0; waited < 400 && !ok(); waited++) await act(async () => realSleep(5));
  expect(ok()).toBe(true);
};
const callsTo = (path: string) => sent.filter((one) => new URL(one.url).pathname === path);
const macRows = () =>
  Array.from(
    host.querySelectorAll('[aria-label="Your Macs"] ul:not([aria-label="Loading"]) li'),
  ).map((one) => one.textContent);

/** Whether `call` carried a signature made by the phone's store key over the store's signed text. */
async function verifies(one: Sent): Promise<boolean> {
  const at = new URL(one.url);
  const hash = Array.from(
    new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(one.body))),
    (byte) => byte.toString(16).padStart(2, "0"),
  ).join("");
  const text =
    one.method +
    at.pathname +
    at.search +
    one.headers["X-Armada-Time"] +
    one.headers["X-Armada-Nonce"] +
    hash;
  return crypto.subtle.verify(
    {
      name: "ECDSA",
      hash: "SHA-256",
    },
    phone.publicKey,
    Uint8Array.from(one.headers["X-Armada-Signature"]!.match(/../g)!.map((b) => parseInt(b, 16))),
    new TextEncoder().encode(text),
  );
}

beforeEach(async () => {
  (
    globalThis as {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
  await world();
});

afterEach(async () => {
  await act(async () => root?.unmount());
  root = undefined;
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("Pair, through the store", {
  timeout: 60_000,
}, () => {
  describe("signing in with GitHub", () => {
    it("asks the Mac for the store, begins a sign in on it and shows GitHub's code", async () => {
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      expect(callsTo("/pair/store")).toHaveLength(1);
      await press(button("Sign in with GitHub"));
      await until(() => text().includes(CODE.user_code));
      const [begun] = callsTo("/v1/sign_in");
      expect(begun!.url).toBe(`${STORE}/v1/sign_in`);
      expect(JSON.parse(begun!.body)).toEqual({
        client_id: CLIENT,
      });
      const open = Array.from(host.querySelectorAll("a")).find(
        (one) => one.textContent === "Open GitHub",
      );
      expect(open?.getAttribute("href")).toBe(CODE.verification_uri);
      expect(text()).toContain("Waiting for GitHub");
      const waiting = host.querySelector('[role="status"].pk-signin__wait');
      expect(waiting?.classList.contains("pk-ring")).toBe(true);
      expect(waiting?.querySelector("svg")).toBeNull();
      expect(host.querySelector(".pk-spin")).toBeNull();
    });

    it("polls at the code's interval through pending, and takes the interval slow_down gives", async () => {
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      vi.useFakeTimers({
        toFake: [
          "setTimeout",
          "clearTimeout",
        ],
      });
      polls.push(
        {
          status: 202,
          body: {
            waiting: "pending",
          },
        },
        {
          status: 202,
          body: {
            waiting: "slow_down",
            interval: 15,
          },
        },
      );
      await press(button("Sign in with GitHub"));
      await until(() => text().includes(CODE.user_code));
      const advance = (seconds: number) =>
        act(async () => {
          await vi.advanceTimersByTimeAsync(seconds * 1000);
        });
      const polled = (times: number) => until(() => callsTo("/v1/sign_in/poll").length === times);

      await advance(4.9);
      await realSleep(30);
      expect(callsTo("/v1/sign_in/poll")).toHaveLength(0);
      await advance(0.1);
      await polled(1);
      // Pending leaves the wait as it was: 5 s on from the first poll.
      await advance(5);
      await polled(2);
      // slow_down gave 15 s, so 5 s on is too soon and 15 s on is the next poll.
      await advance(14.9);
      await realSleep(30);
      expect(callsTo("/v1/sign_in/poll")).toHaveLength(2);
      await advance(0.1);
      await polled(3);
      const [first] = callsTo("/v1/sign_in/poll");
      expect(JSON.parse(first!.body)).toMatchObject({
        client_id: CLIENT,
        device_code: CODE.device_code,
        machine: "Phone",
      });
      // The key the store is told of is the SPKI of a P-256 key: 91 bytes, base64.
      expect(atob(JSON.parse(first!.body).public_key)).toHaveLength(91);
    });

    it("signs in on a 200: keeps the store and the device it answers with, and lists his Macs", async () => {
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      vi.useFakeTimers({
        toFake: [
          "setTimeout",
          "clearTimeout",
        ],
      });
      polls.push({
        status: 200,
        body: {
          device: "7001.phone",
        },
      });
      await press(button("Sign in with GitHub"));
      await until(() => text().includes(CODE.user_code));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(CODE.interval * 1000);
      });
      await until(() => held.kept.length === 1);
      expect(held.kept[0]).toMatchObject({
        store: STORE,
        deviceId: "7001.phone",
      });
      // The key that was kept is the phone's own, private, and cannot be read out of the phone.
      expect(held.kept[0]!.key).toMatchObject({
        type: "private",
        extractable: false,
      });
      await until(() => callsTo("/v1/peers").length === 1);
      await until(() => macRows().length === 2);
      expect(text()).not.toContain("Waiting for GitHub");
    });

    it.each([
      [
        "expired",
        410,
        "The code expired. Sign in again.",
      ],
      [
        "denied",
        403,
        "GitHub denied the sign in. Sign in again.",
      ],
    ])("offers to start again once the sign in is %s", async (_name, status, says) => {
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      vi.useFakeTimers({
        toFake: [
          "setTimeout",
          "clearTimeout",
        ],
      });
      polls.push({
        status,
        body: {
          error: _name,
        },
      });
      await press(button("Sign in with GitHub"));
      await until(() => text().includes(CODE.user_code));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(CODE.interval * 1000);
      });
      await until(() => text().includes(says));
      expect(held.kept).toEqual([]);
      expect(text()).not.toContain(CODE.user_code);
      expect(button("Sign in with GitHub")).toBeUndefined();
      await press(button("Sign in again"));
      await until(() => callsTo("/v1/sign_in").length === 2);
      await until(() => text().includes(CODE.user_code));
    });

    it("says device sign in must be turned on when GitHub refuses with device_flow_disabled", async () => {
      signIn = {
        status: 400,
        body: {
          error: "forge_refused",
          reason: "device_flow_disabled",
        },
      };
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      await press(button("Sign in with GitHub"));
      await until(() => text().includes("turned on"));
      expect(text()).toContain("device sign in");
      expect(text()).not.toContain(CODE.user_code);
      expect(button("Sign in again")).toBeTruthy();
    });

    it("says it too when the refusal comes on a poll", async () => {
      await openPair();
      await until(() => button("Sign in with GitHub") !== undefined);
      vi.useFakeTimers({
        toFake: [
          "setTimeout",
          "clearTimeout",
        ],
      });
      polls.push({
        status: 400,
        body: {
          error: "forge_refused",
          reason: "device_flow_disabled",
        },
      });
      await press(button("Sign in with GitHub"));
      await until(() => text().includes(CODE.user_code));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(CODE.interval * 1000);
      });
      await until(() => text().includes("turned on"));
      expect(button("Sign in again")).toBeTruthy();
    });

    it("shows only the pairing link where the Mac has no store to name", async () => {
      theStore = {
        status: 200,
        body: {
          store: null,
          client_id: null,
        },
      };
      await openPair();
      await until(() => text().includes("is not set up on this Mac"));
      expect(button("Sign in with GitHub")).toBeUndefined();
      expect(callsTo("/v1/sign_in")).toEqual([]);
      expect(host.querySelector('input[aria-label="Pairing link"]')).toBeTruthy();
      expect(host.querySelector('[aria-label="With a pairing link"]')).toBeTruthy();
    });
  });

  describe("his Macs", () => {
    it("lists only the Macs the store has an address for, and marks one already paired", async () => {
      signedIn();
      held.paired = [
        {
          origin: STUDIO,
          name: "studio",
          deviceId: "d",
          key: phone.privateKey,
        },
      ];
      await openPair();
      await until(() => macRows().length === 2);
      expect(macRows().join(" ")).toContain("studio");
      expect(macRows().join(" ")).toContain("mini");
      expect(text()).not.toContain("attic");
      expect(text()).not.toContain("tablet");
      expect(button("studio paired")?.hasAttribute("disabled")).toBe(true);
      expect(button("studio paired")?.textContent).toBe("Paired");
      expect(button("Pair mini")?.hasAttribute("disabled")).toBe(false);
      expect(button("Pair studio")).toBeUndefined();
      // It asked the store who he has, signed with his own key, and never signed in again.
      expect(callsTo("/v1/sign_in")).toEqual([]);
      const [listed] = callsTo("/v1/peers");
      expect(await verifies(listed!)).toBe(true);
    });
  });

  describe("pairing with one", () => {
    it("sends the Mac a request signed with the store key, naming the store, and keeps the Mac on a 200", async () => {
      signedIn();
      held.paired = [
        {
          origin: STUDIO,
          name: "studio",
          deviceId: "d",
          key: phone.privateKey,
        },
      ];
      await openPair();
      await until(() => button("Pair mini") !== undefined);
      await press(button("Pair mini"));
      await until(() => held.added.length === 1);
      const [asked] = callsTo("/pair/vouched");
      expect(asked!.method).toBe("POST");
      expect(asked!.url).toBe(`${MINI}/pair/vouched`);
      expect(asked!.headers["X-Armada-Store"]).toBe(STORE);
      expect(asked!.headers["X-Armada-Device"]).toBe("7001.phone");
      expect(asked!.headers["X-Armada-Time"]).toMatch(/^\d+$/);
      expect(asked!.headers["X-Armada-Nonce"]).toMatch(/^[0-9a-f]{32}$/);
      expect(asked!.headers["X-Armada-Signature"]).toMatch(/^[0-9a-f]{128}$/);
      expect(await verifies(asked!)).toBe(true);
      // The body is the code flow's claim: the phone's name and the SPKI of a fresh key for this Mac.
      expect(JSON.parse(asked!.body)).toEqual({
        name: "Phone",
        spki_hex: expect.stringMatching(/^[0-9a-f]{182}$/),
      });
      // The Mac is kept as the code flow keeps it: named by itself, with the device id it answered and a key of its own.
      expect(held.added[0]).toMatchObject({
        origin: MINI,
        name: "mini",
        deviceId: "fixture-mini",
      });
      expect(held.added[0]!.key).not.toBe(phone.privateKey);
      expect(held.added[0]!.key).toMatchObject({
        type: "private",
        extractable: false,
      });
      expect(callsTo("/api/machine")).toHaveLength(1);
      // And the phone's push subscription goes to it with the Macs it already held.
      await until(() => held.pushed.length === 1);
      expect(held.pushed[0]!.map((one) => one.origin)).toEqual([
        STUDIO,
        MINI,
      ]);
    });

    it("shows a 403 not_vouched as a refusal on that Mac's row, keeps nothing, and tries again on request", async () => {
      signedIn();
      vouches.push({
        status: 403,
        body: {
          error: "not_vouched",
        },
      });
      await openPair();
      await until(() => button("Pair mini") !== undefined);
      await press(button("Pair mini"));
      await until(() => button("Try again with mini") !== undefined);
      expect(text()).toContain("did not find this phone in your store");
      expect(button("Try again with mini")?.textContent).toBe("Try again");
      expect(held.added).toEqual([]);
      expect(callsTo("/api/machine")).toEqual([]);
      await press(button("Try again with mini"));
      await until(() => held.added.length === 1);
      expect(callsTo("/pair/vouched")).toHaveLength(2);
      // Each press is its own request: a nonce is never sent twice.
      const [one, two] = callsTo("/pair/vouched");
      expect(one!.headers["X-Armada-Nonce"]).not.toBe(two!.headers["X-Armada-Nonce"]);
      expect(text()).not.toContain("did not find this phone in your store");
    });
  });
});
