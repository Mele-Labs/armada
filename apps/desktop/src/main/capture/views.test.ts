// The session a walk's page and popup run in — `docs/practices/capture-window.md`, *Origins a Session names*.
// Electron is faked: what is asserted is the agent the partition sends and the grants it withholds.

import { beforeEach, expect, it, vi } from "vitest";

type Headers = Record<string, string>;
type BeforeSend = (details: { requestHeaders: Headers }, done: (answer: { requestHeaders: Headers }) => void) => void;

const world = vi.hoisted(() => ({
  partitions: new Map<string, Record<string, unknown>>(),
  views: [] as { webPreferences: Record<string, unknown> }[],
}));

vi.mock("electron", () => {
  class WebContentsView {
    constructor(public options: { webPreferences: Record<string, unknown> }) {
      world.views.push({ webPreferences: options.webPreferences });
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

import { pageView } from "./views";

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
  (part["permission"] as (a: unknown, b: unknown, decide: (granted: boolean) => void) => void)(null, "media", (granted) => decided.push(granted));
  expect(decided).toEqual([false]);
  expect((part["check"] as () => boolean)()).toBe(false);
  expect(world.views[0]?.webPreferences).toMatchObject({ sandbox: true, contextIsolation: true, nodeIntegration: false });
  expect(world.views[0]?.webPreferences).not.toHaveProperty("preload");
});
