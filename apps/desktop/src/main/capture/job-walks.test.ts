// The walks Jobs showed, as `JobWalks` holds them with a stand-in page: a page is made when its pane first reports a frame, parked when the frame
// goes, rebuilt in place when the Job shows it again, and let go once, with its owner told once.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));
vi.mock("./views", async () => (await import("./pane.fake")).fakeViews);

import { CHANNELS } from "../../shared/bridge";
import { FRAME, HOST } from "./pane.fake";
import { JobWalks, type JobWalk } from "./job-walks";
import { sessionWalkKey } from "./queue";
import type { CaptureWindow, WalkSeat } from "./window";

type Page = {
  seat: WalkSeat;
  events: string[];
  frozen: unknown;
};

const pages: Page[] = [];
const board = {
  shown: vi.fn(),
  approved: vi.fn(),
};
let host: unknown = HOST;
const forward = vi.fn(() => true);
const url = (name: string) => `https://${name}.example.com/app`;

function walks(): JobWalks {
  return new JobWalks(() => host as never, forward, board);
}

/** A Job's walk whose page is a stand-in that records what it is told. */
function walkOf(session: string, name = session, since = "2026-10-19T09:00:00Z"): JobWalk {
  return {
    key: sessionWalkKey(session, url(name)),
    since,
    sessionId: session,
    url: url(name),
    make: (seat) => {
      const page: Page = {
        seat,
        events: [],
        frozen: {
          page: "p.png",
          bar: "b.png",
        },
      };
      pages.push(page);
      return {
        show: () => page.events.push("show"),
        hide: () => page.events.push("hide"),
        resized: () => page.events.push("resized"),
        dispose: () => page.events.push("dispose"),
        thaw: () => page.events.push("thaw"),
        freeze: () => {
          page.events.push("freeze");
          return Promise.resolve(page.frozen);
        },
        owns: (contents: unknown) => contents === seat.bar.webContents,
      } as unknown as CaptureWindow;
    },
    walk: null,
    seen: false,
    left: vi.fn(),
    shape: "one",
  };
}

const place = (held: JobWalks, session: string, name = session, frame: unknown = FRAME) =>
  held.place(HOST, session, url(name), frame);

beforeEach(() => {
  pages.length = 0;
  host = HOST;
  board.shown.mockClear();
  board.approved.mockClear();
  forward.mockClear();
});

it("holds a walk without making its page, and makes it where its pane first reports a frame, once", () => {
  const held = walks();
  held.add(walkOf("a"));
  expect(pages).toHaveLength(0);
  expect(held.made).toHaveLength(0);
  place(held, "a");
  place(held, "a");
  expect(pages).toHaveLength(1);
  expect(held.made).toHaveLength(1);
  expect(board.shown).toHaveBeenCalledTimes(1);
  expect(pages[0]!.events).toEqual([
    "show",
    "resized",
  ]);
});

it("ignores a walk added again, and keeps the order walks were added in", () => {
  const held = walks();
  held.add(walkOf("b"));
  held.add(walkOf("a"));
  held.add(walkOf("b", "b", "later"));
  expect(held.queued.map((one) => one.sessionId)).toEqual([
    "b",
    "a",
  ]);
  expect(held.queued[0]!.since).toBe("2026-10-19T09:00:00Z");
});

it("remembers a frame for a walk that arrives after its pane, and makes its page on arrival", () => {
  const held = walks();
  place(held, "a");
  expect(pages).toHaveLength(0);
  held.add(walkOf("a"));
  expect(pages).toHaveLength(1);
});

it("parks a walk on unplace and keeps its page, and shows it where its pane returns", () => {
  const held = walks();
  held.add(walkOf("a"));
  place(held, "a");
  held.unplace("a", url("a"));
  held.unplace("a", url("a"));
  expect(pages[0]!.events).toEqual([
    "show",
    "hide",
  ]);
  place(held, "a");
  expect(pages).toHaveLength(1);
  expect(pages[0]!.events).toEqual([
    "show",
    "hide",
    "show",
  ]);
});

it("makes a replaced walk's page afresh in its place, in its frame and with its first time", () => {
  const held = walks();
  held.add(walkOf("a", "a", "first"));
  place(held, "a");
  held.replace({
    ...walkOf("a", "a", "second"),
    shape: "two",
  });
  expect(pages).toHaveLength(2);
  expect(pages[0]!.events).toContain("dispose");
  expect(held.queued).toHaveLength(1);
  expect(held.queued[0]).toMatchObject({
    shape: "two",
    since: "first",
  });
  expect(pages[1]!.seat.pane!.frame()).toEqual(FRAME);
});

it("takes a replaced walk it did not hold as an added one", () => {
  const held = walks();
  held.replace(walkOf("a"));
  expect(held.queued).toHaveLength(1);
});

it("lets a walk go with its page, and tells its owner once", () => {
  const held = walks();
  const one = walkOf("a");
  held.add(one);
  place(held, "a");
  held.remove(one.key);
  held.remove(one.key);
  expect(pages[0]!.events).toContain("dispose");
  expect(one.left).toHaveBeenCalledTimes(1);
  expect(held.queued).toHaveLength(0);
  place(held, "a");
  expect(pages).toHaveLength(1);
});

it("settles on Approve: its owner is told approved, then the walk goes", () => {
  const held = walks();
  const one = walkOf("a");
  held.add(one);
  place(held, "a");
  const order: string[] = [];
  board.approved.mockImplementation(() => void order.push(`approved:${held.queued.length}`));
  vi.mocked(one.left).mockImplementation(() => void order.push(`left:${held.queued.length}`));
  pages[0]!.seat.settled(null as never);
  expect(order).toEqual([
    "approved:1",
    "left:0",
  ]);
});

it("seats a walk in the pane: no window, no panel, no queue, on screen only while it has a frame", () => {
  const held = walks();
  held.add(walkOf("a"));
  place(held, "a");
  const seat = pages[0]!.seat;
  expect(seat.window).toBeNull();
  expect(seat.panel).toBeNull();
  expect(seat.queue()).toEqual({
    entries: [],
    at: 0,
  });
  expect(seat.onScreen(null as never)).toBe(true);
  expect(seat.pane!.host()).toBe(HOST);
  expect(seat.pane!.forward).toBe(forward);
  held.unplace("a", url("a"));
  expect(seat.onScreen(null as never)).toBe(false);
  expect(seat.pane!.frame()).toBeNull();
  host = null;
  expect(seat.pane!.host()).toBeNull();
});

it("tells the bar of Approve and Reload from the page, and nothing else", () => {
  const held = walks();
  held.add(walkOf("a"));
  place(held, "a");
  const seat = pages[0]!.seat;
  const sent = seat.bar.webContents as unknown as {
    sent: unknown[][];
  };
  seat.key("approve");
  seat.key("reload");
  seat.key("next" as never);
  seat.key("previous" as never);
  expect(sent.sent).toEqual([
    [
      CHANNELS.captureWindowKey,
      "approve",
    ],
    [
      CHANNELS.captureWindowKey,
      "reload",
    ],
  ]);
});

it("holds a walk's pictures still while a layer lies over it, none for a walk without a page", async () => {
  const held = walks();
  held.add(walkOf("a"));
  expect(await held.freeze("a", url("a"))).toBeNull();
  place(held, "a");
  expect(await held.freeze("a", url("a"))).toEqual(pages[0]!.frozen);
  held.thaw("a", url("a"));
  held.thaw("z", url("z"));
  expect(pages[0]!.events).toEqual([
    "show",
    "freeze",
    "thaw",
  ]);
});

it("names the walk whose bar a call came from", () => {
  const held = walks();
  held.add(walkOf("a"));
  held.add(walkOf("b"));
  place(held, "b");
  const bar = pages[0]!.seat.bar.webContents;
  expect(held.from(bar)).toBe(held.find(sessionWalkKey("b", url("b")))!.walk);
  expect(held.from({} as Electron.WebContents)).toBeNull();
});

it("closes every walk with its page, each told once, and forgets the frames", () => {
  const held = walks();
  const [a, b] = [
    walkOf("a"),
    walkOf("b"),
  ];
  held.add(a);
  held.add(b);
  place(held, "a");
  held.closeAll();
  expect(pages[0]!.events).toContain("dispose");
  expect(a.left).toHaveBeenCalledTimes(1);
  expect(b.left).toHaveBeenCalledTimes(1);
  expect(held.queued).toHaveLength(0);
  held.add(walkOf("a"));
  expect(pages).toHaveLength(1);
});

it("holds a frame inside the window it was reported for, and ignores a window that is gone", () => {
  const held = walks();
  held.add(walkOf("a"));
  held.place(
    {
      isDestroyed: () => true,
      getContentBounds: () => ({
        width: 10,
        height: 10,
      }),
    } as never,
    "a",
    url("a"),
    FRAME,
  );
  expect(pages).toHaveLength(0);
  place(held, "a", "a", {
    x: 1100,
    y: 700,
    width: 800,
    height: 500,
  });
  expect(pages[0]!.seat.pane!.frame()).toEqual({
    x: 1100,
    y: 700,
    width: 100,
    height: 100,
  });
});
