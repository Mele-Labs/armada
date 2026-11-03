// A Job's walk as `CaptureWindows.openForSession` makes it, in a pane and in no window: the cookie jar is its repository's,
// the origins it may pass through are bare origins, and a sign-in is handed to the page and kept
// nowhere else. `docs/practices/capture-window.md`, *What it may load* and *What the page can reach back into*.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

const made = vi.hoisted(() => ({
  windows: [] as unknown[][],
  closed: 0,
  raised: 0,
}));
vi.mock("./window", () => ({
  CaptureWindow: class {
    open = true;
    address: string;
    origins: string[] = [];
    constructor(...args: unknown[]) {
      made.windows.push(args);
      this.address = (
        args[0] as {
          url: string;
        }
      ).url;
    }
    raise = () => void (made.raised += 1);
    close = () => void (made.closed += 1);
    dispose = () => void (made.closed += 1);
    show = () => {};
    hide = () => {};
    resized = () => {};
    setReadiness = () => {};
    question = () => {};
  },
}));
vi.mock("./walk-window", async () => ({
  WalkWindow: (await import("./walk-window.fake")).FakeWalkWindow,
}));
vi.mock("./views", async () => (await import("./pane.fake")).fakeViews);

import type { SessionRecord } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { partitionFor, type Pinned } from "./address";
import { placeAt } from "./pane.fake";
import { made as walkWindows } from "./walk-window.fake";
import { CaptureWindows } from "./windows";

const AT = "2026-10-15T09:00:00Z";
const PAGE = "https://staging.example.com/app";

const record = (id: string, manifest_id?: string): SessionRecord => ({
  id,
  harness: "h",
  origin: "bridge",
  ...(manifest_id === undefined
    ? {}
    : {
        manifest_id,
      }),
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  waiting_for: [],
});

function windows(...sessions: SessionRecord[]): CaptureWindows {
  const board = {} as unknown as ConstructorParameters<typeof CaptureWindows>[0];
  const shown = new CaptureWindows(board);
  shown.changed({
    ...NOTHING_YET,
    sessions: {
      state: "read",
      sessions,
    },
  });
  return shown;
}

const pinOf = (at: number): Pinned => made.windows[at]?.[0] as Pinned;

beforeEach(() => {
  made.windows = [];
  made.closed = 0;
  made.raised = 0;
  walkWindows.windows = [];
});

const place = (shown: CaptureWindows, session: string, url: string) =>
  placeAt(shown, session, url);

it("keeps a Session's window in its repository's cookie jar, and two repositories apart", () => {
  const shown = windows(record("a", "01ARMADA"), record("b", "01WIDGETS"), record("c"));
  shown.openForSession("a", "One", PAGE);
  shown.openForSession("b", "Two", PAGE);
  shown.openForSession("c", "Three", PAGE);
  // A walk's page is made as its pane places it, and no window holds them.
  expect(walkWindows.windows).toHaveLength(0);
  place(shown, "a", PAGE);
  place(shown, "b", PAGE);
  place(shown, "c", PAGE);
  expect(pinOf(0).manifestId).toBe("01ARMADA");
  expect(pinOf(1).manifestId).toBe("01WIDGETS");
  expect(pinOf(2).manifestId).toBeNull();
  expect(partitionFor(pinOf(0).manifestId)).not.toBe(partitionFor(pinOf(1).manifestId));
});

it("pins the window to the shown origin and the bare origins named, and drops an entry that is more", () => {
  const shown = windows(record("a", "01ARMADA"));
  shown.openForSession(
    "a",
    "Staging",
    PAGE,
    [],
    [
      "https://login.example.org",
      "https://login.example.org/sso",
      "ftp://x.example.net",
    ],
  );
  place(shown, "a", PAGE);
  expect(pinOf(0)).toMatchObject({
    origin: "https://staging.example.com",
    through: [
      "https://login.example.org",
    ],
  });
});

it("hands the sign-in to the window and to nothing else", () => {
  const shown = windows(record("a", "01ARMADA"));
  const link = "https://staging.example.com/dev/login?token=abc";
  shown.openForSession(
    "a",
    "Staging",
    PAGE,
    [],
    [],
    [
      link,
    ],
  );
  place(shown, "a", PAGE);
  const [pin, , , , , , signIn] = made.windows[0] ?? [];
  expect(signIn).toEqual([
    link,
  ]);
  expect(JSON.stringify(pin)).not.toContain("token=abc");
});

it("opens no walk window for a Job's walk, raises nothing, and leaves a walk shown again where it is", () => {
  const shown = windows(record("a", "01ARMADA"));
  shown.openForSession("a", "Staging", PAGE);
  shown.openForSession("a", "Staging", PAGE);
  expect(walkWindows.windows).toHaveLength(0);
  // The page is not made until a pane has a place for it.
  expect(made.windows).toHaveLength(0);
  place(shown, "a", PAGE);
  shown.openForSession("a", "Staging", PAGE);
  expect([
    walkWindows.windows.length,
    made.windows.length,
    made.raised,
    made.closed,
  ]).toEqual([
    0,
    1,
    0,
    0,
  ]);
});

it("makes a walk's page afresh in its place when it is shown again with a sign-in to run", () => {
  const shown = windows(record("a", "01ARMADA"));
  shown.openForSession("a", "Staging", PAGE);
  place(shown, "a", PAGE);
  shown.openForSession(
    "a",
    "Staging",
    PAGE,
    [],
    [],
    [
      "https://staging.example.com/dev/login",
    ],
  );
  expect([
    made.windows.length,
    made.closed,
  ]).toEqual([
    2,
    1,
  ]);
});

it("keeps two Jobs' walks as two pages in their panes, and neither in a window", () => {
  const shown = windows(record("a", "01ARMADA"), record("b", "01ARMADA"));
  shown.openForSession("b", "Later", PAGE);
  shown.openForSession("a", "Another", "https://staging.example.com/other");
  place(shown, "a", "https://staging.example.com/other");
  expect(made.windows).toHaveLength(1);
  place(shown, "b", PAGE);
  expect(made.windows).toHaveLength(2);
  expect(walkWindows.windows).toHaveLength(0);
});
