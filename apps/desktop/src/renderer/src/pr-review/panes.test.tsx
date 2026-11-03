// A pull request's tool tabs: a tool already open comes to the front and never opens a second, and opening one runs it only where the setting no longer asks first. A press on the tab's own Run runs at once.

import { beforeEach, expect, test, vi } from "vitest";

import { forgetPanes, prPanes } from "../job-panes/store";
import { holderOf, panesOf } from "../job-panes/tree";
import type { ReviewPr, ToolDef } from "./model";
import { openToolTab, runIfNotRun } from "./panes";
import { NO_WORK, type ReviewStore } from "./store";

const PR = "Mele-Labs/armada#1899";

const tool = (id: string, over: Partial<ToolDef> = {}): ToolDef => ({
  id,
  kind: "tool",
  name: id,
  source: "armada",
  mark: "risks",
  checks: "",
  agent: true,
  ...over,
});

const risks = tool("risks");
const bugs = tool("possible-bugs");
const how = tool("how-it-works", {
  kind: "explanation",
});

const toolTabs = () =>
  panesOf(prPanes.peek(PR).root)
    .flatMap((pane) => pane.tabs)
    .filter((tab) => tab.kind === "pr-tool");

beforeEach(() => forgetPanes());

test("a tool opens in the tools pane, and one already open comes to the front rather than opening a second", () => {
  openToolTab(PR, risks);
  openToolTab(PR, bugs);
  const holder = holderOf(prPanes.peek(PR), "pr-tool")!;
  expect(toolTabs().map((tab) => tab.page)).toEqual([
    "risks",
    "possible-bugs",
  ]);
  const front = (arrangement = prPanes.peek(PR)) => {
    const pane = holderOf(arrangement, "pr-tool")!;
    return pane.tabs.find((tab) => tab.id === pane.active)?.page;
  };
  expect(front()).toBe("possible-bugs");

  openToolTab(PR, risks);
  expect(toolTabs()).toHaveLength(2);
  expect(front()).toBe("risks");
  expect(holderOf(prPanes.peek(PR), "pr-tool")!.id).toBe(holder.id);
});

test("a tool's tab is content: opening it leaves the layout as it was", () => {
  openToolTab(PR, risks);
  expect(JSON.parse(localStorage.getItem(`armada:kept:pr-panes:pr:${PR}`)!).edited).toBe(false);
});

/** A store that answers what `runIfNotRun` reads and records what it starts. */
function store(askFirst: boolean, work = NO_WORK) {
  const started = {
    explain: vi.fn(),
    runTool: vi.fn(),
    findPlace: vi.fn(),
    spendUnasked: vi.fn((run: () => void, _again: boolean) => run()),
  };
  return {
    ...started,
    store: {
      get: () => ({
        askFirst,
        work: {
          [PR]: work,
        },
      }),
      ...started,
    } as unknown as ReviewStore,
  };
}
const pr = {
  key: PR,
} as ReviewPr;

test("opening a tool that has not run starts nothing while the setting asks first, and runs it once it does not", () => {
  const asking = store(true);
  runIfNotRun(asking.store, pr, risks);
  runIfNotRun(asking.store, pr, how);
  expect(asking.runTool).not.toHaveBeenCalled();
  expect(asking.explain).not.toHaveBeenCalled();

  const free = store(false);
  runIfNotRun(free.store, pr, risks);
  runIfNotRun(free.store, pr, how);
  expect(free.runTool).toHaveBeenCalledWith(PR, risks);
  expect(free.explain).toHaveBeenCalledWith(PR, how);
});

test("opening a tool with a result kept, or one running, runs nothing again", () => {
  const kept = store(false, {
    ...NO_WORK,
    runs: {
      risks: {
        state: "done",
      },
    },
    explains: {
      "how-it-works": {
        state: "running",
      },
    },
  } as unknown as typeof NO_WORK);
  runIfNotRun(kept.store, pr, how);
  expect(kept.explain).not.toHaveBeenCalled();
  // Even a press on Run spends nothing on a tool whose result is kept: the store shows it.
  runIfNotRun(kept.store, pr, risks, {
    again: false,
  });
  expect(kept.spendUnasked).not.toHaveBeenCalled();
});

test("a press on the tab's Run runs at once, and ticking Don't ask again is told to the store", () => {
  const asking = store(true);
  runIfNotRun(asking.store, pr, risks, {
    again: false,
  });
  expect(asking.spendUnasked).toHaveBeenCalledWith(expect.any(Function), false);
  expect(asking.runTool).toHaveBeenCalledWith(PR, risks);

  runIfNotRun(asking.store, pr, how, {
    again: true,
  });
  expect(asking.spendUnasked).toHaveBeenLastCalledWith(expect.any(Function), true);
  expect(asking.explain).toHaveBeenCalledWith(PR, how);
});
