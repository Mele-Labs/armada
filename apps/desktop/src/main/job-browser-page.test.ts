// What a Job browser's page does in Electron, with Electron faked: the popup it opens, the fullscreen it enters, the permission it is
// asked for and the dialogs it may draw. What is asserted is which handler answers what, and where the view ends up.

import { beforeEach, describe, expect, it, vi } from "vitest";

type Handler = (...args: never[]) => unknown;
type Box = {
  x: number;
  y: number;
  width: number;
  height: number;
};

const world = vi.hoisted(() => {
  let next = 0;
  const fakeContents = () => {
    const handlers = new Map<string, Handler[]>();
    const state = {
      url: "https://login.example.com/start",
      loads: [] as string[],
      closed: 0,
      destroyed: false,
      opener: null as Handler | null,
    };
    return {
      id: (next += 1),
      state,
      on: (name: string, handler: Handler) =>
        void handlers.set(name, [
          ...(handlers.get(name) ?? []),
          handler,
        ]),
      off: () => {},
      emit: (name: string, ...args: unknown[]) =>
        (handlers.get(name) ?? []).forEach((handler) =>
          (handler as (...rest: unknown[]) => unknown)(...args),
        ),
      setBackgroundThrottling: () => {},
      setWindowOpenHandler: (handler: Handler) => void (state.opener = handler),
      getURL: () => state.url,
      isDestroyed: () => state.destroyed,
      loadURL: async (url: string) => void state.loads.push(url),
      close: () => {
        state.closed += 1;
        state.destroyed = true;
      },
    };
  };
  type Contents = ReturnType<typeof fakeContents>;
  type Hold = {
    parent: unknown;
    allowed: (address: string) => boolean;
    refuse: (address: string, what: string) => void;
    closed: () => void;
  };
  return {
    fakeContents,
    views: [] as {
      options: unknown;
      contents: Contents;
      bounds: {
        x: number;
        y: number;
        width: number;
        height: number;
      }[];
    }[],
    popups: [] as {
      how: string;
      contents: Contents;
      closed: number;
    }[],
    holds: [] as Hold[],
  };
});

vi.mock("electron", () => ({
  BrowserWindow: class {},
  session: {},
  app: {},
  WebContentsView: class {},
}));

vi.mock("./capture/views", () => ({
  pageView: (_pin: unknown, options?: unknown) => {
    const view = {
      options,
      contents: world.fakeContents(),
      bounds: [] as Box[],
    };
    world.views.push(view);
    return {
      webContents: view.contents,
      setVisible: () => {},
      setBounds: (box: Box) => void view.bounds.push(box),
      getBounds: () =>
        view.bounds.at(-1) ?? {
          x: 0,
          y: 0,
          width: 1,
          height: 1,
        },
    };
  },
}));

vi.mock("./capture/popup", () => {
  const popupFor = (
    how: string,
    hold: (typeof world.holds)[number],
    contents: (typeof world.popups)[number]["contents"],
  ) => {
    world.holds.push(hold);
    const popup = {
      how,
      contents,
      closed: 0,
    };
    world.popups.push(popup);
    return {
      contents,
      close: () => {
        popup.closed += 1;
        hold.closed();
      },
    };
  };
  return {
    openPopup: (
      hold: (typeof world.holds)[number],
      options: {
        webContents: (typeof world.popups)[number]["contents"];
      },
    ) => popupFor("opened by the page", hold, options.webContents),
    openPopupOn: (
      hold: (typeof world.holds)[number],
      view: {
        webContents: (typeof world.popups)[number]["contents"];
      },
    ) => popupFor("opened for the owner", hold, view.webContents),
  };
});

import type { Answer, SitePermission } from "../shared/site-permissions";
import { deciderOf } from "./capture/permissions";
import { type BrowserPage, makeBrowserPage, type PageTold } from "./job-browsers";
import { SiteSettingsStore } from "./site-permission-store";

/** A window with a content area that can be resized, which the page's view is a child of. */
function fakeWindow(width = 1200, height = 800) {
  const resized: (() => void)[] = [];
  const window = {
    size: {
      width,
      height,
    },
    destroyed: false,
    isDestroyed: () => window.destroyed,
    getContentBounds: () => ({
      x: 0,
      y: 0,
      ...window.size,
    }),
    contentView: {
      addChildView: () => {},
      removeChildView: () => {},
    },
    webContents: {
      focus: () => {},
    },
    on: (name: string, handler: () => void) => void (name === "resize" && resized.push(handler)),
    off: (name: string, handler: () => void) =>
      void (name === "resize" && resized.splice(resized.indexOf(handler), 1)),
    resizeTo: (next: { width: number; height: number }) => {
      window.size = next;
      for (const handler of [
        ...resized,
      ])
        handler();
    },
    resizeHandlers: resized,
  };
  return window;
}

type Saved = [
  string,
  SitePermission,
  "allow" | "block" | "ask",
];

function page(
  over: { window?: ReturnType<typeof fakeWindow> | null; answer?: Answer; saved?: Saved[] } = {},
) {
  const window = over.window === undefined ? fakeWindow() : over.window;
  const store = new SiteSettingsStore({
    read: () => null,
    write: () => {},
  });
  for (const [host, permission, choice] of over.saved ?? [])
    store.choose("repo", host, permission, choice);
  const asked: [
    SitePermission,
    string,
  ][] = [];
  const told: PageTold = {
    changed: () => {},
    chord: () => {},
    ask: async (permission, host) => {
      asked.push([
        permission,
        host,
      ]);
      return over.answer ?? "dismiss";
    },
  };
  const made: BrowserPage = makeBrowserPage("repo", () => window as never, told, store);
  const view = world.views.at(-1)!;
  return {
    made,
    view,
    contents: view.contents,
    window: window as ReturnType<typeof fakeWindow>,
    asked,
    store,
  };
}

type Opened = {
  action: string;
  createWindow?: (options: { webContents: unknown }) => unknown;
};
const opener = (one: ReturnType<typeof page>, url: string) =>
  (one.contents.state.opener as unknown as (details: { url: string }) => Opened)({
    url,
  });
const tick = () => new Promise((resolve) => setTimeout(resolve));

beforeEach(() => {
  world.views = [];
  world.popups = [];
  world.holds = [];
});

describe("the page's own view", () => {
  it("may draw alert and confirm, which the walk window never does", () => {
    page();
    expect(world.views[0]!.options).toMatchObject({
      disableDialogs: false,
    });
  });
});

describe("a popup the page opens", () => {
  it("is a real window by default, made by Electron so the page keeps its opener, and loads nothing into the page itself", () => {
    const one = page();
    const answer = opener(one, "https://login.example.org/authorize");
    expect(answer.action).toBe("allow");
    const made = world.fakeContents();
    expect(
      answer.createWindow!({
        webContents: made,
      }),
    ).toBe(made);
    expect(world.popups).toHaveLength(1);
    expect(one.contents.state.loads).toEqual([]);
  });

  it("holds the popup to the web, and lets it follow the web", () => {
    const one = page();
    opener(one, "https://login.example.org/authorize").createWindow!({
      webContents: world.fakeContents(),
    });
    const { allowed } = world.holds[0]!;
    expect(allowed("https://anywhere.example.net/next")).toBe(true);
    expect(allowed("http://localhost:3000/cb")).toBe(true);
    expect(allowed("file:///etc/hosts")).toBe(false);
    expect(allowed("javascript:alert(1)")).toBe(false);
  });

  it("answers permissions from the saved choices as the page does, for the host it asks on", async () => {
    const one = page({
      saved: [
        [
          "login.example.org",
          "camera",
          "block",
        ],
        [
          "login.example.org",
          "location",
          "allow",
        ],
      ],
    });
    const made = world.fakeContents();
    opener(one, "https://login.example.org/authorize").createWindow!({
      webContents: made,
    });
    const decider = deciderOf(made as never)!;
    expect(
      await decider.request("media", "https://login.example.org/", {
        mediaTypes: [
          "video",
        ],
      }),
    ).toBe(false);
    expect(await decider.request("geolocation", "https://login.example.org/", {})).toBe(true);
    // Local network is Ask by default: not given by the window's origin test, and not by the synchronous check.
    expect(decider.check("local-network-access", "https://login.example.org", {})).toBe(false);
  });

  it("is denied where the opener's host blocks popups, and nothing opens", () => {
    const one = page({
      saved: [
        [
          "login.example.com",
          "popups",
          "block",
        ],
      ],
    });
    expect(opener(one, "https://login.example.org/authorize")).toEqual({
      action: "deny",
    });
    expect(world.popups).toEqual([]);
  });

  it("is denied for an address that is not the web, whatever the choice", () => {
    const one = page();
    for (const url of [
      "file:///etc/hosts",
      "javascript:alert(1)",
      "about:blank",
      "data:text/html,hi",
      "armada://x",
    ])
      expect(opener(one, url)).toEqual({
        action: "deny",
      });
    expect(world.popups).toEqual([]);
  });

  it("is denied now where the choice is Ask, asks, and on Allow opens the address in a real window", async () => {
    const one = page({
      answer: "allow",
      saved: [
        [
          "login.example.com",
          "popups",
          "ask",
        ],
      ],
    });
    expect(opener(one, "https://login.example.org/authorize")).toEqual({
      action: "deny",
    });
    await tick();
    expect(one.asked).toEqual([
      [
        "popups",
        "login.example.com",
      ],
    ]);
    expect(world.popups).toHaveLength(1);
    expect(world.popups[0]!.how).toBe("opened for the owner");
    expect(world.popups[0]!.contents.state.loads).toEqual([
      "https://login.example.org/authorize",
    ]);
    expect(one.store.settingsOf("repo").sites["login.example.com"]).toEqual({
      popups: "allow",
    });
    // A view of its own with the page's dialogs, answering permissions as a page does.
    expect(world.views.at(-1)!.options).toMatchObject({
      disableDialogs: false,
    });
    expect(deciderOf(world.popups[0]!.contents as never)).toBeDefined();
  });

  it("opens nothing where the owner blocks the ask, and keeps nothing where he dismisses it", async () => {
    const blocked = page({
      answer: "block",
      saved: [
        [
          "login.example.com",
          "popups",
          "ask",
        ],
      ],
    });
    opener(blocked, "https://login.example.org/authorize");
    await tick();
    expect(world.popups).toEqual([]);
    expect(blocked.store.settingsOf("repo").sites["login.example.com"]).toEqual({
      popups: "block",
    });
    const dismissed = page({
      answer: "dismiss",
      saved: [
        [
          "login.example.com",
          "popups",
          "ask",
        ],
      ],
    });
    opener(dismissed, "https://login.example.org/authorize");
    await tick();
    expect(world.popups).toEqual([]);
    expect(dismissed.store.settingsOf("repo").sites["login.example.com"]).toEqual({
      popups: "ask",
    });
  });

  it("is denied where there is no window to put it in", () => {
    const one = page({
      window: null,
    });
    expect(opener(one, "https://login.example.org/authorize")).toEqual({
      action: "deny",
    });
  });

  it("closes with the page's tab, and not twice", () => {
    const one = page();
    opener(one, "https://a.example.org/").createWindow!({
      webContents: world.fakeContents(),
    });
    opener(one, "https://b.example.org/").createWindow!({
      webContents: world.fakeContents(),
    });
    one.made.destroy();
    expect(world.popups.map((popup) => popup.closed)).toEqual([
      1,
      1,
    ]);
  });

  it("is let go of once it closes itself, so the tab closing does not close it again", () => {
    const one = page();
    opener(one, "https://a.example.org/").createWindow!({
      webContents: world.fakeContents(),
    });
    world.holds[0]!.closed();
    one.made.destroy();
    expect(world.popups[0]!.closed).toBe(0);
  });
});

describe("fullscreen", () => {
  const BOX = {
    x: 100,
    y: 50,
    width: 600,
    height: 400,
  };

  it("fills the window it is in while the page is fullscreen, and returns to the pane's rect after", () => {
    const one = page();
    one.made.placeIn(one.window, BOX);
    expect(one.view.bounds.at(-1)).toEqual(BOX);
    one.contents.emit("enter-html-full-screen");
    expect(one.view.bounds.at(-1)).toEqual({
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
    });
    one.contents.emit("leave-html-full-screen");
    expect(one.view.bounds.at(-1)).toEqual(BOX);
  });

  it("keeps filling the window as it is resized, and ignores the pane's rect until it leaves", () => {
    const one = page();
    one.made.placeIn(one.window, BOX);
    one.contents.emit("enter-html-full-screen");
    one.window.resizeTo({
      width: 1440,
      height: 900,
    });
    const filled = {
      x: 0,
      y: 0,
      width: 1440,
      height: 900,
    };
    expect(one.view.bounds.at(-1)).toEqual(filled);
    one.made.placeIn(one.window, {
      ...BOX,
      width: 700,
    });
    expect(one.view.bounds.at(-1)).toEqual(filled);
    one.contents.emit("leave-html-full-screen");
    expect(one.view.bounds.at(-1)).toEqual({
      ...BOX,
      width: 700,
    });
  });

  it("follows the window it is in and no other: moved to a window of its own, the first is let go", () => {
    const one = page();
    const own = fakeWindow(900, 700);
    one.made.placeIn(one.window, BOX);
    one.made.placeIn(own, BOX);
    expect(one.window.resizeHandlers).toEqual([]);
    expect(own.resizeHandlers).toHaveLength(1);
    one.contents.emit("enter-html-full-screen");
    expect(one.view.bounds.at(-1)).toEqual({
      x: 0,
      y: 0,
      width: 900,
      height: 700,
    });
  });

  it("is given by default to a request for fullscreen or pointer lock, from the Fullscreen row", async () => {
    const one = page();
    const decider = deciderOf(one.contents as never)!;
    expect(await decider.request("fullscreen", "https://login.example.com/", {})).toBe(true);
    expect(await decider.request("pointerLock", "https://login.example.com/", {})).toBe(true);
    expect(decider.check("fullscreen", "https://login.example.com", {})).toBe(true);
    const blocked = page({
      saved: [
        [
          "login.example.com",
          "fullscreen",
          "block",
        ],
      ],
    });
    const refusing = deciderOf(blocked.contents as never)!;
    expect(await refusing.request("fullscreen", "https://login.example.com/", {})).toBe(false);
    expect(await refusing.request("pointerLock", "https://login.example.com/", {})).toBe(false);
  });
});

describe("local network access", () => {
  const LOCAL = "local-network-access";
  const PAGE = "https://app.example.com/";

  it("is given where the choice is Allow, refused where it is Block, and asked where it is Ask", async () => {
    const allowed = page({
      saved: [
        [
          "app.example.com",
          "local_network",
          "allow",
        ],
      ],
    });
    const yes = deciderOf(allowed.contents as never)!;
    expect(await yes.request(LOCAL, PAGE, {})).toBe(true);
    expect(yes.check(LOCAL, "https://app.example.com", {})).toBe(true);
    const blocked = page({
      saved: [
        [
          "app.example.com",
          "local_network",
          "block",
        ],
      ],
    });
    const no = deciderOf(blocked.contents as never)!;
    expect(await no.request(LOCAL, PAGE, {})).toBe(false);
    expect(no.check(LOCAL, "https://app.example.com", {})).toBe(false);
    const asks = page({
      answer: "allow",
    });
    const ask = deciderOf(asks.contents as never)!;
    expect(await ask.request(LOCAL, PAGE, {})).toBe(true);
    expect(asks.asked).toEqual([
      [
        "local_network",
        "app.example.com",
      ],
    ]);
    // Asked about once: the answer is the site's from now on.
    expect(ask.check(LOCAL, "https://app.example.com", {})).toBe(true);
  });
});
