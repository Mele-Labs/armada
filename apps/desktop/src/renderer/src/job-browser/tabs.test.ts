// A page for each Browser tab: what the renderer tells main about a Job's tabs, what it holds for each, and the acts that name a tab.

import { beforeEach, expect, test, vi } from "vitest";

import type { JobBrowserState } from "../../../shared/api/job-browser";
import { NO_SETTINGS } from "../../../shared/site-permissions";
import { moveToWindow } from "../job-panes/windows";
import { build } from "../job-panes/tree";
import {
  browserTabIds,
  browserToPane,
  browserToWindow,
  peekJobBrowser,
  reportTabs,
  wireJobBrowsers,
} from "./store";

vi.mock("../job-panes/store", () => ({
  jobOpens: vi.fn(),
  look: vi.fn(),
  peek: vi.fn(),
}));
vi.mock("../cockpit/viewed", () => ({
  viewingNow: vi.fn(() => false),
}));

const state = (tabId: string, url: string): JobBrowserState => ({
  sessionId: "job",
  tabId,
  url,
  title: "",
  back: false,
  forward: false,
  driver: "agent",
  windowed: false,
  site: NO_SETTINGS,
});

let changed: (state: JobBrowserState) => void;
const api = {
  list: async () => [],
  onChanged: (listener: (state: JobBrowserState) => void) => {
    changed = listener;
    return () => undefined;
  },
  onShow: () => () => undefined,
  tabs: vi.fn(async () => undefined),
  show: vi.fn(async () => undefined),
  toWindow: vi.fn(async () => undefined),
};

beforeEach(() => {
  api.tabs.mockClear();
  api.show.mockClear();
  api.toWindow.mockClear();
  wireJobBrowsers(api as unknown as Parameters<typeof wireJobBrowsers>[0]);
});

test("each tab holds its own page: two tabs, two addresses", () => {
  changed(state("t1", "http://localhost:5173"));
  changed(state("t2", "http://localhost:5174"));
  expect(peekJobBrowser("job", "t1")?.url).toBe("http://localhost:5173");
  expect(peekJobBrowser("job", "t2")?.url).toBe("http://localhost:5174");
});

test("main is told the Job's tabs, and what it no longer holds is let go here", () => {
  changed(state("t1", "http://localhost:5173"));
  changed(state("t2", "http://localhost:5174"));
  changed(state("", "http://localhost:5175"));
  reportTabs("job", [
    "t1",
  ]);
  expect(api.tabs).toHaveBeenCalledWith("job", [
    "t1",
  ]);
  expect(peekJobBrowser("job", "t1")).toBeDefined();
  expect(peekJobBrowser("job", "t2")).toBeUndefined();
  expect(peekJobBrowser("job", "")).toBeUndefined();
});

test("a Job's Browser tabs are read in pane order, whichever is in front", () => {
  const arrangement = build({
    dir: "row",
    ratio: 0.5,
    a: [
      "thread",
      "browser",
    ],
    b: {
      dir: "column",
      ratio: 0.5,
      a: [
        "terminal",
      ],
      b: [
        "browser",
        "browser",
      ],
    },
  });
  expect(browserTabIds(arrangement)).toEqual([
    "t4",
    "t9",
    "t10",
  ]);
});

test("Move to window names the tab, and Move to the pane names it back", () => {
  moveToWindow("job", {
    id: "t2",
    kind: "browser",
  });
  expect(api.toWindow).toHaveBeenCalledWith("job", "t2");
  browserToWindow("job", "t3");
  expect(api.toWindow).toHaveBeenLastCalledWith("job", "t3");
  browserToPane("job", "t2");
  expect(api.show).toHaveBeenCalledWith("job", "t2");
});
