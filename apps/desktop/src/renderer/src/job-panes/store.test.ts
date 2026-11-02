// What the panes store keeps: each Job's arrangement, the named layouts and the one new Jobs open on. A "restart" is the modules loaded again over the same browser storage.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type { Arrangement } from "./tree";

// The store reads and writes only through the kept values, so those alone are loaded, afresh at each start: the copy a restarted window holds is empty, and the browser's storage is what it finds.

/** A Storage whose keys list as its own properties, as the browser's does. */
function storage(): Storage {
  const items = new Map<string, string>();
  const target: Record<string, unknown> = {};
  const methods = {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => {
      items.set(key, value);
      target[key] = value;
    },
    removeItem: (key: string) => {
      items.delete(key);
      delete target[key];
    },
  };
  return new Proxy(target, { get: (_, key: string) => (key in methods ? methods[key as keyof typeof methods] : items.get(key)) }) as unknown as Storage;
}

const KEEP = new URL("../../../../../../packages/components/src/keep.ts", import.meta.url).pathname;

let kept: Storage;

beforeEach(() => {
  kept = storage();
  vi.stubGlobal("window", { localStorage: kept, addEventListener: () => undefined });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

/** The store and tree as a freshly started window loads them. */
async function start() {
  vi.resetModules();
  vi.doMock("@armada/components", () => import(/* @vite-ignore */ KEEP));
  const [store, tree] = await Promise.all([import("./store"), import("./tree")]);
  return { ...store, ...tree };
}

/** What was kept of a Job's panes, which says whether it is edited and what layout it is on. */
const entryOf = (id: string) => JSON.parse(kept.getItem(`armada:kept:job-panes:job:${id}`)!) as { layout: string; edited: boolean };

const kinds = (arrangement: Arrangement, panes: (a: Arrangement) => { tabs: readonly { kind: string }[] }[]) => panes(arrangement).map((pane) => pane.tabs.map((tab) => tab.kind));

describe("layouts", () => {
  test("a Job opens on Default: its thread with Done, Now and Open stacked to the right", async () => {
    const s = await start();
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))).toEqual([["thread"], ["done"], ["now"], ["open"]]);
  });

  test("rearranging marks the Job edited, Update keeps it as the layout for every Job not edited, and Reset takes it back", async () => {
    const s = await start();
    s.peek("a");
    s.peek("b");
    s.rearrange("a", (x) => s.addTab(x, x.focus, "terminal"));
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toContain("terminal");
    s.updateLayout("a");
    // Default after its update: a Job that was not edited follows it.
    expect(kinds(s.peek("b"), (x) => s.panesOf(x.root))[0]).toContain("terminal");

    s.rearrange("b", (x) => s.closePane(x, x.focus));
    s.resetLayout("b");
    expect(kinds(s.peek("b"), (x) => s.panesOf(x.root))[0]).toContain("terminal");
  });

  test("Save as names the arrangement, refuses a name taken or Default, and another Job can take it", async () => {
    const s = await start();
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    s.saveLayoutAs("a", "  Files  ");
    expect(s.nameTaken("Files")).toBe(true);
    expect(s.nameTaken("Default")).toBe(true);
    expect(s.nameTaken("Other")).toBe(false);
    s.saveLayoutAs("a", "Default");
    expect(entryOf("a").layout).toBe("Files");
    s.takeLayout("b", "Files");
    expect(kinds(s.peek("b"), (x) => s.panesOf(x.root))[0]).toContain("files");
  });

  test("the layout new Jobs open on is followed by a Job nobody arranged, and not by one that was", async () => {
    const s = await start();
    s.rearrange("arranged", (x) => s.addTab(x, x.focus, "files"));
    s.peek("plain");
    s.saveLayoutAs("arranged", "Wide");
    s.setLayoutForAll("Wide");
    // A Job first seen after the change, and one seen before it that nobody chose a layout for, both open on it.
    expect(kinds(s.peek("fresh"), (x) => s.panesOf(x.root))[0]).toContain("files");
    expect(kinds(s.peek("plain"), (x) => s.panesOf(x.root))[0]).toContain("files");
    s.takeLayout("chosen", "Default");
    s.setLayoutForAll("Default");
    s.setLayoutForAll("Wide");
    expect(kinds(s.peek("chosen"), (x) => s.panesOf(x.root))).toEqual([["thread"], ["done"], ["now"], ["open"]]);
  });
});

describe("a tab the Job opens", () => {
  test("one that joins a pane leaves the layout as it was, and one that makes a split is arrangement", async () => {
    const s = await start();
    s.rearrange("a", (x) => s.addTab(x, x.focus, "browser"));
    s.updateLayout("a");
    s.jobOpens("a", "browser", "localhost:5173");
    // Joined the pane holding a browser: content, so the Job is not edited.
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual(["thread", "browser", "browser"]);
    expect(entryOf("a").edited).toBe(false);

    s.takeLayout("c", "Default");
    s.jobOpens("c", "terminal", "zsh");
    expect(kinds(s.peek("c"), (x) => s.panesOf(x.root)).at(-1)).toEqual(["terminal"]);
    expect(entryOf("c").edited).toBe(true);
  });
});

describe("undo", () => {
  test("takes back the last change and puts it back, and a run of one kind of change is one step", async () => {
    const s = await start();
    s.peek("a");
    const before = s.peek("a");
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    s.rearrange("a", (x) => s.addTab(x, x.focus, "plan"));
    s.undo("a");
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual(["thread", "files"]);
    s.undo("a");
    expect(s.peek("a")).toEqual(before);
    s.redo("a");
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual(["thread", "files"]);

    const split = s.rectsOf(s.peek("a").root).splits[0]!.id;
    s.rearrange("a", (x) => s.resizeSplit(x, split, 0.3), "resize");
    s.rearrange("a", (x) => s.resizeSplit(x, split, 0.4), "resize");
    s.undo("a");
    expect((s.rectsOf(s.peek("a").root).splits[0]!).ratio).toBeCloseTo(0.62);
  });

  test("is this window's: a restart starts with nothing to take back", async () => {
    const s = await start();
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    const later = await start();
    later.undo("a");
    expect(kinds(later.peek("a"), (x) => later.panesOf(x.root))[0]).toEqual(["thread", "files"]);
  });
});

describe("minimum sizes", () => {
  test("a change that squeezes a pane below its minimum in a measured area is refused, and one forced, as a Job opening a tab is, is not", async () => {
    const s = await start();
    // Room for the thread and one pane beside it, and no more.
    s.measured("a", { w: s.MIN_PANE_W * 2 + 1, h: s.MIN_PANE_H * 8 });
    const split = (x: Arrangement) => s.splitPane(x, x.focus, "right");
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    s.rearrange("a", (x) => s.moveTabToEdge(x, s.panesOf(x.root)[0]!.tabs[1]!.id, x.focus, "right"));
    const crowded = s.panesOf(s.peek("a").root).length;
    s.rearrange("a", split);
    expect(s.panesOf(s.peek("a").root).length).toBe(crowded);
    s.rearrange("a", split, undefined, true);
    expect(s.panesOf(s.peek("a").root).length).toBe(crowded + 1);
  });

  test("whether a drop can be made reads the same rule", async () => {
    const s = await start();
    s.measured("a", { w: s.MIN_PANE_W * 2 + 1, h: s.MIN_PANE_H * 8 });
    const arrangement = s.peek("a");
    const tab = s.panesOf(arrangement.root)[1]!.tabs[0]!.id;
    expect(s.droppableEdge("a", tab, "right")).toBe(false);
    s.measured("a", { w: s.MIN_PANE_W * 20, h: s.MIN_PANE_H * 8 });
    expect(s.droppableEdge("a", tab, "right")).toBe(true);
  });
});

describe("kept across a restart", () => {
  test("each Job's own arrangement, the saved layouts and the one new Jobs open on come back", async () => {
    const first = await start();
    first.rearrange("a", (x) => first.addTab(x, x.focus, "files"));
    first.saveLayoutAs("a", "Files");
    first.setLayoutForAll("Files");
    first.rearrange("b", (x) => first.addTab(x, x.focus, "plan"));

    const second = await start();
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))[0]).toEqual(["thread", "files"]);
    // Seen after the layout new Jobs open on was changed, so it opened on that one and then took a plan tab.
    expect(kinds(second.peek("b"), (x) => second.panesOf(x.root))[0]).toEqual(["thread", "files", "plan"]);
    expect(second.nameTaken("Files")).toBe(true);
    // A Job never seen before opens on the layout new Jobs were left on.
    expect(kinds(second.peek("new"), (x) => second.panesOf(x.root))[0]).toEqual(["thread", "files"]);
  });

  test("an edited Job comes back edited, and Default after Update comes back updated", async () => {
    const first = await start();
    first.rearrange("a", (x) => first.addTab(x, x.focus, "files"));
    first.updateLayout("a");
    first.rearrange("a", (x) => first.addTab(x, x.focus, "plan"));
    const second = await start();
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))[0]).toEqual(["thread", "files", "plan"]);
    expect(kinds(second.peek("other"), (x) => second.panesOf(x.root))[0]).toEqual(["thread", "files"]);
  });

  test("a Custom tab keeps the name of its mod on the tab itself", async () => {
    const first = await start();
    const bound = JSON.parse(JSON.stringify(first.addTab(first.peek("a"), first.peek("a").focus, "custom"))) as Arrangement;
    (first.panesOf(bound.root)[0]!.tabs.at(-1) as { mod?: string }).mod = "review-queue";
    first.rearrange("a", () => bound);
    const second = await start();
    expect(second.panesOf(second.peek("a").root)[0]!.tabs.at(-1)).toMatchObject({ kind: "custom", mod: "review-queue" });
  });

  test("something kept that is not a layout is not used", async () => {
    kept.setItem("armada:kept:job-panes:job:a", JSON.stringify({ arrangement: { root: { type: "pane", id: "p1", tabs: [{ id: "t1", kind: "nonsense" }] }, focus: "p1", next: 2 }, layout: "Default" }));
    kept.setItem("armada:kept:job-panes:shared", "not json");
    const s = await start();
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))).toEqual([["thread"], ["done"], ["now"], ["open"]]);
  });

  test("forgetting the panes clears what was kept, for every Job", async () => {
    const first = await start();
    first.rearrange("a", (x) => first.addTab(x, x.focus, "files"));
    first.forgetPanes();
    const second = await start();
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))).toEqual([["thread"], ["done"], ["now"], ["open"]]);
  });
});
