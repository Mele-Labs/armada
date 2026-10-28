// The real `CaptureWindow` in a walk window's seat, over stand-in views, asked a question about a place on its page.
// A question that Fleet takes off while its Job works and puts back must not take the keyboard from the box the owner is
// typing in: it comes forward once, and never with focus.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  BaseWindow: class {},
  screen: { getPrimaryDisplay: () => ({ workAreaSize: { width: 1920 } }) },
  shell: {},
}));
vi.mock("./views", () => ({ barView: () => view(), pageView: () => view(), panelView: () => view() }));

import type { WalkSeat } from "./window";
import { CaptureWindow, type WindowQuestion } from "./window";

const ADDRESS = "https://app.example.com/page";

/** A view whose page does nothing and is always at the address it was shown. */
function view() {
  return {
    setBounds: () => {},
    setVisible: () => {},
    webContents: {
      id: 1,
      on: () => {},
      send: () => {},
      isDestroyed: () => false,
      close: () => {},
      setWindowOpenHandler: () => {},
      getURL: () => ADDRESS,
      loadURL: () => Promise.resolve(),
      executeJavaScript: () => Promise.resolve(null),
    },
  };
}

/** The calls the walk window's native window was sent, in order. */
let calls: string[];
const native = () => ({
  contentView: { addChildView: () => {}, removeChildView: () => {} },
  getContentBounds: () => ({ width: 1280, height: 900 }),
  isDestroyed: () => false,
  isMinimized: () => false,
  setTitle: () => {},
  on: () => {},
  showInactive: () => calls.push("showInactive"),
  show: () => calls.push("show"),
  restore: () => calls.push("restore"),
  focus: () => calls.push("focus"),
});

function walkWindow(): CaptureWindow {
  const seat = {
    window: native(),
    bar: view(),
    panel: null,
    onScreen: () => true,
    queue: () => ({ entries: [], at: 0 }),
    key: () => {},
    settled: () => {},
  } as unknown as WalkSeat;
  const pin = { run: "run", name: "app", origin: "https://app.example.com", url: ADDRESS, manifestId: null, through: [] };
  const board = { focused: () => {}, heldOn: () => 0 } as never;
  return new CaptureWindow(pin, { session: { id: "job" } } as never, board, { bar: 40, panel: 300, floor: 100 }, null, [], [], [], seat);
}

const ask = (id: string): WindowQuestion => ({ id, question: "Is this right?", options: [], place: { url: ADDRESS, selector: "#box" } });

beforeEach(() => {
  calls = [];
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());

it("brings the window forward once for a question that is taken off and put back, and never with focus", () => {
  const page = walkWindow();
  page.question(ask("q1"));
  expect(calls).toEqual(["showInactive"]);
  // Fleet drops the waiting item while the Job works, then lists it again.
  page.question(null);
  page.question(ask("q1"));
  page.question(null);
  page.question(ask("q1"));
  expect(calls).toEqual(["showInactive"]);
  expect(calls).not.toContain("focus");
});

it("brings the window forward again for a different question, still without focus", () => {
  const page = walkWindow();
  page.question(ask("q1"));
  page.question(null);
  page.question(ask("q2"));
  expect(calls).toEqual(["showInactive", "showInactive"]);
});

it("neither shows nor focuses the window for a question in a walk it did not come to", () => {
  const page = walkWindow();
  page.question(ask("q1"), false);
  page.question(null);
  page.question(ask("q1"), false);
  expect(calls).toEqual([]);
});
