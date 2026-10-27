// The session a walk's page and popup run in — `docs/practices/capture-window.md`, *Origins a Session names*.
// Electron is faked: what is asserted is the agent the partition sends and the grants it withholds.

import { beforeEach, expect, it, vi } from "vitest";

type Headers = Record<string, string>;
type BeforeSend = (details: { requestHeaders: Headers }, done: (answer: { requestHeaders: Headers }) => void) => void;

type FakeContents = { id: number; on: (event: string, fn: () => void) => void; fire: (event: string) => void };

const world = vi.hoisted(() => ({
  partitions: new Map<string, Record<string, unknown>>(),
  views: [] as { webPreferences: Record<string, unknown>; background?: string; webContents: FakeContents }[],
  next: 0,
}));

vi.mock("electron", () => {
  class WebContentsView {
    webContents: FakeContents;
    constructor(public options: { webPreferences: Record<string, unknown> }) {
      const handlers = new Map<string, (() => void)[]>();
      const webContents = {
        id: ++world.next,
        on: (event: string, fn: () => void) => void handlers.set(event, [...(handlers.get(event) ?? []), fn]),
        fire: (event: string) => (handlers.get(event) ?? []).forEach((fn) => fn()),
      };
      world.views.push({ webPreferences: options.webPreferences, webContents });
      this.webContents = webContents;
    }
    setBackgroundColor(colour: string): void {
      world.views.at(-1)!.background = colour;
    }
  }
  const session = {
    fromPartition: (name: string) => {
      const existing = world.partitions.get(name);
      if (existing !== undefined) return existing;
      const part: Record<string, unknown> = {
        name,
        agent: null as string | null,
        beforeSend: null as BeforeSend | null,
        setUserAgent: (agent: string) => void (part["agent"] = agent),
        webRequest: { onBeforeSendHeaders: (handler: BeforeSend) => void (part["beforeSend"] = handler) },
        setPermissionRequestHandler: (handler: unknown) => void (part["permission"] = handler),
        setPermissionCheckHandler: (handler: unknown) => void (part["check"] = handler),
        setDisplayMediaRequestHandler: () => {},
        on: () => {},
      };
      world.partitions.set(name, part);
      return part;
    },
  };
  return {
    app: {
      userAgentFallback:
        "Mozilla/5.0 (Macintosh) AppleWebKit/537.36 (KHTML, like Gecko) Armada/0.1.0 Chrome/144.0.0.0 Electron/40.10.6 Safari/537.36",
    },
    session,
    WebContentsView,
  };
});

import tokens from "@armada/tokens/tokens.json";

import type { Pinned } from "./address";
import { pageView as view } from "./views";

const pin = (manifestId: string | null, through: string[] = []): Pinned => ({
  run: "run-1",
  name: "Example",
  origin: "https://app.example.com",
  url: "https://app.example.com/",
  manifestId,
  through,
});
const pageView = (manifestId: string | null, through: string[] = []) => view(pin(manifestId, through));

const CHROME = "Mozilla/5.0 (Macintosh) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

beforeEach(() => {
  world.views = [];
});

it("sends Chrome's agent from the partition, for the page's own string and the header", () => {
  pageView("01ARMADA");
  const part = world.partitions.get("persist:armada-capture-01ARMADA")!;
  expect(part["agent"]).toBe(CHROME);

  let sent: Headers | null = null;
  (part["beforeSend"] as BeforeSend)(
    { requestHeaders: { "User-Agent": "Mozilla/5.0 (Macintosh) Chrome/144.0.0.0 Electron/40.10.6 Safari/537.36", Accept: "text/html" } },
    (answer) => void (sent = answer.requestHeaders),
  );
  expect(sent).toEqual({ "User-Agent": CHROME, Accept: "text/html" });
});

it("sets it once for a partition two windows share, and on a partition of its own for another repository", () => {
  pageView("01ARMADA");
  const part = world.partitions.get("persist:armada-capture-01ARMADA")!;
  const first = part["beforeSend"];
  pageView("01ARMADA");
  expect(part["beforeSend"]).toBe(first);
  pageView("01WIDGETS");
  expect(world.partitions.get("persist:armada-capture-01WIDGETS")?.["agent"]).toBe(CHROME);
});

it("withholds every permission, and gives the page no preload", () => {
  pageView(null);
  const part = world.partitions.get("persist:armada-capture-unnamed")!;
  const decided: boolean[] = [];
  (part["permission"] as Ask)(null, "media", (granted) => decided.push(granted), { requestingUrl: "https://app.example.com/" });
  expect(decided).toEqual([false]);
  expect((part["check"] as Check)(null, "media", "https://app.example.com")).toBe(false);
  expect(world.views[0]?.webPreferences).toMatchObject({ sandbox: true, contextIsolation: true, nodeIntegration: false });
  expect(world.views[0]?.webPreferences).not.toHaveProperty("preload");
});

it("paints the page Bridge's ground until it draws, never white", () => {
  pageView(null);
  expect(world.views[0]?.background).toBe(tokens.tokens.find((one) => one.name === "--bg-base")?.value);
  expect(world.views[0]?.background).toMatch(/^#[0-9A-Fa-f]{6}$/);
});

type Ask = (contents: unknown, permission: string, decide: (granted: boolean) => void, details: { requestingUrl: string }) => void;
type Check = (contents: unknown, permission: string, requestingOrigin: string) => boolean;

const LOCAL = "local-network-access";
const THROUGH = ["https://login.example.com"];

function asked(part: Record<string, unknown>, contents: unknown, permission: string, requestingUrl: string): boolean {
  const decided: boolean[] = [];
  (part["permission"] as Ask)(contents, permission, (granted) => decided.push(granted), { requestingUrl });
  return decided[0]!;
}

function walkPage() {
  pageView("01ARMADA", THROUGH);
  return { part: world.partitions.get("persist:armada-capture-01ARMADA")!, contents: world.views.at(-1)!.webContents };
}

it("gives a sign-in provider's page, and the page's own, local network access", () => {
  const { part, contents } = walkPage();
  expect(asked(part, contents, LOCAL, "https://login.example.com/sso")).toBe(true);
  expect(asked(part, contents, LOCAL, "https://app.example.com/")).toBe(true);
  expect((part["check"] as Check)(contents, LOCAL, "https://login.example.com")).toBe(true);
});

it("gives local network access to no origin the window may not reach", () => {
  const { part, contents } = walkPage();
  expect(asked(part, contents, LOCAL, "https://elsewhere.example.com/")).toBe(false);
  expect((part["check"] as Check)(contents, LOCAL, "https://elsewhere.example.com")).toBe(false);
});

it("gives no other permission, even to the origin the Session named", () => {
  const { part, contents } = walkPage();
  for (const permission of ["media", "notifications", "clipboard-read"]) {
    expect(asked(part, contents, permission, "https://login.example.com/sso"), permission).toBe(false);
    expect((part["check"] as Check)(contents, permission, "https://login.example.com"), permission).toBe(false);
  }
});

it("gives nothing to contents the window did not register, or to a check with none", () => {
  const { part } = walkPage();
  expect(asked(part, { id: 9999 }, LOCAL, "https://login.example.com/sso")).toBe(false);
  expect((part["check"] as Check)({ id: 9999 }, LOCAL, "https://login.example.com")).toBe(false);
  expect((part["check"] as Check)(null, LOCAL, "https://login.example.com")).toBe(false);
});

it("takes the grant back when the contents are destroyed", () => {
  const { part, contents } = walkPage();
  contents.fire("destroyed");
  expect(asked(part, contents, LOCAL, "https://login.example.com/sso")).toBe(false);
  expect((part["check"] as Check)(contents, LOCAL, "https://login.example.com")).toBe(false);
});
