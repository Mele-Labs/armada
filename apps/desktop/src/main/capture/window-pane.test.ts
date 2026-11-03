// The real `CaptureWindow` in a pane's seat: a Job's walk drawn as a tab, its page and its bar two views laid inside the frame the pane reports in
// Bridge's own window. Over stand-in views, this holds where they are laid, when they are parked, and which keys go back to Bridge. What the
// real compositor does with the views, and whether the keys arrive, is not proved here. `docs/practices/capture-window.md`.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  BaseWindow: class {},
  screen: {
    getPrimaryDisplay: () => ({
      workAreaSize: {
        width: 1920,
      },
    }),
  },
  shell: {},
}));

type Handler = (
  event: {
    preventDefault: () => void;
  },
  input: unknown,
) => void;
type Stub = ReturnType<typeof view>;

const views = vi.hoisted(() => ({
  page: null as unknown,
}));
vi.mock("./views", () => ({
  barView: () => view(),
  pageView: () => views.page,
  panelView: () => view(),
}));

import type { PaneFrame, WalkSeat } from "./window";
import { CaptureWindow } from "./window";

const ADDRESS = "https://app.example.com/page";

/** A view that keeps the bounds it was given, whether it is visible, and the press handlers it was told to hear. */
function view() {
  const handlers: Handler[] = [];
  const stub = {
    bounds: null as unknown,
    visible: true,
    /** What the page answers to whether a field has the cursor. */
    field: false,
    /** Whether the view has the keyboard. */
    focus: false,
    handlers,
    closed: false,
    setBounds(bounds: unknown) {
      stub.bounds = bounds;
    },
    setVisible(visible: boolean) {
      stub.visible = visible;
    },
    webContents: {
      id: 1,
      on: (event: string, handler: Handler) => {
        if (event === "before-input-event") handlers.push(handler);
      },
      send: () => {},
      isDestroyed: () => stub.closed,
      isFocused: () => stub.focus,
      close: () => void (stub.closed = true),
      setWindowOpenHandler: () => {},
      getURL: () => ADDRESS,
      loadURL: () => Promise.resolve(),
      executeJavaScript: () => Promise.resolve(false),
      executeJavaScriptInIsolatedWorld: () => Promise.resolve(stub.field),
      capturePage: () =>
        Promise.resolve({
          toPNG: () =>
            Buffer.from([
              1,
              2,
              3,
            ]),
        }),
    },
  };
  return stub;
}

/** Bridge's window: the views added to it, in order, and how many times its own page was given the keyboard. */
function host() {
  const children: unknown[] = [];
  const focused = vi.fn();
  return {
    children,
    focused,
    webContents: {
      focus: focused,
    },
    isDestroyed: () => false,
    contentView: {
      addChildView: (child: unknown) => void children.push(child),
      removeChildView: (child: unknown) => void children.splice(children.indexOf(child), 1),
    },
  };
}

const FRAME = {
  x: 300,
  y: 120,
  width: 900,
  height: 600,
};
const BAR = 40;

let bridge: ReturnType<typeof host> | null;
let frame: typeof FRAME | null;
let forwarded: unknown[];
let keyed: string[];
let page: Stub;
let bar: Stub;

function walk(): CaptureWindow {
  const pane: PaneFrame = {
    host: () => bridge as never,
    frame: () => frame,
    forward: (input) => {
      forwarded.push(input);
      return true;
    },
  };
  const seat = {
    window: null,
    bar,
    panel: null,
    onScreen: () => true,
    queue: () => ({
      entries: [],
      at: 0,
    }),
    key: (key: string) => void keyed.push(key),
    settled: () => {},
    pane,
  } as unknown as WalkSeat;
  return new CaptureWindow(
    {
      run: "run",
      name: "app",
      origin: "https://app.example.com",
      url: ADDRESS,
      manifestId: null,
      through: [],
    },
    {
      session: {
        id: "job",
      },
    } as never,
    {
      focused: () => {},
      heldOn: () => 0,
    } as never,
    {
      bar: BAR,
      panel: 300,
      floor: 100,
    },
    null,
    [],
    [],
    [],
    seat,
  );
}

const press = (code: string, key: string, held: Record<string, boolean> = {}) => ({
  type: "keyDown",
  code,
  key,
  meta: false,
  alt: false,
  control: false,
  shift: false,
  ...held,
});

/** What a view does with a press: whether it let it through to its page, `true`, or took it. */
function pressed(view: Stub, input: unknown): boolean {
  let prevented = false;
  for (const handler of view.handlers)
    handler(
      {
        preventDefault: () => void (prevented = true),
      },
      input,
    );
  return !prevented;
}

beforeEach(() => {
  bridge = host();
  frame = FRAME;
  forwarded = [];
  keyed = [];
  page = view();
  views.page = page;
  bar = view();
});

it("lays the bar over the top two rows of the frame and the page under it, both offset into Bridge's window", () => {
  const shown = walk();
  shown.show();
  expect(bridge!.children).toEqual([
    page,
    bar,
  ]);
  expect(bar.bounds).toEqual({
    x: 300,
    y: 120,
    width: 900,
    height: BAR * 2,
  });
  expect(page.bounds).toEqual({
    x: 300,
    y: 120 + BAR * 2,
    width: 900,
    height: 600 - BAR * 2,
  });
  expect(page.visible && bar.visible).toBe(true);
});

it("moves with a frame that moves, and is added to Bridge's window once", () => {
  const shown = walk();
  shown.show();
  frame = {
    ...FRAME,
    x: 10,
    width: 500,
  };
  shown.resized();
  expect(bridge!.children).toHaveLength(2);
  expect(page.bounds).toMatchObject({
    x: 10,
    width: 500,
  });
  expect(bar.bounds).toMatchObject({
    x: 10,
    width: 500,
  });
});

it("parks both views, keeping the page, where the pane has no frame or Bridge has no window", () => {
  const shown = walk();
  shown.show();
  frame = null;
  shown.resized();
  expect(page.visible || bar.visible).toBe(false);
  expect(page.webContents.isDestroyed()).toBe(false);
  frame = FRAME;
  shown.resized();
  expect(page.visible && bar.visible).toBe(true);
  bridge = null;
  shown.resized();
  expect(page.visible || bar.visible).toBe(false);
});

it("is moved to a new window of Bridge's when the first one is gone, and leaves the old one", () => {
  const shown = walk();
  shown.show();
  const first = bridge!;
  bridge = host();
  shown.resized();
  expect(first.children).toEqual([]);
  expect(bridge.children).toEqual([
    page,
    bar,
  ]);
});

it("hides both views on hide, and shows them again with show", () => {
  const shown = walk();
  shown.show();
  shown.hide();
  expect(page.visible || bar.visible).toBe(false);
  shown.show();
  expect(page.visible && bar.visible).toBe(true);
});

it("brings the bar down over the page as far as a menu hangs, and takes it back", () => {
  const shown = walk();
  shown.show();
  shown.overhang(300);
  expect(bar.bounds).toMatchObject({
    height: 300,
  });
  shown.overhang(null);
  expect(bar.bounds).toMatchObject({
    height: BAR * 2,
  });
});

it("holds both views as pictures and hides them under a layer of Bridge's own, and lets them back on thaw", async () => {
  const shown = walk();
  shown.show();
  const frozen = await shown.freeze();
  expect(frozen).toEqual({
    page: new Uint8Array([
      1,
      2,
      3,
    ]),
    bar: new Uint8Array([
      1,
      2,
      3,
    ]),
  });
  expect(page.visible || bar.visible).toBe(false);
  shown.thaw();
  expect(page.visible && bar.visible).toBe(true);
});

it.each([
  [
    "page",
    () => page,
  ],
  [
    "bar",
    () => bar,
  ],
])(
  "gives the keyboard back to Bridge's window when a layer of its own holds the %s that has it",
  async (_name, which) => {
    const shown = walk();
    shown.show();
    which().focus = true;
    await shown.freeze();
    expect(bridge!.focused).toHaveBeenCalledTimes(1);
  },
);

it("leaves the keyboard where it is when a layer holds views that do not have it", async () => {
  const shown = walk();
  shown.show();
  await shown.freeze();
  expect(bridge!.focused).not.toHaveBeenCalled();
});

it.each([
  [
    "page",
    () => page,
  ],
  [
    "bar",
    () => bar,
  ],
])(
  "gives the keyboard back to Bridge's window when hide puts away the %s that has it",
  (_name, which) => {
    const shown = walk();
    shown.show();
    which().focus = true;
    shown.hide();
    expect(bridge!.focused).toHaveBeenCalledTimes(1);
  },
);

it("takes both views out of Bridge's window and closes their pages when the walk goes", () => {
  const shown = walk();
  shown.show();
  shown.dispose();
  expect(bridge!.children).toEqual([]);
  expect(page.webContents.isDestroyed()).toBe(true);
  expect(bar.webContents.isDestroyed()).toBe(true);
});

it("hands Bridge's keys to Bridge's window from the page and from the bar, and keeps them from the view", () => {
  walk();
  for (const view of [
    page,
    bar,
  ]) {
    forwarded = [];
    // ⌘K and Esc are asked of the seat's forwarder, which takes them, so the view does not hear them. What it refuses is `pane-keys.test.ts`'s.
    expect(
      pressed(
        view,
        press("KeyK", "k", {
          meta: true,
        }),
      ),
    ).toBe(false);
    expect(pressed(view, press("Escape", "Escape"))).toBe(false);
    expect(forwarded).toHaveLength(2);
  }
});

it("hands nothing back that the walk reads itself: Approve and Reload, from the page to its bar, and the capture chord", async () => {
  walk();
  expect(
    pressed(
      page,
      press("KeyR", "r", {
        meta: true,
      }),
    ),
  ).toBe(false);
  expect(keyed).toEqual([
    "reload",
  ]);
  pressed(
    page,
    press("Enter", "Enter", {
      meta: true,
    }),
  );
  await Promise.resolve();
  await Promise.resolve();
  expect(keyed).toEqual([
    "reload",
    "approve",
  ]);
  // The bar acts on its own ⌘↵ and ⌘R.
  pressed(
    bar,
    press("Enter", "Enter", {
      meta: true,
    }),
  );
  pressed(
    bar,
    press("KeyR", "r", {
      meta: true,
    }),
  );
  await Promise.resolve();
  expect(keyed).toHaveLength(2);
  // ⌥⌘C is the capture binding, the walk's own.
  pressed(
    page,
    press("KeyC", "c", {
      meta: true,
      alt: true,
    }),
  );
  expect(forwarded).toEqual([]);
});

it("leaves Escape to the bar while a menu hangs over the page", () => {
  const shown = walk();
  shown.show();
  shown.overhang(200);
  pressed(bar, press("Escape", "Escape"));
  pressed(page, press("Escape", "Escape"));
  expect(forwarded).toEqual([]);
  shown.overhang(null);
  pressed(bar, press("Escape", "Escape"));
  expect(forwarded).toHaveLength(1);
});

it("leaves the page's own ⌘↵ to a field that has the cursor, and approves from it otherwise", async () => {
  walk();
  page.field = true;
  pressed(
    page,
    press("Enter", "Enter", {
      meta: true,
    }),
  );
  await Promise.resolve();
  await Promise.resolve();
  expect(keyed).toEqual([]);
});
