// What the capture window may load — #1294, `docs/practices/capture-window.md`.
//
// The subject is the three tests a window opens through: the link is one the
// live holder carries, the Run is still serving, and the address is a process
// on this machine.

import { describe, expect, it } from "vitest";

import type { ServerState } from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../../shared/bridge";
import { chromeAgent, granted, isPinned, LOCAL_NETWORK, loopbackOrigin, onNamedOrigin, webOrigin, offerable, onAnyOrigin, onOrigin, partitionFor, passThrough, pinned, shown } from "./address";

const SERVING: ServerState = {
  id: "01SERVER",
  name: "web_dev",
  manifest_id: "01MANIFEST",
  checkout: { path: "/repos/armada", commit: "cf4bcaea", behind: 0 },
  phase: "serving",
  serve: "pnpm dev --port 41207",
  ports: [{ name: "web", port: 41207 }],
  links: [
    { url: "http://localhost:41207", name: "The app" },
    { url: "http://127.0.0.1:41207/admin" },
  ],
  started_by: "person",
  started_at: "2026-09-18T10:00:00Z",
  stopped: false,
  log: "servers/01SERVER.log",
};

const holding = (...servers: ServerState[]): BridgeState => ({
  ...NOTHING_YET,
  servers: { servers },
});

describe("what the capture window may load", () => {
  it("resolves one of the live holder's own links, and nothing a caller composed", () => {
    const answer = pinned(holding(SERVING), "01SERVER", "http://localhost:41207");
    expect(isPinned(answer)).toBe(true);
    expect(answer).toMatchObject({
      run: "01SERVER",
      name: "web_dev",
      origin: "http://localhost:41207",
      url: "http://localhost:41207",
      manifestId: "01MANIFEST",
    });
  });

  it("refuses an address that server does not publish, however plausible", () => {
    // The port is the Run's and the path is not: the test is on the link, whole.
    expect(pinned(holding(SERVING), "01SERVER", "http://localhost:41207/secret")).toEqual({
      ok: false,
      why: "no_address",
    });
  });

  it("refuses a server nothing is holding", () => {
    expect(pinned(holding(), "01SERVER", "http://localhost:41207")).toEqual({ ok: false, why: "no_address" });
  });

  it("refuses a Run that has stopped serving, because a port is not an identity", () => {
    const exited = { ...SERVING, phase: "exited", stopped: true };
    expect(pinned(holding(exited), "01SERVER", "http://localhost:41207")).toEqual({
      ok: false,
      why: "not_serving",
    });
  });

  it("names the address when a Manifest declares a server that is not on this machine", () => {
    const remote: ServerState = { ...SERVING, links: [{ url: "https://staging.example.com" }] };
    expect(pinned(holding(remote), "01SERVER", "https://staging.example.com")).toEqual({
      ok: false,
      why: "not_loopback",
      address: "https://staging.example.com",
    });
  });
});

describe("a loopback origin", () => {
  it.each([
    ["http://127.0.0.1:41207", "http://127.0.0.1:41207"],
    ["http://localhost:3000/some/path?q=1", "http://localhost:3000"],
    ["https://localhost:8443", "https://localhost:8443"],
    ["http://[::1]:5173", "http://[::1]:5173"],
  ])("takes %s", (address, origin) => {
    expect(loopbackOrigin(address)).toBe(origin);
  });

  it.each([
    // The host is matched whole: nothing here is a prefix or a suffix test.
    "http://127.0.0.1.example.com",
    "http://localhost.evil.test",
    "http://notlocalhost",
    "http://192.168.1.4:3000",
    "file:///tmp/index.html",
    "data:text/html,<b>hi</b>",
    "about:blank",
    "javascript:alert(1)",
    "not an address",
  ])("refuses %s", (address) => {
    expect(loopbackOrigin(address)).toBeNull();
  });
});

describe("staying on the origin", () => {
  const origin = "http://localhost:41207";

  it("takes a path, a query and a hash on the same origin", () => {
    expect(onOrigin(origin, "http://localhost:41207/checkout?step=2#pay")).toBe(true);
  });

  it.each([
    // Scheme, host and port together — a different one of the three is off it.
    ["a different port", "http://localhost:41208/"],
    ["a different scheme", "https://localhost:41207/"],
    ["a different host on the same port", "http://127.0.0.1:41207/"],
    ["a host this one is a prefix of", "http://localhost:41207.example.com/"],
    ["an auth provider", "https://accounts.google.com/o/oauth2/v2/auth"],
    ["a scheme with no origin", "about:blank"],
    ["something unparseable", "http://[bad"],
  ])("refuses %s", (_what, address) => {
    expect(onOrigin(origin, address)).toBe(false);
  });
});

describe("what a refusal offers", () => {
  it("offers the system browser the web and nothing else", () => {
    expect(offerable("https://accounts.google.com/x")).toBe(true);
    expect(offerable("http://example.test")).toBe(true);
    expect(offerable("mailto:someone@example.test")).toBe(false);
    expect(offerable("file:///etc/passwd")).toBe(false);
    expect(offerable("bank://transfer")).toBe(false);
  });
});

describe("the session partition", () => {
  it("is persistent and one per repository, so two repositories share nothing", () => {
    expect(partitionFor("01MANIFEST")).toBe("persist:armada-capture-01MANIFEST");
    expect(partitionFor("01MANIFEST")).not.toBe(partitionFor("01OTHER"));
    expect(partitionFor(null)).toBe("persist:armada-capture-unnamed");
  });
});

describe("a web origin", () => {
  it.each([
    ["https://example.com/a?b=1", "https://example.com"],
    ["http://localhost:3000/x", "http://localhost:3000"],
  ])("reads %s", (address, origin) => {
    expect(webOrigin(address)).toBe(origin);
  });

  it.each(["file:///etc/passwd", "javascript:alert(1)", "ftp://example.com/", "not an address", "http://"])("refuses %s", (address) => {
    expect(webOrigin(address)).toBeNull();
  });
});

describe("what the bar says of a page", () => {
  const page = { run: "session:s1", origin: "http://localhost:47342", url: "http://localhost:47342/?walk=a", manifestId: null, through: [] };

  it("names the page by its title and the origin it is pinned to", () => {
    expect(shown({ ...page, name: "Store clock report" })).toEqual({ name: "Store clock report", address: "http://localhost:47342" });
  });

  it("drops the name where the window was titled by its address, and draws the address whole", () => {
    expect(shown({ ...page, name: page.url })).toEqual({ name: null, address: page.url });
  });
});

describe("the origins a Session lets its window pass through", () => {
  const page = {
    run: "session:s1",
    name: "Staging",
    origin: "https://staging.example.com",
    url: "https://staging.example.com/app",
    manifestId: "01MANIFEST",
    through: ["https://login.example.org", "http://localhost:8443"],
  };

  it("allows the shown origin and each named one, whole, and refuses every other", () => {
    for (const address of ["https://staging.example.com/x", "https://login.example.org/sso?next=%2F", "http://localhost:8443/cb"]) {
      expect(onAnyOrigin(page, address), address).toBe(true);
    }
    for (const address of [
      "https://login.example.org.evil.example.net/sso",
      "https://evil.example.net/login.example.org",
      "http://login.example.org/sso",
      "https://login.example.org:8443/sso",
      "https://example.org/",
      "javascript:alert(1)",
      "about:blank",
      "not an address",
    ]) {
      expect(onAnyOrigin(page, address), address).toBe(false);
    }
  });

  it("allows only the shown origin where none are named", () => {
    expect(onAnyOrigin({ ...page, through: [] }, "https://login.example.org/sso")).toBe(false);
    expect(onAnyOrigin({ ...page, through: [] }, "https://staging.example.com/x")).toBe(true);
  });

  it("keeps an entry only if it is a bare http or https origin, and reads it as the engine does", () => {
    expect(passThrough(["https://login.example.org", "HTTP://Localhost:8443/", "https://sso.example.net:443"])).toEqual([
      "https://login.example.org",
      "http://localhost:8443",
      "https://sso.example.net",
    ]);
    expect(
      passThrough([
        "https://login.example.org/sso",
        "https://login.example.org?next=1",
        "https://login.example.org#x",
        "https://user:pass@login.example.org",
        "ftp://login.example.org",
        "file:///etc/passwd",
        "login.example.org",
        "",
      ]),
    ).toEqual([]);
  });
});

describe("the user agent a provider reads", () => {
  const ELECTRON =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Armada/0.1.0 Chrome/144.0.7559.236 Electron/40.10.6 Safari/537.36";

  it("is Electron's with its own two tokens taken out, and Chrome's left", () => {
    expect(chromeAgent(ELECTRON)).toBe(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.7559.236 Safari/537.36",
    );
  });

  it("takes out an app name of any spelling, and an Electron token wherever it sits", () => {
    expect(chromeAgent(ELECTRON.replace("Armada/0.1.0", "Electron/1.0"))).not.toContain("Electron");
    expect(chromeAgent(ELECTRON.replace("Armada/0.1.0", "Armada-Dev/9.9.9"))).not.toContain("Armada");
  });

  it("returns an agent that is already Chrome's as it came", () => {
    const chrome = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.7559.236 Safari/537.36";
    expect(chromeAgent(chrome)).toBe(chrome);
  });
});

describe("a page on a provider the Session named", () => {
  const pin = { run: "session:s", name: "Staging", origin: "https://app.example.com", url: "https://app.example.com/", manifestId: null, through: ["https://login.example.org"] };

  it("is on a named origin and not on the shown one", () => {
    expect(onNamedOrigin(pin, "https://login.example.org/authorize?x=1")).toBe(true);
    expect(onNamedOrigin(pin, "https://app.example.com/callback")).toBe(false);
  });

  it("is on none for an address on neither, and for one that does not parse", () => {
    expect(onNamedOrigin(pin, "https://login.example.org.evil.net/")).toBe(false);
    expect(onNamedOrigin(pin, "http://login.example.org/")).toBe(false);
    expect(onNamedOrigin(pin, "")).toBe(false);
  });
});

describe("the one permission a walk's page is given", () => {
  const reach = (address: string): boolean => address.startsWith("https://login.example.com");

  it("is local network access, on an origin the window may reach", () => {
    expect(LOCAL_NETWORK).toBe("local-network-access");
    expect(granted(LOCAL_NETWORK, "https://login.example.com/sso", reach)).toBe(true);
  });

  it("is not given on an origin the window may not reach", () => {
    expect(granted(LOCAL_NETWORK, "https://elsewhere.example.com", reach)).toBe(false);
  });

  it("is not any other permission, on any origin", () => {
    for (const permission of ["media", "notifications", "clipboard-read", "geolocation", "unknown"]) {
      expect(granted(permission, "https://login.example.com", reach), permission).toBe(false);
    }
  });
});
