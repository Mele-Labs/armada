// A Job's walk as `CaptureWindows` drives it: **a tab of the Job in Bridge's own window**, with no window of its own and nothing raised or
// focused. Its page and bar are made when its pane first reports a frame and parked when the frame goes, and it follows its Job's waiting
// item. A pull request's walk beside it still opens the one walk window. Each walk's page is a stand-in, which keeps the calls made on it.
// `docs/practices/capture-window.md`.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

const electron = vi.hoisted(() => ({
  windows: [] as unknown[],
}));
/** The contents the bar of a walk is told apart by. */
const BAR_OF = vi.hoisted(() => ({
  id: 99,
}));
vi.mock("electron", async () => {
  const { EventEmitter } = await import("node:events");
  class BaseWindow extends EventEmitter {
    calls: string[] = [];
    destroyed = false;
    contentView = {
      addChildView: () => {},
    };
    constructor() {
      super();
      electron.windows.push(this);
    }
    showInactive() {
      this.calls.push("showInactive");
    }
    show() {
      this.calls.push("show");
    }
    focus() {
      this.calls.push("focus");
    }
    restore() {
      this.calls.push("restore");
    }
    isMinimized() {
      return false;
    }
    isDestroyed() {
      return this.destroyed;
    }
    close() {
      if (this.destroyed) return;
      this.destroyed = true;
      this.emit("closed");
    }
  }
  return {
    BaseWindow,
    screen: {
      getPrimaryDisplay: () => ({
        workAreaSize: {
          width: 1920,
        },
      }),
    },
  };
});

// The bar is contents 1 and the panel 2, so a call can be told to come from the bar.
vi.mock("./views", () => {
  const view = (id: number) => () => ({
    webContents: {
      id,
      on: () => {},
      send: () => {},
      isDestroyed: () => false,
      close: () => {},
      focus: () => {},
    },
    setVisible: () => {},
  });
  return {
    barView: view(1),
    panelView: view(2),
  };
});

const made = vi.hoisted(() => ({
  pages: [] as unknown[],
}));
vi.mock("./window", () => ({
  isCaptureBinding: () => false,
  CaptureWindow: class {
    open = true;
    origins: string[];
    seat: unknown;
    args: unknown[];
    events: string[] = [];
    questions: unknown[] = [];
    held: string[] = [];
    frozen = {
      page: "page.png",
      bar: "bar.png",
    };
    constructor(...args: unknown[]) {
      this.args = args;
      this.seat = args[8];
      this.origins = [
        (
          args[0] as {
            origin: string;
          }
        ).origin,
      ];
      made.pages.push(this);
    }
    show() {
      this.events.push("show");
    }
    hide() {
      this.events.push("hide");
    }
    dispose() {
      this.events.push("dispose");
    }
    redraw() {
      this.events.push("redraw");
    }
    resized() {
      this.events.push("resized");
    }
    heldChanged(sessionId: string) {
      this.held.push(sessionId);
    }
    owns(contents: unknown) {
      return contents === BAR_OF;
    }
    freeze() {
      this.events.push("freeze");
      return Promise.resolve(this.frozen);
    }
    thaw() {
      this.events.push("thaw");
    }
    toggleCapture() {}
    readiness: unknown[] = [];
    setReadiness(readiness: unknown) {
      this.readiness.push(readiness);
    }
    question(asked: unknown) {
      this.questions.push(asked);
    }
    // What the bar's Approve ends in once the owner's Job has been told: the walk leaves the window.
    approve() {
      (this.seat as Seat).settled(this);
    }
  },
}));

import type { BrowserWindow } from "electron";

import type { SessionRecord, WaitingItem, WalkStep } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { FRAME, HOST, placeAt } from "./pane.fake";
import { CaptureWindows } from "./windows";

type Pane = {
  host: () => unknown;
  frame: () => {
    x: number;
    y: number;
    width: number;
    height: number;
  } | null;
};
type Seat = {
  window: unknown;
  panel: unknown;
  queue: () => {
    entries: unknown[];
    at: number;
  };
  onScreen: () => boolean;
  settled: (page: unknown) => void;
  pane: Pane;
};
type Page = {
  seat: Seat;
  args: unknown[];
  events: string[];
  questions: unknown[];
  readiness: unknown[];
  held: string[];
  frozen: unknown;
  approve: () => void;
};

const pages = () => made.pages as Page[];
const windows_ = () =>
  electron.windows as {
    calls: string[];
  }[];

const at = (time: string) => `2026-10-19T${time}:00Z`;
const url = (name: string) => `https://${name}.example.com/app`;

const walk = (name: string, since: string): WaitingItem => ({
  id: `walk:${name}`,
  text: `Walk ${name}`,
  since,
  source: "walk",
  act: {
    kind: "walk",
    target: url(name),
  },
});

const record = (id: string, ...waiting: WaitingItem[]): SessionRecord => ({
  id,
  harness: "h",
  origin: "bridge",
  cwd: "/repo",
  state: "live",
  started_at: at("08:00"),
  last_seen_at: at("08:00"),
  usage: {},
  attachments: [],
  waiting_for: waiting,
});

/** The Job's record with the window it showed at `name`'s address, which Fleet marks working once notes were sent on it. */
const showing = (
  job: SessionRecord,
  name: string,
  working: boolean,
  title = `Walk ${name}`,
): SessionRecord => ({
  ...job,
  attachments: [
    {
      kind: "artifact",
      target: url(name),
      state: "standing",
      detail: {
        form: "window",
        title,
        ...(working
          ? {
              working: "true",
              changed: "true",
            }
          : {}),
      },
      since: at("09:00"),
      changed_at: at("09:00"),
    },
  ],
});

const state = (...sessions: SessionRecord[]) => ({
  ...NOTHING_YET,
  sessions: {
    state: "read" as const,
    sessions,
  },
});

const board = {
  sessionClosed: vi.fn(),
  sessionApproved: vi.fn(),
  answerAsk: vi.fn(),
  reviewClosed: vi.fn(),
  focused: vi.fn(),
};

let host: unknown = HOST;
/** Bridge, with its own window up, holding these Jobs. */
function holding(...sessions: SessionRecord[]): CaptureWindows {
  const shown = new CaptureWindows(
    board as unknown as ConstructorParameters<typeof CaptureWindows>[0],
    () => host as BrowserWindow | null,
  );
  shown.changed(state(...sessions));
  return shown;
}

const STEPS: WalkStep[] = [
  {
    say: "Open the list",
    press: {
      role: "button",
      name: "Drafts",
    },
  },
];

beforeEach(() => {
  made.pages = [];
  electron.windows = [];
  host = HOST;
  for (const one of Object.values(board)) one.mockClear();
});
afterEach(() => vi.useRealTimers());

it("makes no window for a Job's walk, loads nothing until its pane has a frame, and takes no focus", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  expect(windows_()).toHaveLength(0);
  expect(pages()).toHaveLength(0);
  expect(board.focused).not.toHaveBeenCalled();
  placeAt(shown, "a", url("a"));
  expect(pages()).toHaveLength(1);
  expect(windows_()).toHaveLength(0);
  expect(board.focused).not.toHaveBeenCalled();
});

it("still opens the one walk window, behind, for a pull request's walk", async () => {
  const shown = holding();
  await shown.openForReview({
    walkId: "w1",
    manifestId: "armada",
    number: 12,
    title: "#12 Take a draft out",
    address: "https://pr-12.preview.invalid/app",
    steps: STEPS,
  });
  expect(windows_()).toHaveLength(1);
  expect(windows_()[0]!.calls).toEqual([
    "showInactive",
  ]);
});

it("makes a walk's page in a pane's seat: no window, no panel, no queue, the frame its pane reported", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"), [], [], [], STEPS);
  placeAt(shown, "a", url("a"), {
    x: 40.4,
    y: 60,
    width: 800,
    height: 500,
  });
  const seat = pages()[0]!.seat;
  expect(seat.window).toBeNull();
  expect(seat.panel).toBeNull();
  expect(seat.queue().entries).toEqual([]);
  expect(seat.pane.host()).toBe(HOST);
  expect(seat.pane.frame()).toEqual(FRAME);
  expect(seat.onScreen()).toBe(true);
  expect(pages()[0]!.args[7]).toEqual(STEPS);
  expect(pages()[0]!.events).toEqual([
    "show",
  ]);
});

it("holds the frame inside Bridge's window, whatever the renderer reported", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"), {
    x: -50,
    y: 100,
    width: 5000,
    height: Number.NaN,
  });
  expect(pages()[0]!.seat.pane.frame()).toEqual({
    x: 0,
    y: 100,
    width: 1200,
    height: 0,
  });
});

it("places nothing in a window that is gone", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.place(
    {
      isDestroyed: () => true,
      getContentBounds: () => ({
        width: 1,
        height: 1,
      }),
    } as never,
    "a",
    url("a"),
    FRAME,
  );
  expect(pages()).toHaveLength(0);
});

it("parks a walk's views when its pane leaves, keeps its page, and shows it again where its pane returns", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  shown.unplace("a", url("a"));
  expect(pages()[0]!.events).toEqual([
    "show",
    "hide",
  ]);
  expect(pages()[0]!.seat.pane.frame()).toBeNull();
  expect(pages()[0]!.seat.onScreen()).toBe(false);
  placeAt(shown, "a", url("a"));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).toEqual([
    "show",
    "hide",
    "show",
  ]);
  expect(pages()[0]!.events).not.toContain("dispose");
});

it("follows a pane that moves with the same page, and parks nothing it never placed", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.unplace("a", url("a"));
  expect(pages()).toHaveLength(0);
  placeAt(shown, "a", url("a"));
  placeAt(shown, "a", url("a"), {
    ...FRAME,
    width: 600,
  });
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).toEqual([
    "show",
    "resized",
  ]);
  expect(pages()[0]!.seat.pane.frame()?.width).toBe(600);
});

it("settles a walk on Approve: its page goes, Fleet is told its window closed, and the publish before Fleet settles does not hold it again", () => {
  const items = () => [
    record("a", walk("a", at("09:01"))),
    record("b", walk("b", at("09:02"))),
  ];
  const shown = holding(...items());
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  pages()[0]!.approve();
  expect(pages()[0]!.events).toContain("dispose");
  expect(board.sessionClosed).toHaveBeenCalledTimes(1);
  expect(board.sessionClosed).toHaveBeenCalledWith("a", url("a"));
  shown.changed(state(...items()));
  placeAt(shown, "a", url("a"));
  expect(pages()).toHaveLength(1);
  // Fleet settles the item; a later walk of the same page, newly asked for, is held once more.
  shown.changed(state(record("a"), record("b", walk("b", at("09:02")))));
  shown.changed(state(record("a", walk("a", at("09:20"))), record("b", walk("b", at("09:02")))));
  placeAt(shown, "a", url("a"));
  expect(pages()).toHaveLength(2);
});

it("keeps a walk, reading working, while its item is gone and its Job works on the notes sent, and lets it go once it reads ready", () => {
  const shown = holding(
    showing(record("a", walk("a", at("09:01"))), "a", false),
    record("b", walk("b", at("09:02"))),
  );
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  shown.changed(state(showing(record("a"), "a", true), record("b", walk("b", at("09:02")))));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).not.toContain("dispose");
  expect(pages()[0]!.readiness.at(-1)).toBe("working");
  expect(board.sessionClosed).not.toHaveBeenCalled();
  shown.changed(state(showing(record("a"), "a", false), record("b", walk("b", at("09:02")))));
  expect(pages()[0]!.events).toContain("dispose");
  expect(board.sessionClosed).toHaveBeenCalledWith("a", url("a"));
});

it("makes a kept walk's page afresh in its place, in the frame it had, reading ready, when the Job shows it again", () => {
  const shown = holding(showing(record("a", walk("a", at("09:01"))), "a", false));
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  shown.changed(state(showing(record("a"), "a", true)));
  shown.changed(state(showing(record("a", walk("a", at("09:10"))), "a", false, "Walk a, again")));
  expect(pages()[0]!.events).toContain("dispose");
  expect(pages()).toHaveLength(2);
  expect(pages()[1]!.readiness.at(-1)).toBe("ready");
  expect(pages()[1]!.seat.pane.frame()).toEqual(FRAME);
  // Shown again, it is no longer kept: its item going while ready lets it go.
  shown.changed(state(showing(record("a"), "a", false)));
  expect(pages()[1]!.events).toContain("dispose");
});

it("makes a walk shown again with a sign-in afresh in its place, in the frame it had", () => {
  const link = "https://a.example.com/dev/login";
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  shown.openForSession(
    "a",
    "Walk a",
    url("a"),
    [],
    [],
    [
      link,
    ],
  );
  expect(pages()).toHaveLength(2);
  expect(pages()[0]!.events).toContain("dispose");
  expect(pages()[1]!.args[6]).toEqual([
    link,
  ]);
  expect(pages()[1]!.seat.pane.frame()).toEqual(FRAME);
});

it("holds a sign-in until the walk's page is first made, and hands it to that page alone", () => {
  const link = "https://a.example.com/dev/login";
  const shown = holding(record("a", walk("a", at("09:01"))));
  shown.openForSession(
    "a",
    "Walk a",
    url("a"),
    [],
    [],
    [
      link,
    ],
  );
  expect(pages()).toHaveLength(0);
  placeAt(shown, "a", url("a"));
  expect(pages()[0]!.args[6]).toEqual([
    link,
  ]);
  expect(JSON.stringify(pages()[0]!.args[0])).not.toContain("dev/login");
});

it("shows a question on its walk's bar wherever the walk's tab is, before and after its page is made", () => {
  const asked: WaitingItem = {
    id: "window_ask:q1",
    text: "Is this the right heading?",
    since: at("09:05"),
    source: "window_ask",
    options: [],
    place: {
      url: url("b"),
      selector: "main > h1",
    },
  };
  const shown = holding(
    record("a", walk("a", at("09:01"))),
    record("b", walk("b", at("09:02")), asked),
  );
  shown.openForSession("a", "Walk a", url("a"));
  placeAt(shown, "a", url("a"));
  expect(shown.askedFor("b", "Walk b", url("b"), "main > h1", "window_ask:q1")).toEqual({
    ok: true,
  });
  // The walk of the Job asked about is held, with no page and no window; the walk on a pane is not asked.
  expect(pages()).toHaveLength(1);
  expect(windows_()).toHaveLength(0);
  expect(pages()[0]!.questions.at(-1)).toBeNull();
  placeAt(shown, "b", url("b"));
  expect(pages()[1]!.questions.at(-1)).toMatchObject({
    id: "window_ask:q1",
    question: "Is this the right heading?",
  });
});

it("hands a question to the walk the Job asked in, of two on one origin, wherever their tabs are", () => {
  const other = "https://a.example.com/other";
  const asked: WaitingItem = {
    id: "window_ask:q2",
    text: "Which heading?",
    since: at("09:05"),
    source: "window_ask",
    options: [],
    place: {
      url: url("a"),
      selector: "h1",
    },
  };
  const shown = holding(record("a", asked));
  shown.openForSession("a", "First", url("a"));
  shown.openForSession("a", "Other", other);
  placeAt(shown, "a", url("a"));
  placeAt(shown, "a", other);
  // Bridge has not heard where it was asked: each walk on its origin shows it.
  expect(pages()[0]!.questions.at(-1)).toMatchObject({
    id: "window_ask:q2",
  });
  expect(pages()[1]!.questions.at(-1)).toMatchObject({
    id: "window_ask:q2",
  });
  shown.askedFor("a", "Other", other, "h1", "window_ask:q2");
  expect(pages()[0]!.questions.at(-1)).toBeNull();
  expect(pages()[1]!.questions.at(-1)).toMatchObject({
    id: "window_ask:q2",
  });
  expect(windows_()).toHaveLength(0);
});

it("holds a walk that stands in Fleet's record with no tab, as after a restart, and makes its page when a tab places it", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  expect(pages()).toHaveLength(0);
  placeAt(shown, "a", url("a"));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.seat.pane.frame()).toEqual(FRAME);
  expect(windows_()).toHaveLength(0);
  // A walk Fleet does not hold, placed, makes nothing.
  placeAt(shown, "z", url("z"));
  expect(pages()).toHaveLength(1);
});

it("tells the Job's held notes to each walk made, and to none that was never placed", () => {
  const shown = holding(record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02"))));
  placeAt(shown, "a", url("a"));
  shown.heldChanged("a");
  expect(pages()[0]!.held).toEqual([
    "a",
  ]);
  placeAt(shown, "b", url("b"));
  shown.heldChanged("b");
  expect(pages().map((one) => one.held)).toEqual([
    [
      "a",
      "b",
    ],
    [
      "b",
    ],
  ]);
});

it("names the walk whose bar a call came from, and no other", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  placeAt(shown, "a", url("a"));
  expect(shown.from(BAR_OF as unknown as Electron.WebContents)).toBe(pages()[0]);
  expect(shown.from({} as Electron.WebContents)).toBeNull();
});

it("holds a walk's views as pictures over a DOM layer, and lets them back", async () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  expect(await shown.freeze("a", url("a"))).toBeNull();
  placeAt(shown, "a", url("a"));
  expect(await shown.freeze("a", url("a"))).toEqual(pages()[0]!.frozen);
  shown.thaw("a", url("a"));
  expect(pages()[0]!.events).toEqual([
    "show",
    "freeze",
    "thaw",
  ]);
});

it("outlives Bridge's window: with none up the walk is held, and a new window's pane places it again", () => {
  const shown = holding(record("a", walk("a", at("09:01"))));
  placeAt(shown, "a", url("a"));
  host = null;
  expect(pages()[0]!.seat.pane.host()).toBeNull();
  host = HOST;
  expect(pages()[0]!.seat.pane.host()).toBe(HOST);
  expect(pages()[0]!.events).not.toContain("dispose");
});

it("closes every walk it holds with the app, each told to Fleet as closed and none as approved", () => {
  const shown = holding(record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02"))));
  placeAt(shown, "a", url("a"));
  shown.closeAll();
  expect(pages()[0]!.events).toContain("dispose");
  expect(board.sessionClosed.mock.calls.map(([session]) => session).sort()).toEqual([
    "a",
    "b",
  ]);
  expect(board.sessionApproved).not.toHaveBeenCalled();
});
