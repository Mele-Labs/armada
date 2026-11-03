// A Job that shows a window with `steps`, as `CaptureWindows.openForSession` takes them: the walk's page, made when
// its pane places it, is made with the steps, a page shown again with the same steps is left as it is, and one shown with
// other steps (or a sign-in to run) is made afresh in its place.
// `docs/practices/capture-window.md`, *Steps Bridge plays*.

import type { WalkStep } from "@armada/protocol";
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

import { NOTHING_YET } from "../../shared/bridge";
import { placeAt } from "./pane.fake";
import { CaptureWindows } from "./windows";

const PAGE = "http://localhost:5173/depot";

const DEPOT: WalkStep[] = [
  {
    say: "The stock table",
    look: {
      role: "table",
      name: "Stock",
    },
  },
  {
    say: "Open the receiving tab",
    press: {
      role: "tab",
      name: "Receiving",
    },
  },
];

function windows(): CaptureWindows {
  const board = {
    windowsChanged: () => {},
  } as unknown as ConstructorParameters<typeof CaptureWindows>[0];
  const shown = new CaptureWindows(board);
  shown.changed({
    ...NOTHING_YET,
    sessions: {
      state: "read",
      sessions: [],
    },
  });
  return shown;
}

beforeEach(() => {
  made.windows = [];
  made.closed = 0;
  made.raised = 0;
});

it("makes the window with the steps, after the sign-in it may also carry", () => {
  const shown = windows();
  shown.openForSession("a", "Depot", PAGE, [], [], [], DEPOT);
  // Nothing is loaded until the pane has a place for it.
  expect(made.windows).toHaveLength(0);
  placeAt(shown, "a", PAGE);
  const [, , , , , takes, signIn, steps] = made.windows[0] ?? [];
  expect(takes).toEqual([]);
  expect(signIn).toEqual([]);
  expect(steps).toEqual(DEPOT);
});

it("a page shown again with the same steps is left as it is", () => {
  const shown = windows();
  shown.openForSession("a", "Depot", PAGE, [], [], [], DEPOT);
  placeAt(shown, "a", PAGE);
  shown.openForSession("a", "Depot", PAGE, [], [], [], DEPOT);
  expect([
    made.windows.length,
    made.raised,
    made.closed,
  ]).toEqual([
    1,
    0,
    0,
  ]);
});

it("a page shown again with other steps is made afresh with them", () => {
  const shown = windows();
  shown.openForSession("a", "Depot", PAGE, [], [], [], DEPOT);
  placeAt(shown, "a", PAGE);
  const other: WalkStep[] = [
    {
      say: "Only this",
      look: {
        role: "status",
        name: "Saved",
      },
    },
  ];
  shown.openForSession("a", "Depot", PAGE, [], [], [], other);
  expect([
    made.windows.length,
    made.raised,
    made.closed,
  ]).toEqual([
    2,
    0,
    1,
  ]);
  expect(made.windows[1]?.[7]).toEqual(other);
});

it("a page shown again with the same steps and a sign-in to run is made afresh, with both", () => {
  const shown = windows();
  const link = "http://localhost:5173/dev/login";
  shown.openForSession("a", "Depot", PAGE, [], [], [], DEPOT);
  placeAt(shown, "a", PAGE);
  shown.openForSession(
    "a",
    "Depot",
    PAGE,
    [],
    [],
    [
      link,
    ],
    DEPOT,
  );
  expect([
    made.windows.length,
    made.raised,
    made.closed,
  ]).toEqual([
    2,
    0,
    1,
  ]);
  expect(made.windows[1]?.slice(6, 8)).toEqual([
    [
      link,
    ],
    DEPOT,
  ]);
});

it("a page shown without steps is made with none, and a mock that plays its own walk keeps doing so", () => {
  const shown = windows();
  shown.openForSession("a", "Mock", PAGE);
  placeAt(shown, "a", PAGE);
  expect(made.windows[0]?.[7]).toEqual([]);
});
