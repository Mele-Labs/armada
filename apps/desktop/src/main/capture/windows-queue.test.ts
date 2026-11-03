// The one walk window as `CaptureWindows` drives it with the real `WalkWindow` over a stand-in Electron. **It holds pull requests' walks
// only**: a Job's walk is a tab of the Job, in no window. The queue's order, a walk that arrives, Approve, the window closing, and Bridge's
// window moving to no Job. Each walk's page is a stand-in, which keeps the calls made on it. `docs/practices/capture-window.md`, *One window for every walk*.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

const electron = vi.hoisted(() => ({
  windows: [] as unknown[],
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
    resized() {}
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

import type { SessionRecord, WalkStep } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { placeAt } from "./pane.fake";
import { CaptureWindows } from "./windows";

type Entry = {
  id: string;
  asking?: string;
  readiness?: string;
  title?: string;
};
type Seat = {
  queue: () => {
    entries: Entry[];
    at: number;
  };
  settled: (page: unknown) => void;
};
type Page = {
  seat: Seat;
  events: string[];
  questions: unknown[];
  readiness: unknown[];
  approve: () => void;
};
type Window = {
  calls: string[];
  destroyed: boolean;
  emit: (event: string) => void;
  close: () => void;
};

const pages = () => made.pages as Page[];
const windows_ = () => electron.windows as Window[];
const lastWindow = () => windows_().at(-1)!;
/** What the bar of the newest walk window is told: the queue, and the place on screen. */
const bar = () => pages().at(-1)!.seat.queue();
const onScreen = () => {
  const { entries, at } = bar();
  return entries[at]?.id;
};
const ids = () => bar().entries.map((one) => one.id);
const BAR = {
  id: 1,
} as Electron.WebContents;

const at = (time: string) => `2026-10-19T${time}:00Z`;
const url = (name: string) => `https://${name}.example.com/app`;
const REVIEW = (number: number) => `review:armada#${number}`;

const record = (id: string): SessionRecord => ({
  id,
  harness: "h",
  origin: "bridge",
  cwd: "/repo",
  state: "live",
  started_at: at("08:00"),
  last_seen_at: at("08:00"),
  usage: {},
  attachments: [],
  waiting_for: [],
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

/** Every member of the board a call reached, by name, a member the board does not declare included. */
let reached: string[] = [];
const recording = new Proxy(board, {
  get: (target, name) => {
    const known = Reflect.get(target, name) as ((...args: unknown[]) => unknown) | undefined;
    return (...args: unknown[]) => (reached.push(String(name)), known?.(...args));
  },
});

/** Bridge holding these Jobs. The walk window is not up until a pull request's walk is opened. */
function holding(...sessions: SessionRecord[]): CaptureWindows {
  const shown = new CaptureWindows(
    recording as unknown as ConstructorParameters<typeof CaptureWindows>[0],
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

/** A pull request's walk, shown at `time`. */
const review = (shown: CaptureWindows, number: number, time: string) => {
  vi.setSystemTime(new Date(at(time)));
  return shown.openForReview({
    walkId: `w${number}`,
    manifestId: "armada",
    number,
    title: `#${number} Take a draft out`,
    address: `https://pr-${number}.preview.invalid/app`,
    steps: STEPS,
  });
};

beforeEach(() => {
  made.pages = [];
  electron.windows = [];
  reached = [];
  for (const one of Object.values(board)) one.mockClear();
  vi.useFakeTimers({
    toFake: [
      "Date",
    ],
  });
});
afterEach(() => vi.useRealTimers());

it("queues pull requests' walks oldest first, each an entry of the one window", async () => {
  const shown = holding();
  await review(shown, 12, "09:03");
  await review(shown, 7, "09:01");
  await review(shown, 9, "09:02");
  expect(ids()).toEqual([
    REVIEW(7),
    REVIEW(9),
    REVIEW(12),
  ]);
  expect(windows_()).toHaveLength(1);
});

it("opens the window behind on the first walk, and takes no focus", async () => {
  await review(holding(), 12, "09:00");
  expect(windows_()).toHaveLength(1);
  expect(lastWindow().calls).toEqual([
    "showInactive",
  ]);
  expect(onScreen()).toBe(REVIEW(12));
});

it("joins a walk that arrives into the open window without changing the walk on screen, raising or focusing it", async () => {
  const shown = holding();
  await review(shown, 12, "09:00");
  await review(shown, 7, "09:05");
  expect(ids()).toEqual([
    REVIEW(12),
    REVIEW(7),
  ]);
  expect(onScreen()).toBe(REVIEW(12));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).not.toContain("hide");
  expect(windows_()).toHaveLength(1);
  expect(lastWindow().calls).toEqual([
    "showInactive",
  ]);
});

it("makes a walk's page when it first comes on screen, and keeps it on jumping away and back", async () => {
  const shown = holding();
  await review(shown, 12, "09:00");
  await review(shown, 7, "09:05");
  expect(pages()).toHaveLength(1);
  shown.walkTo(BAR, 1);
  expect(pages()).toHaveLength(2);
  expect(onScreen()).toBe(REVIEW(7));
  shown.walkTo(BAR, 0);
  expect(pages()).toHaveLength(2);
  expect(pages()[0]!.events).not.toContain("dispose");
  expect(pages()[1]!.events).not.toContain("dispose");
});

it("moves to the next walk on Approve, drops the page that left, and closes the window with the last one", async () => {
  const shown = holding();
  await review(shown, 12, "09:00");
  await review(shown, 7, "09:05");
  pages()[0]!.approve();
  expect(pages()[0]!.events).toContain("dispose");
  expect(board.reviewClosed).toHaveBeenCalledTimes(1);
  expect(ids()).toEqual([
    REVIEW(7),
  ]);
  expect(lastWindow().destroyed).toBe(false);
  pages()[1]!.approve();
  expect(lastWindow().destroyed).toBe(true);
  expect(board.reviewClosed).toHaveBeenCalledTimes(2);
  // A pull request's walk is no Job's: Fleet is told of no Job's window.
  expect(board.sessionClosed).not.toHaveBeenCalled();
});

it("settles nothing when the window closes, and the next walk opens one window with the walks still held", async () => {
  const shown = holding();
  await review(shown, 12, "09:00");
  await review(shown, 7, "09:05");
  lastWindow().close();
  expect(lastWindow().destroyed).toBe(true);
  expect(board.reviewClosed).toHaveBeenCalledTimes(2);
  expect(board.sessionApproved).not.toHaveBeenCalled();
  expect(board.answerAsk).not.toHaveBeenCalled();
  await review(shown, 9, "09:10");
  expect(windows_()).toHaveLength(2);
  expect(ids()).toEqual([
    REVIEW(9),
  ]);
});

// The owner, 19 Oct 2026: "When I leave an annotation, it shouldnt take me anywhere." Asked when the Cockpit should move to the
// walk's Job, he answered Never. A walk coming on screen or the window taking focus tells Bridge's window nothing but the dimming.
it("moves Bridge's window to no Job as a walk comes on screen, the window takes focus, or Approve moves on", async () => {
  const shown = holding();
  await review(shown, 12, "09:00");
  await review(shown, 7, "09:05");
  lastWindow().emit("focus");
  shown.walkTo(BAR, 1);
  lastWindow().emit("blur");
  pages()[1]!.approve();
  // The approved walk's own ending is told; nothing moves Bridge's window.
  expect(new Set(reached)).toEqual(
    new Set([
      "focused",
      "reviewClosed",
    ]),
  );
  expect(board.focused.mock.calls).toEqual([
    [
      "walks",
      true,
    ],
    [
      "walks",
      false,
    ],
  ]);
});

it("holds no Job's walk: one shown opens no window of its own, and takes no entry beside a pull request's", async () => {
  const shown = holding(record("a"));
  shown.openForSession("a", "Walk a", url("a"));
  expect(windows_()).toHaveLength(0);
  await review(shown, 12, "09:00");
  const queue = pages()[0]!.seat.queue;
  shown.openForSession("b", "Walk b", url("b"));
  placeAt(shown, "a", url("a"));
  // The Job's page is made in its pane, a second page and no second window; the window's queue is still the pull request's alone.
  expect(pages()).toHaveLength(2);
  expect(windows_()).toHaveLength(1);
  expect(lastWindow().calls).toEqual([
    "showInactive",
  ]);
  expect(queue().entries.map((one) => one.id)).toEqual([
    REVIEW(12),
  ]);
});
