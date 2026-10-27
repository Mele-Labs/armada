// The popup a Session's page may open — `docs/practices/capture-window.md`, *A popup a Session's page opened*.
// Electron is faked: what is asserted is who may open one, what holds it, and that it goes with the walk.

import { beforeEach, describe, expect, it, vi } from "vitest";

type Handler = (...args: never[]) => unknown;

const world = vi.hoisted(() => ({ windows: [] as unknown[], throwOn: null as string | null }));

vi.mock("electron", () => {
  class Emitter {
    handlers = new Map<string, Handler[]>();
    on(name: string, handler: Handler): this {
      this.handlers.set(name, [...(this.handlers.get(name) ?? []), handler]);
      return this;
    }
    emit(name: string, ...args: unknown[]): void {
      for (const handler of this.handlers.get(name) ?? []) (handler as (...rest: unknown[]) => unknown)(...args);
    }
  }
  class BaseWindow extends Emitter {
    destroyed = false;
    shown = false;
    title: string | undefined;
    children: unknown[] = [];
    options: unknown;
    contentView = { addChildView: (view: unknown) => void this.children.push(view) };
    constructor(options: unknown) {
      super();
      if (world.throwOn === "window" && (options as { show?: boolean }).show === false) throw new Error("no window");
      this.options = options;
      world.windows.push(this);
    }
    isDestroyed = () => this.destroyed;
    getContentBounds = () => ({ x: 0, y: 0, width: 520, height: 720 });
    show = () => void (this.shown = true);
    setTitle = (title: string) => void (this.title = title);
    close = () => {
      if (this.destroyed) return;
      this.destroyed = true;
      this.emit("closed");
    };
  }
  class WebContentsView {
    webContents: unknown;
    bounds: unknown;
    constructor(options: { webContents: unknown }) {
      if (world.throwOn === "view") throw new Error("no view");
      this.webContents = options.webContents;
    }
    setBounds = (bounds: unknown) => void (this.bounds = bounds);
  }
  return {
    app: {
      userAgentFallback:
        "Mozilla/5.0 (Macintosh) AppleWebKit/537.36 (KHTML, like Gecko) Armada/0.1.0 Chrome/144.0.0.0 Electron/40.10.6 Safari/537.36",
    },
    BaseWindow,
    WebContentsView,
  };
});

import { BaseWindow } from "electron";
import { may } from "./permissions";
import { mayOpenPopup, openPopup, type PopupHold } from "./popup";

let next = 0;

type Fake = {
  destroyed: boolean;
  shown: boolean;
  title: string | undefined;
  children: unknown[];
  options: unknown;
};

/** The contents Electron hands `createWindow`: a record of what was set on it and what it was told. */
function made() {
  const handlers = new Map<string, Handler[]>();
  const state = { destroyed: false, closed: 0, agent: "", url: "", opened: null as Handler | null };
  const contents = {
    id: ++next,
    on: (name: string, handler: Handler) => void handlers.set(name, [...(handlers.get(name) ?? []), handler]),
    emit: (name: string, ...args: unknown[]) => (handlers.get(name) ?? []).forEach((handler) => (handler as (...rest: unknown[]) => unknown)(...args)),
    setUserAgent: (agent: string) => void (state.agent = agent),
    setWindowOpenHandler: (handler: Handler) => void (state.opened = handler),
    getURL: () => state.url,
    isDestroyed: () => state.destroyed,
    close: () => {
      state.closed += 1;
      state.destroyed = true;
    },
  };
  return { contents, state };
}

const SHOWN = "https://app.example.com";
const NAMED = "https://login.example.org";

beforeEach(() => {
  world.windows = [];
  world.throwOn = null;
});

describe("which window.open becomes the popup", () => {
  const ask = { session: true, standing: false, allowed: true };

  it("a Session's window, none open, onto an address it may reach", () => {
    expect(mayOpenPopup(ask)).toBe(true);
  });

  it("not onto an address the window may not reach", () => {
    expect(mayOpenPopup({ ...ask, allowed: false })).toBe(false);
  });

  it("one at a time: a second is refused while one stands", () => {
    expect(mayOpenPopup({ ...ask, standing: true })).toBe(false);
  });

  it("never for a Run's or a Job's window", () => {
    expect(mayOpenPopup({ ...ask, session: false })).toBe(false);
  });
});

describe("the popup", () => {
  function opened() {
    const calls = { closed: 0 };
    const refused: [string, string][] = [];
    const parent = new BaseWindow({});
    const guard: PopupHold = {
      parent,
      allowed: (address) => address.startsWith(SHOWN) || address.startsWith(NAMED),
      refuse: (address, what) => void refused.push([address, what]),
      closed: () => void (calls.closed += 1),
    };
    const fake = made();
    const popup = openPopup(guard, { webContents: fake.contents as never });
    return { popup, fake, refused, calls, window: world.windows[1] as Fake };
  }

  it("is a shown child window around the contents Electron made, with the partition's Chrome agent", () => {
    const { popup, fake, window } = opened();
    expect(popup?.contents).toBe(fake.contents);
    expect(window.shown).toBe(true);
    expect(window.children).toHaveLength(1);
    expect((window.options as { parent: unknown }).parent).toBe(world.windows[0]);
    expect(fake.state.agent).toBe("Mozilla/5.0 (Macintosh) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36");
  });

  it("is given local network access on the origins the window may reach, and no longer once it is destroyed", () => {
    const { fake } = opened();
    expect(may(fake.contents as never)(`${NAMED}/authorize`)).toBe(true);
    expect(may(fake.contents as never)("https://elsewhere.example.net/x")).toBe(false);
    fake.contents.emit("destroyed");
    expect(may(fake.contents as never)(`${NAMED}/authorize`)).toBe(false);
  });

  it("follows a navigation onto the shown origin or a named one, and refuses every other", () => {
    const { fake, refused } = opened();
    const event = () => ({ preventDefault: vi.fn() });
    const kept = event();
    fake.contents.emit("will-navigate", kept, `${NAMED}/authorize`);
    fake.contents.emit("will-redirect", kept, `${SHOWN}/callback`);
    expect(kept.preventDefault).not.toHaveBeenCalled();

    const navigation = event();
    fake.contents.emit("will-navigate", navigation, "https://elsewhere.example.net/x");
    const redirect = event();
    fake.contents.emit("will-redirect", redirect, "https://elsewhere.example.net/y");
    expect(navigation.preventDefault).toHaveBeenCalled();
    expect(redirect.preventDefault).toHaveBeenCalled();
    expect(refused).toEqual([
      ["https://elsewhere.example.net/x", "navigation"],
      ["https://elsewhere.example.net/y", "redirect"],
    ]);
  });

  it("holds a subframe to the same origins", () => {
    const { fake } = opened();
    const frame = { isMainFrame: false, url: "https://ads.example.net/", preventDefault: vi.fn() };
    fake.contents.emit("will-frame-navigate", frame);
    expect(frame.preventDefault).toHaveBeenCalled();
    const allowed = { isMainFrame: false, url: `${NAMED}/frame`, preventDefault: vi.fn() };
    fake.contents.emit("will-frame-navigate", allowed);
    expect(allowed.preventDefault).not.toHaveBeenCalled();
  });

  it("refuses a popup's own popup, even onto an origin it may reach", () => {
    const { fake, refused } = opened();
    expect(fake.state.opened?.({ url: `${NAMED}/more` } as never)).toEqual({ action: "deny" });
    expect(refused).toEqual([[`${NAMED}/more`, "window"]]);
  });

  it("is titled by the origin it is on", () => {
    const { fake, window } = opened();
    fake.state.url = `${NAMED}/authorize?x=1`;
    fake.contents.emit("did-navigate");
    expect(window.title).toBe(NAMED);
  });

  it("closes when the page calls window.close(), which destroys its contents", () => {
    const { fake, window, calls } = opened();
    fake.state.destroyed = true;
    fake.contents.emit("destroyed");
    expect(window.destroyed).toBe(true);
    expect(calls.closed).toBe(1);
  });

  it("closes with the walk window, and its contents with it", () => {
    const { popup, fake, window, calls } = opened();
    popup?.close();
    expect(window.destroyed).toBe(true);
    expect(fake.state.closed).toBe(1);
    expect(calls.closed).toBe(1);
    popup?.close(); // already gone: nothing is closed twice
    expect(fake.state.closed).toBe(1);
  });

  it("builds nothing without the contents Electron made", () => {
    const guard = { parent: new BaseWindow({}), allowed: () => true, refuse: () => {}, closed: () => {} } as unknown as PopupHold;
    expect(openPopup(guard, {})).toBeNull();
  });

  it("closes quietly and answers nothing when building throws", () => {
    for (const where of ["window", "view"]) {
      world.windows = [];
      world.throwOn = null;
      const guard = { parent: new BaseWindow({}), allowed: () => true, refuse: () => {}, closed: () => {} } as unknown as PopupHold;
      world.throwOn = where;
      const fake = made();
      expect(openPopup(guard, { webContents: fake.contents as never })).toBeNull();
      expect(fake.state.closed).toBe(1);
      // A window made before the throw does not stay on screen.
      for (const window of world.windows.slice(1) as Fake[]) expect(window.destroyed).toBe(true);
    }
  });
});
