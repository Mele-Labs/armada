// The one walk window as `CaptureWindows` drives it with the real `WalkWindow` over a stand-in Electron: the queue's order, a walk
// that arrives, Approve, the window closing, a question for a walk not on screen, and the Job Bridge follows. Each walk's page is a
// stand-in too, which keeps the calls made on it. `docs/practices/capture-window.md`, *One window for every walk*.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

const electron = vi.hoisted(() => ({ windows: [] as unknown[] }));
vi.mock("electron", async () => {
  const { EventEmitter } = await import("node:events");
  class BaseWindow extends EventEmitter {
    calls: string[] = [];
    destroyed = false;
    contentView = { addChildView: () => {} };
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
  return { BaseWindow, screen: { getPrimaryDisplay: () => ({ workAreaSize: { width: 1920 } }) } };
});

// The bar is contents 1 and the panel 2, so a call can be told to come from the bar.
vi.mock("./views", () => {
  const view = (id: number) => () => ({
    webContents: { id, on: () => {}, send: () => {}, isDestroyed: () => false, close: () => {}, focus: () => {} },
    setVisible: () => {},
  });
  return { barView: view(1), panelView: view(2) };
});

const made = vi.hoisted(() => ({ pages: [] as unknown[] }));
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
      this.origins = [(args[0] as { origin: string }).origin];
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
    setReadiness() {}
    question(asked: unknown) {
      this.questions.push(asked);
    }
    // What the bar's Approve ends in once the owner's Job has been told: the walk leaves the window.
    approve() {
      (this.seat as Seat).settled(this);
    }
  },
}));

import type { SessionRecord, WaitingItem, WalkStep } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { sessionWalkKey } from "./queue";
import { CaptureWindows } from "./windows";

type Entry = { id: string; asking?: string };
type Seat = { queue: () => { entries: Entry[]; at: number }; settled: (page: unknown) => void };
type Page = { seat: Seat; events: string[]; questions: unknown[]; approve: () => void };
type Window = { calls: string[]; destroyed: boolean; emit: (event: string) => void; close: () => void };

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
const BAR = { id: 1 } as Electron.WebContents;

const at = (time: string) => `2026-10-19T${time}:00Z`;
const url = (name: string) => `https://${name}.example.com/app`;
const key = (session: string, name = session) => sessionWalkKey(session, url(name));

const walk = (name: string, since: string): WaitingItem => ({
  id: `walk:${name}`,
  text: `Walk ${name}`,
  since,
  source: "walk",
  act: { kind: "walk", target: url(name) },
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

const state = (...sessions: SessionRecord[]) => ({ ...NOTHING_YET, sessions: { state: "read" as const, sessions } });

const board = {
  windowsChanged: vi.fn(),
  sessionClosed: vi.fn(),
  sessionApproved: vi.fn(),
  answerAsk: vi.fn(),
  reviewClosed: vi.fn(),
  focused: vi.fn(),
  follow: vi.fn(),
};

/** Bridge holding these Jobs. The walk window is not up until a walk is opened. */
function holding(...sessions: SessionRecord[]): CaptureWindows {
  const shown = new CaptureWindows(board as unknown as ConstructorParameters<typeof CaptureWindows>[0]);
  shown.changed(state(...sessions));
  return shown;
}

/** Three Jobs, a walk each, shown in the order a, b, c. */
const three = () => [record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02"))), record("c", walk("c", at("09:03")))];

beforeEach(() => {
  made.pages = [];
  electron.windows = [];
  for (const one of Object.values(board)) one.mockClear();
});
afterEach(() => vi.useRealTimers());

it("queues the walks of several Jobs and a pull request's walk oldest first, and a Job's two walks as two entries", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(at("09:04")));
  const shown = holding(
    record("a", walk("a1", at("09:05")), walk("a2", at("09:02"))),
    record("b", walk("b", at("09:01"))),
    record("c", walk("c", at("09:03"))),
  );
  shown.openForSession("b", "Walk b", url("b"));
  const steps: WalkStep[] = [{ say: "Open the list", press: { role: "button", name: "Drafts" } }];
  await shown.openForReview({ walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "https://pr-12.preview.invalid/app", steps });
  expect(ids()).toEqual([key("b"), key("a", "a2"), key("c"), "review:armada#12", key("a", "a1")]);
});

it("opens the window behind on the first walk, and takes no focus", () => {
  holding(...three()).openForSession("a", "Walk a", url("a"));
  expect(windows_()).toHaveLength(1);
  expect(lastWindow().calls).toEqual(["showInactive"]);
  expect(onScreen()).toBe(key("a"));
});

it("joins a walk that arrives into the open window without changing the walk on screen, raising or focusing it", () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(at("09:04")));
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  // One older than the walk on screen arrives from Fleet's publish, another from a Job showing a page.
  shown.changed(state(...three(), record("d", walk("d", at("09:00")))));
  shown.openForSession("e", "Walk e", url("e"));
  expect(ids()).toEqual([key("d"), key("a"), key("b"), key("c"), key("e")]);
  expect(onScreen()).toBe(key("a"));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).not.toContain("hide");
  expect(lastWindow().calls).toEqual(["showInactive"]);
});

it("makes a walk's page when it first comes on screen, and keeps it on jumping away and back", () => {
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  expect(pages()).toHaveLength(1);
  shown.walkTo(BAR, 1);
  expect(pages()).toHaveLength(2);
  expect(onScreen()).toBe(key("b"));
  shown.walkTo(BAR, 0);
  expect(pages()).toHaveLength(2);
  expect(onScreen()).toBe(key("a"));
  expect(pages()[0]!.events).not.toContain("dispose");
  expect(pages()[1]!.events).not.toContain("dispose");
});

it("moves to the next walk on Approve, drops the page that left and tells Fleet its window closed", () => {
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  pages()[0]!.approve();
  expect(pages()[0]!.events).toContain("dispose");
  expect(board.sessionClosed).toHaveBeenCalledTimes(1);
  expect(board.sessionClosed).toHaveBeenCalledWith("a", url("a"));
  expect(ids()).toEqual([key("b"), key("c")]);
  expect(onScreen()).toBe(key("b"));
  // The next one's page is made as it comes on screen, and the one after it is not.
  expect(pages()).toHaveLength(2);
  // Fleet has not settled the item yet: the walk is not queued again by the publish before it does.
  shown.changed(state(...three()));
  expect(ids()).toEqual([key("b"), key("c")]);
});

it("closes the window when the last walk is approved", () => {
  const shown = holding(record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02"))));
  shown.openForSession("a", "Walk a", url("a"));
  pages()[0]!.approve();
  expect(lastWindow().destroyed).toBe(false);
  pages()[1]!.approve();
  expect(lastWindow().destroyed).toBe(true);
  expect(board.sessionClosed.mock.calls).toEqual([["a", url("a")], ["b", url("b")]]);
});

it("settles nothing when the window closes, and the next walk opens one window with the whole queue", () => {
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  shown.walkTo(BAR, 1);
  lastWindow().close();
  expect(lastWindow().destroyed).toBe(true);
  // Every walk in it left, told to Fleet as closed; none was approved or answered.
  expect(board.sessionClosed.mock.calls.map(([session]) => session).sort()).toEqual(["a", "b", "c"]);
  expect(board.sessionApproved).not.toHaveBeenCalled();
  expect(board.answerAsk).not.toHaveBeenCalled();
  // Their items still stand in what Fleet publishes; that alone opens no window.
  shown.changed(state(...three()));
  expect(windows_()).toHaveLength(1);
  shown.openForSession("b", "Walk b", url("b"));
  expect(windows_()).toHaveLength(2);
  expect(lastWindow().calls).toEqual(["showInactive"]);
  expect(ids()).toEqual([key("a"), key("b"), key("c")]);
  expect(onScreen()).toBe(key("b"));
});

it("marks the entry of a walk not on screen that a Job asked about, and leaves the walk on screen alone", () => {
  const asked: WaitingItem = {
    id: "window_ask:q1",
    text: "Is this the right heading?",
    since: at("09:05"),
    source: "window_ask",
    options: [],
    place: { url: url("b"), selector: "main > h1" },
  };
  const shown = holding(record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02")), asked));
  shown.openForSession("a", "Walk a", url("a"));
  expect(shown.askedFor("b", "Walk b", url("b"), "main > h1", "window_ask:q1")).toEqual({ ok: true });
  expect(bar().entries.map((one) => one.asking)).toEqual([undefined, "Is this the right heading?"]);
  expect(onScreen()).toBe(key("a"));
  expect(pages()).toHaveLength(1);
  expect(lastWindow().calls).toEqual(["showInactive"]);
  // Going to the walk shows the question, and the mark goes with it.
  shown.walkTo(BAR, 1);
  expect(pages()[1]!.questions.at(-1)).toMatchObject({ id: "window_ask:q1", question: "Is this the right heading?" });
  expect(bar().entries.map((one) => one.asking)).toEqual([undefined, undefined]);
});

it("follows a Job's walk coming on screen, and the window taking focus, and nothing on a blur", () => {
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  expect(board.follow.mock.calls).toEqual([["a"]]);
  shown.walkTo(BAR, 1);
  expect(board.follow.mock.calls).toEqual([["a"], ["b"]]);
  lastWindow().emit("focus");
  expect(board.follow.mock.calls).toEqual([["a"], ["b"], ["b"]]);
  lastWindow().emit("blur");
  expect(board.follow.mock.calls).toHaveLength(3);
  pages()[1]!.approve();
  expect(board.follow.mock.calls.at(-1)).toEqual(["c"]);
});

it("follows no Job while a pull request's walk is on screen", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(at("09:00")));
  const shown = holding(record("a", walk("a", at("09:01"))));
  const steps: WalkStep[] = [{ say: "Open the list", press: { role: "button", name: "Drafts" } }];
  await shown.openForReview({ walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "https://pr-12.preview.invalid/app", steps });
  expect(ids()).toEqual(["review:armada#12", key("a")]);
  expect(onScreen()).toBe("review:armada#12");
  lastWindow().emit("focus");
  expect(board.follow).not.toHaveBeenCalled();
  shown.walkTo(BAR, 1);
  expect(board.follow.mock.calls).toEqual([["a"]]);
  shown.walkTo(BAR, 0);
  lastWindow().emit("focus");
  expect(board.follow.mock.calls).toEqual([["a"]]);
});
