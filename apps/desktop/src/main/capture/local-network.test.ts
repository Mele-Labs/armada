import { describe, expect, it, vi } from "vitest";

import { reachTest, spaceOfAddress, spacesThrough, type AddressSpace } from "./local-network";
import type { Decider } from "./permissions";

const NAMES: Record<string, string[]> = {
  "site.example": [
    "93.184.216.34",
  ],
  "rebound.example": [
    "93.184.216.34",
    "127.0.0.1",
  ],
  "router.lan": [
    "192.168.1.1",
  ],
};

const spaceOf = spacesThrough(async (name) => {
  const addresses = NAMES[name];
  if (addresses === undefined) throw new Error("ERR_NAME_NOT_RESOLVED");
  return {
    endpoints: addresses.map((address) => ({
      address,
    })),
  };
});

function setup(answer: Promise<boolean> | boolean = false) {
  const request = vi.fn(() => Promise.resolve(answer));
  const decider = {
    request,
  } as unknown as Decider;
  const test = reachTest(() => decider, spaceOf);
  const from = (origin: string, url: string, resourceType = "xhr") =>
    test({
      url,
      resourceType,
      frame: {
        origin,
        parent: null,
      },
      webContents: {
        id: 1,
        getURL: () => origin,
      },
    });
  return {
    from,
    test,
    request,
  };
}

describe("the space of an address", () => {
  it.each<
    [
      string,
      AddressSpace,
    ]
  >([
    [
      "127.0.0.1",
      "loopback",
    ],
    [
      "127.8.8.8",
      "loopback",
    ],
    [
      "0.0.0.0",
      "loopback",
    ],
    [
      "::1",
      "loopback",
    ],
    [
      "::ffff:127.0.0.1",
      "loopback",
    ],
    [
      "10.1.2.3",
      "local",
    ],
    [
      "100.64.0.1",
      "local",
    ],
    [
      "169.254.169.254",
      "local",
    ],
    [
      "172.16.0.1",
      "local",
    ],
    [
      "172.31.255.255",
      "local",
    ],
    [
      "192.168.68.98",
      "local",
    ],
    [
      "fd00::1",
      "local",
    ],
    [
      "fe80::1",
      "local",
    ],
    [
      "::ffff:192.168.0.1",
      "local",
    ],
    [
      "172.32.0.1",
      "public",
    ],
    [
      "93.184.216.34",
      "public",
    ],
    [
      "2606:4700::1",
      "public",
    ],
  ])("%s is %s", (address, space) => {
    expect(spaceOfAddress(address)).toBe(space);
  });
});

describe("the space of a host", () => {
  it.each<
    [
      string,
      AddressSpace,
    ]
  >([
    [
      "localhost",
      "loopback",
    ],
    [
      "app.localhost",
      "loopback",
    ],
    [
      "[::1]",
      "loopback",
    ],
    [
      "printer.local",
      "local",
    ],
    [
      "site.example",
      "public",
    ],
    [
      "router.lan",
      "local",
    ],
    [
      "nothing.example",
      "public",
    ],
  ])("%s is %s", async (host, space) => {
    expect(await spaceOf(host)).toBe(space);
  });

  it("takes the most private answer a name gives, so a site's name for 127.0.0.1 is still this Mac", async () => {
    expect(await spaceOf("rebound.example")).toBe("loopback");
  });

  it("looks a name up again once its minute is over, so a changed answer is seen", async () => {
    let now = 0;
    const resolve = vi.fn(async () => ({
      endpoints: [
        {
          address: "93.184.216.34",
        },
      ],
    }));
    const spaces = spacesThrough(resolve, () => now);
    await spaces("site.example");
    await spaces("site.example");
    expect(resolve).toHaveBeenCalledTimes(1);
    now = 60_001;
    await spaces("site.example");
    expect(resolve).toHaveBeenCalledTimes(2);
  });
});

describe("what a Job browser's page reaches", () => {
  it.each([
    [
      "a public page, this Mac",
      "https://site.example",
      "http://127.0.0.1:5482/ping",
    ],
    [
      "a public page, the network",
      "https://site.example",
      "http://192.168.1.1/",
    ],
    [
      "a public page, a name for this Mac",
      "https://site.example",
      "http://rebound.example/",
    ],
    [
      "a public page, a WebSocket to this Mac",
      "https://site.example",
      "ws://localhost:5482/",
    ],
    [
      "a page on the network, this Mac",
      "http://192.168.68.98:5481",
      "http://127.0.0.1:5482/ping",
    ],
  ])("asks the row for %s, naming the page that asked", async (_, origin, url) => {
    const { from, request } = setup(false);
    expect(await from(origin, url)).toBe(false);
    expect(request).toHaveBeenCalledWith("local-network-access", origin, {});
  });

  it("lets the request through when the row answers yes", async () => {
    const { from } = setup(true);
    expect(await from("https://site.example", "http://127.0.0.1:5482/ping")).toBe(true);
  });

  it.each([
    [
      "a public page, the web",
      "https://site.example",
      "https://site.example/app.js",
    ],
    [
      "a page on this Mac, the network",
      "http://localhost:5173",
      "http://192.168.1.1/",
    ],
    [
      "a page on this Mac, this Mac",
      "http://localhost:5173",
      "http://127.0.0.1:5482/ping",
    ],
    [
      "a page on the network, the network",
      "http://192.168.68.98:5481",
      "http://192.168.1.1/",
    ],
    [
      "a page on the network, the web",
      "http://192.168.68.98:5481",
      "https://site.example/",
    ],
    [
      "a public page, a data address",
      "https://site.example",
      "data:text/plain,hello",
    ],
  ])("does not ask for %s", async (_, origin, url) => {
    const { from, request } = setup(false);
    expect(await from(origin, url)).toBe(true);
    expect(request).not.toHaveBeenCalled();
  });

  it("does not ask when the person opens an address in the page itself", async () => {
    const { from, request } = setup(false);
    expect(await from("https://site.example", "http://192.168.1.1/", "mainFrame")).toBe(true);
    expect(request).not.toHaveBeenCalled();
  });

  it("holds an iframe's address to its parent, whatever the frame shows now", async () => {
    const { test, request } = setup(false);
    const allowed = await test({
      url: "http://127.0.0.1:5482/",
      resourceType: "subFrame",
      frame: {
        origin: "http://localhost:5482",
        parent: {
          origin: "https://site.example",
        },
      },
      webContents: {
        id: 1,
        getURL: () => "https://site.example/",
      },
    });
    expect(allowed).toBe(false);
    expect(request).toHaveBeenCalledWith("local-network-access", "https://site.example", {});
  });

  it("holds a frame with no origin of its own to its page", async () => {
    const { test, request } = setup(false);
    const allowed = await test({
      url: "http://127.0.0.1:5482/",
      resourceType: "xhr",
      frame: {
        origin: "null",
        parent: null,
      },
      webContents: {
        id: 1,
        getURL: () => "https://site.example/page",
      },
    });
    expect(allowed).toBe(false);
    expect(request).toHaveBeenCalledWith("local-network-access", "https://site.example", {});
  });

  it("asks once for requests made while the question is open, and gives them all the answer", async () => {
    let settle!: (answer: boolean) => void;
    const { from, request } = setup(
      new Promise<boolean>((resolve) => {
        settle = resolve;
      }),
    );
    const both = Promise.all([
      from("https://site.example", "http://127.0.0.1:5482/one"),
      from("https://site.example", "http://127.0.0.1:5482/two"),
    ]);
    await vi.waitFor(() => expect(request).toHaveBeenCalled());
    settle(true);
    expect(await both).toEqual([
      true,
      true,
    ]);
    expect(request).toHaveBeenCalledTimes(1);
  });

  it("asks again after the answer, so a site whose row has moved gets the row's new answer", async () => {
    const { from, request } = setup(false);
    await from("https://site.example", "http://127.0.0.1:5482/");
    await from("https://site.example", "http://127.0.0.1:5482/");
    expect(request).toHaveBeenCalledTimes(2);
  });

  it("leaves a page with no decider alone: the walk window keeps its own rules", async () => {
    const test = reachTest(() => undefined, spaceOf);
    const allowed = await test({
      url: "http://127.0.0.1:5482/",
      resourceType: "xhr",
      frame: {
        origin: "https://site.example",
        parent: null,
      },
      webContents: {
        id: 1,
        getURL: () => "https://site.example/",
      },
    });
    expect(allowed).toBe(true);
  });
});
