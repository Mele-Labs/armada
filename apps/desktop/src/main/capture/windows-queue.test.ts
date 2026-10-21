// The one walk window as `CaptureWindows` drives it with the real `WalkWindow` over a stand-in Electron: the queue's order, a walk
// that arrives, Approve, the window closing, a question for a walk not on screen, and Bridge's window moving to no Job. Each walk's page is a
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

import type { SessionRecord, WaitingItem, WalkStep } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { sessionWalkKey } from "./queue";
import { CaptureWindows } from "./windows";

type Entry = { id: string; asking?: string; readiness?: string; title?: string };
type Seat = { queue: () => { entries: Entry[]; at: number }; settled: (page: unknown) => void };
type Page = { seat: Seat; events: string[]; questions: unknown[]; readiness: unknown[]; approve: () => void };
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

/** The Job's record with the window it showed at `name`'s address, which Fleet marks working once notes were sent on it. */
const showing = (job: SessionRecord, name: string, working: boolean, title = `Walk ${name}`): SessionRecord => ({
  ...job,
  attachments: [
    {
      kind: "artifact",
      target: url(name),
      state: "standing",
      detail: { form: "window", title, ...(working ? { working: "true", changed: "true" } : {}) },
      since: at("09:00"),
      changed_at: at("09:00"),
    },
  ],
});

const state = (...sessions: SessionRecord[]) => ({ ...NOTHING_YET, sessions: { state: "read" as const, sessions } });

const board = {
  windowsChanged: vi.fn(),
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

/** Bridge holding these Jobs. The walk window is not up until a walk is opened. */
function holding(...sessions: SessionRecord[]): CaptureWindows {
  const shown = new CaptureWindows(recording as unknown as ConstructorParameters<typeof CaptureWindows>[0]);
  shown.changed(state(...sessions));
  return shown;
}

/** Three Jobs, a walk each, shown in the order a, b, c. */
const three = () => [record("a", walk("a", at("09:01"))), record("b", walk("b", at("09:02"))), record("c", walk("c", at("09:03")))];

beforeEach(() => {
  made.pages = [];
  electron.windows = [];
  reached = [];
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

// The owner, 21 Oct 2026: "When I send notes, it shouldn't immediately jump me to the next walk." Sending notes marks the
// window working, and Fleet's walk item goes until the Job shows the page again.
it("keeps a walk on screen, reading working, while its item is gone and its window works on the notes sent", () => {
  const shown = holding(showing(record("a", walk("a", at("09:01"))), "a", false), record("b", walk("b", at("09:02"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.changed(state(showing(record("a"), "a", true), record("b", walk("b", at("09:02")))));
  expect(ids()).toEqual([key("a"), key("b")]);
  expect(onScreen()).toBe(key("a"));
  expect(pages()).toHaveLength(1);
  expect(pages()[0]!.events).not.toContain("dispose");
  expect(pages()[0]!.readiness.at(-1)).toBe("working");
  expect(bar().entries.map((one) => one.readiness)).toEqual(["working", undefined]);
  expect(board.sessionClosed).not.toHaveBeenCalled();
});

it("lets a walk go whose item went while its window reads ready, and the walk after it comes on screen", () => {
  const shown = holding(showing(record("a", walk("a", at("09:01"))), "a", false), record("b", walk("b", at("09:02"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.changed(state(showing(record("a"), "a", false), record("b", walk("b", at("09:02")))));
  expect(ids()).toEqual([key("b")]);
  expect(onScreen()).toBe(key("b"));
  expect(pages()[0]!.events).toContain("dispose");
});

it("lets a kept walk go once its window reads ready with its item still gone: the Job's turn ended without showing it again", () => {
  const shown = holding(showing(record("a", walk("a", at("09:01"))), "a", false), record("b", walk("b", at("09:02"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.changed(state(showing(record("a"), "a", true), record("b", walk("b", at("09:02")))));
  shown.changed(state(showing(record("a"), "a", false), record("b", walk("b", at("09:02")))));
  expect(ids()).toEqual([key("b")]);
  expect(onScreen()).toBe(key("b"));
});

it("makes a kept walk's page afresh in its place, reading ready, when the Job shows it again", () => {
  const shown = holding(showing(record("a", walk("a", at("09:01"))), "a", false), record("b", walk("b", at("09:02"))));
  shown.openForSession("a", "Walk a", url("a"));
  shown.changed(state(showing(record("a"), "a", true), record("b", walk("b", at("09:02")))));
  shown.changed(state(showing(record("a", walk("a", at("09:10"))), "a", false, "Walk a, again"), record("b", walk("b", at("09:02")))));
  expect(ids()).toEqual([key("a"), key("b")]);
  expect(onScreen()).toBe(key("a"));
  expect(pages()[0]!.events).toContain("dispose");
  expect(pages()).toHaveLength(2);
  expect(pages()[1]!.readiness.at(-1)).toBe("ready");
  expect(bar().entries[0]).toMatchObject({ title: "Walk a, again", readiness: "ready" });
  // Shown again, it is no longer kept: its item going while ready lets it go.
  shown.changed(state(showing(record("a"), "a", false), record("b", walk("b", at("09:02")))));
  expect(ids()).toEqual([key("b")]);
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

// The owner, 19 Oct 2026: "When I leave an annotation, it shouldnt take me anywhere." Asked when the Cockpit should move to the
// walk's Job, he answered Never. A walk coming on screen or the window taking focus tells Bridge's window nothing but the dimming.
it("moves Bridge's window to no Job as a Job's walk comes on screen, the window takes focus, or Approve moves on", () => {
  const shown = holding(...three());
  shown.openForSession("a", "Walk a", url("a"));
  shown.walkTo(BAR, 1);
  lastWindow().emit("focus");
  lastWindow().emit("blur");
  pages()[1]!.approve();
  expect(onScreen()).toBe(key("c"));
  // The approved walk's own ending is told; nothing moves Bridge's window.
  expect(new Set(reached)).toEqual(new Set(["windowsChanged", "focused", "sessionClosed"]));
  expect(board.focused.mock.calls).toEqual([["walks", true], ["walks", false]]);
});

it("moves Bridge's window to no Job while a pull request's walk is on screen", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(at("09:00")));
  const shown = holding(record("a", walk("a", at("09:01"))));
  const steps: WalkStep[] = [{ say: "Open the list", press: { role: "button", name: "Drafts" } }];
  await shown.openForReview({ walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "https://pr-12.preview.invalid/app", steps });
  lastWindow().emit("focus");
  shown.walkTo(BAR, 1);
  expect(onScreen()).toBe(key("a"));
  expect(new Set(reached)).toEqual(new Set(["windowsChanged", "focused"]));
});
