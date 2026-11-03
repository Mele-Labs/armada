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
  return new Proxy(target, {
    get: (_, key: string) =>
      key in methods ? methods[key as keyof typeof methods] : items.get(key),
  }) as unknown as Storage;
}

const KEEP = new URL("../../../../../../packages/components/src/keep.ts", import.meta.url).pathname;

let kept: Storage;

beforeEach(() => {
  kept = storage();
  vi.stubGlobal("window", {
    localStorage: kept,
    addEventListener: () => undefined,
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

/** The store and tree as a freshly started window loads them. */
async function start() {
  vi.resetModules();
  vi.doMock("@armada/components", () => import(/* @vite-ignore */ KEEP));
  const [store, tree] = await Promise.all([
    import("./store"),
    import("./tree"),
  ]);
  return {
    ...store,
    ...tree,
  };
}

/** What was kept of a Job's panes, which says whether it is edited and what layout it is on. */
const entryOf = (id: string) =>
  JSON.parse(kept.getItem(`armada:kept:job-panes:job:${id}`)!) as {
    layout: string;
    edited: boolean;
  };

const kinds = (
  arrangement: Arrangement,
  panes: (a: Arrangement) => {
    tabs: readonly {
      kind: string;
    }[];
  }[],
) => panes(arrangement).map((pane) => pane.tabs.map((tab) => tab.kind));

describe("layouts", () => {
  test("a Job opens on Default: its thread, and to its right Open, Done and Deferred as tabs of one pane over Now", async () => {
    const s = await start();
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))).toEqual([
      [
        "thread",
      ],
      [
        "open",
        "done",
        "deferred",
      ],
      [
        "now",
      ],
    ]);
    // The pane over Now takes the upper 40%, so Now keeps the lower 60%, and Open is the tab in front.
    const arrangement = s.peek("a");
    expect(
      (
        arrangement.root as unknown as {
          b: unknown;
        }
      ).b,
    ).toMatchObject({
      type: "split",
      dir: "column",
      ratio: 0.4,
    });
    const upper = s.panesOf(arrangement.root)[1]!;
    expect(upper.tabs.find((tab) => tab.id === upper.active)?.kind).toBe("open");
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
    expect(kinds(s.peek("chosen"), (x) => s.panesOf(x.root))).toEqual([
      [
        "thread",
      ],
      [
        "open",
        "done",
        "deferred",
      ],
      [
        "now",
      ],
    ]);
  });
});

describe("a pull request's panes", () => {
  const pane = (kind: string) => ({
    root: {
      type: "pane",
      id: "p1",
      tabs: [
        {
          id: "t1",
          kind,
        },
      ],
      active: "t1",
    },
    focus: "p1",
    next: 2,
  });

  test("a pull request opens on Default: Overview, Files, Checks, Commits and Review as one pane, an empty tools pane, and Comments", async () => {
    const s = await start();
    expect(kinds(s.prPanes.peek("pr1"), (x) => s.panesOf(x.root))).toEqual([
      [
        "pr-overview",
        "pr-files",
        "pr-checks",
        "pr-commits",
        "pr-review",
      ],
      [],
      [
        "pr-comments",
      ],
    ]);
  });

  test("its layouts are kept under pr-panes, apart from the Jobs' under job-panes", async () => {
    const s = await start();
    s.prPanes.rearrange("pr1", (x) => s.addTab(x, x.focus, "pr-tool"));
    s.prPanes.saveLayoutAs("pr1", "Wide");
    s.jobPanes.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    s.jobPanes.saveLayoutAs("a", "Tall");
    expect(JSON.parse(kept.getItem("armada:kept:pr-panes:shared")!).saved).toHaveProperty("Wide");
    expect(JSON.parse(kept.getItem("armada:kept:pr-panes:shared")!).saved).not.toHaveProperty(
      "Tall",
    );
    expect(JSON.parse(kept.getItem("armada:kept:job-panes:shared")!).saved).toHaveProperty("Tall");
    expect(JSON.parse(kept.getItem("armada:kept:job-panes:shared")!).saved).not.toHaveProperty(
      "Wide",
    );
    expect(kept.getItem("armada:kept:pr-panes:pr:pr1")).not.toBeNull();
    expect(kept.getItem("armada:kept:job-panes:job:pr1")).toBeNull();
    // A name taken in one family is free in the other, and each lists only its own.
    expect(s.prPanes.nameTaken("Wide")).toBe(true);
    expect(s.jobPanes.nameTaken("Wide")).toBe(false);
    expect(s.jobPanes.nameTaken("Tall")).toBe(true);
    expect(s.prPanes.nameTaken("Tall")).toBe(false);
  });

  test("saving, updating and resetting a layout, and setting what new ones open on, in one family leave the other as it was", async () => {
    const s = await start();
    const jobDefault = kinds(s.jobPanes.peek("job"), (x) => s.panesOf(x.root));
    s.jobPanes.peek("fresh");

    s.prPanes.rearrange("pr1", (x) => s.addTab(x, x.focus, "pr-tool"));
    s.prPanes.updateLayout("pr1");
    s.prPanes.rearrange("pr1", (x) => s.addTab(x, x.focus, "pr-tool"));
    s.prPanes.saveLayoutAs("pr1", "Wide");
    s.prPanes.setLayoutForAll("Wide");
    s.prPanes.rearrange("pr1", (x) => s.closePane(x, x.focus));
    s.prPanes.resetLayout("pr1");

    expect(kinds(s.jobPanes.peek("job"), (x) => s.panesOf(x.root))).toEqual(jobDefault);
    expect(kinds(s.jobPanes.peek("fresh"), (x) => s.panesOf(x.root))).toEqual(jobDefault);
    expect(s.jobPanes.nameTaken("Wide")).toBe(false);
    expect(JSON.parse(kept.getItem("armada:kept:job-panes:shared") ?? "{}").saved).toBeUndefined();
    // A pull request first seen now opens on the layout its own family was set to.
    expect(kinds(s.prPanes.peek("pr2"), (x) => s.panesOf(x.root))[0]).toContain("pr-tool");

    // And the Jobs' layouts, set after, leave the pull requests' as they were.
    s.jobPanes.rearrange("job", (x) => s.addTab(x, x.focus, "files"));
    s.jobPanes.updateLayout("job");
    s.jobPanes.saveLayoutAs("job", "Mine");
    s.jobPanes.setLayoutForAll("Mine");
    expect(kinds(s.prPanes.peek("pr3"), (x) => s.panesOf(x.root))[0]).toContain("pr-tool");
    expect(s.prPanes.nameTaken("Mine")).toBe(false);
    expect(JSON.parse(kept.getItem("armada:kept:pr-panes:shared")!).forAll).toBe("Wide");
  });

  test("a kept tab of a kind outside the family is dropped: a Job's tab in a pull request's panes, and the other way", async () => {
    kept.setItem(
      "armada:kept:pr-panes:pr:a",
      JSON.stringify({
        arrangement: pane("thread"),
        layout: "Default",
      }),
    );
    kept.setItem(
      "armada:kept:job-panes:job:a",
      JSON.stringify({
        arrangement: pane("pr-files"),
        layout: "Default",
      }),
    );
    const s = await start();
    expect(kinds(s.prPanes.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual([
      "pr-overview",
      "pr-files",
      "pr-checks",
      "pr-commits",
      "pr-review",
    ]);
    expect(kinds(s.jobPanes.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual([
      "thread",
    ]);
  });
});

describe("a tab the Job opens", () => {
  test("one that joins a pane leaves the layout as it was, and one that makes a split is arrangement", async () => {
    const s = await start();
    s.rearrange("a", (x) => s.addTab(x, x.focus, "browser"));
    s.updateLayout("a");
    s.jobOpens("a", "browser", "localhost:5173");
    // Joined the pane holding a browser: content, so the Job is not edited.
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual([
      "thread",
      "browser",
      "browser",
    ]);
    expect(entryOf("a").edited).toBe(false);

    s.takeLayout("c", "Default");
    s.jobOpens("c", "terminal");
    expect(kinds(s.peek("c"), (x) => s.panesOf(x.root)).at(-1)).toEqual([
      "terminal",
    ]);
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
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);
    s.undo("a");
    expect(s.peek("a")).toEqual(before);
    s.redo("a");
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);

    const split = s.rectsOf(s.peek("a").root).splits[0]!.id;
    s.rearrange("a", (x) => s.resizeSplit(x, split, 0.3), "resize");
    s.rearrange("a", (x) => s.resizeSplit(x, split, 0.4), "resize");
    s.undo("a");
    expect(s.rectsOf(s.peek("a").root).splits[0]!.ratio).toBeCloseTo(0.62);
  });

  test("is this window's: a restart starts with nothing to take back", async () => {
    const s = await start();
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    const later = await start();
    later.undo("a");
    expect(kinds(later.peek("a"), (x) => later.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);
  });
});

describe("minimum sizes", () => {
  test("a change that squeezes a pane below its minimum in a measured area is refused, and one forced, as a Job opening a tab is, is not", async () => {
    const s = await start();
    // Room for the thread and one pane beside it, and no more.
    s.measured("a", {
      w: s.MIN_PANE_W * 2 + 1,
      h: s.MIN_PANE_H * 8,
    });
    const split = (x: Arrangement) => s.splitPane(x, x.focus, "right");
    s.rearrange("a", (x) => s.addTab(x, x.focus, "files"));
    s.rearrange("a", (x) =>
      s.moveTabToEdge(x, s.panesOf(x.root)[0]!.tabs[1]!.id, x.focus, "right"),
    );
    const crowded = s.panesOf(s.peek("a").root).length;
    s.rearrange("a", split);
    expect(s.panesOf(s.peek("a").root).length).toBe(crowded);
    s.rearrange("a", split, undefined, true);
    expect(s.panesOf(s.peek("a").root).length).toBe(crowded + 1);
  });

  test("whether a drop can be made reads the same rule", async () => {
    const s = await start();
    s.measured("a", {
      w: s.MIN_PANE_W * 2 + 1,
      h: s.MIN_PANE_H * 8,
    });
    const arrangement = s.peek("a");
    const tab = s.panesOf(arrangement.root)[1]!.tabs[0]!.id;
    expect(s.droppableEdge("a", tab, "right")).toBe(false);
    s.measured("a", {
      w: s.MIN_PANE_W * 20,
      h: s.MIN_PANE_H * 8,
    });
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
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);
    // Seen after the layout new Jobs open on was changed, so it opened on that one and then took a plan tab.
    expect(kinds(second.peek("b"), (x) => second.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
      "plan",
    ]);
    expect(second.nameTaken("Files")).toBe(true);
    // A Job never seen before opens on the layout new Jobs were left on.
    expect(kinds(second.peek("new"), (x) => second.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);
  });

  test("an edited Job comes back edited, and Default after Update comes back updated", async () => {
    const first = await start();
    first.rearrange("a", (x) => first.addTab(x, x.focus, "files"));
    first.updateLayout("a");
    first.rearrange("a", (x) => first.addTab(x, x.focus, "plan"));
    const second = await start();
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
      "plan",
    ]);
    expect(kinds(second.peek("other"), (x) => second.panesOf(x.root))[0]).toEqual([
      "thread",
      "files",
    ]);
  });

  test("a Custom tab keeps the name of its mod on the tab itself", async () => {
    const first = await start();
    const bound = JSON.parse(
      JSON.stringify(first.addTab(first.peek("a"), first.peek("a").focus, "custom")),
    ) as Arrangement;
    (
      first.panesOf(bound.root)[0]!.tabs.at(-1) as {
        mod?: string;
      }
    ).mod = "review-queue";
    first.rearrange("a", () => bound);
    const second = await start();
    expect(second.panesOf(second.peek("a").root)[0]!.tabs.at(-1)).toMatchObject({
      kind: "custom",
      mod: "review-queue",
    });
  });

  test("something kept that is not a layout is not used", async () => {
    kept.setItem(
      "armada:kept:job-panes:job:a",
      JSON.stringify({
        arrangement: {
          root: {
            type: "pane",
            id: "p1",
            tabs: [
              {
                id: "t1",
                kind: "nonsense",
              },
            ],
          },
          focus: "p1",
          next: 2,
        },
        layout: "Default",
      }),
    );
    kept.setItem("armada:kept:job-panes:shared", "not json");
    const s = await start();
    expect(kinds(s.peek("a"), (x) => s.panesOf(x.root))).toEqual([
      [
        "thread",
      ],
      [
        "open",
        "done",
        "deferred",
      ],
      [
        "now",
      ],
    ]);
  });

  test("forgetting the panes clears what was kept, for every Job", async () => {
    const first = await start();
    first.rearrange("a", (x) => first.addTab(x, x.focus, "files"));
    first.forgetPanes();
    const second = await start();
    expect(kinds(second.peek("a"), (x) => second.panesOf(x.root))).toEqual([
      [
        "thread",
      ],
      [
        "open",
        "done",
        "deferred",
      ],
      [
        "now",
      ],
    ]);
  });
});

describe("a Deferred tab given to what was kept", () => {
  /** What was kept before Open and Deferred were two tabs: Open in front beside Done, and no Deferred anywhere. */
  const before = async () => {
    const { build } = await import("./tree");
    return build(
      {
        dir: "row",
        ratio: 0.5,
        a: [
          "thread",
        ],
        b: [
          "done",
          "open",
        ],
      },
      "open",
    );
  };
  const keep = (key: string, value: unknown) =>
    kept.setItem(`armada:kept:${key}`, JSON.stringify(value));
  const entryFor = (arrangement: Arrangement) => ({
    arrangement,
    layout: "Default",
    edited: false,
    chosen: false,
    version: 0,
  });
  /** The kinds of the pane that holds Open, and the kind of the tab it shows. */
  const openPane = (
    s: Awaited<ReturnType<typeof start>>,
    arrangement: Arrangement,
  ): {
    kinds: string[];
    front: string | undefined;
  } => {
    const pane = s
      .panesOf(arrangement.root)
      .find((one) => one.tabs.some((tab) => tab.kind === "open"))!;
    return {
      kinds: pane.tabs.map((tab) => tab.kind),
      front: pane.tabs.find((tab) => tab.id === pane.active)?.kind,
    };
  };

  test("a Job's kept panes that hold Open and no Deferred get one after it, with Open still in front", async () => {
    keep("job-panes:job:old", entryFor(await before()));
    const s = await start();
    expect(openPane(s, s.peek("old"))).toEqual({
      kinds: [
        "done",
        "open",
        "deferred",
      ],
      front: "open",
    });
    expect(kept.getItem("armada:kept:job-panes:deferred-given")).toBe("true");
  });

  test("the kept base of Default and every saved layout get one too", async () => {
    keep("job-panes:shared", {
      saved: {
        Wide: await before(),
      },
      forAll: "Default",
      versions: {},
      base: await before(),
    });
    const s = await start();
    const shared = JSON.parse(kept.getItem("armada:kept:job-panes:shared")!) as {
      saved: Record<string, Arrangement>;
      base: Arrangement;
    };
    for (const one of [
      shared.base,
      shared.saved.Wide!,
    ])
      expect(openPane(s, one)).toEqual({
        kinds: [
          "done",
          "open",
          "deferred",
        ],
        front: "open",
      });
  });

  test("a kept arrangement with no Open, or with a Deferred already, is left as it was", async () => {
    const { build } = await import("./tree");
    const files = build([
      "thread",
      "files",
    ]);
    const both = build([
      "open",
      "deferred",
    ]);
    keep("job-panes:job:files", entryFor(files));
    keep("job-panes:job:both", entryFor(both));
    const s = await start();
    expect(kinds(s.peek("files"), (x) => s.panesOf(x.root))).toEqual([
      [
        "thread",
        "files",
      ],
    ]);
    expect(kinds(s.peek("both"), (x) => s.panesOf(x.root))).toEqual([
      [
        "open",
        "deferred",
      ],
    ]);
  });

  test("a person who closes the tab keeps it closed across the next start", async () => {
    keep("job-panes:job:old", entryFor(await before()));
    const first = await start();
    const arrangement = first.peek("old");
    const deferred = first
      .panesOf(arrangement.root)
      .flatMap((pane) => pane.tabs)
      .find((tab) => tab.kind === "deferred")!;
    first.rearrange("old", (x) => first.closeTab(x, deferred.id));
    expect(openPane(first, first.peek("old")).kinds).toEqual([
      "done",
      "open",
    ]);
    const second = await start();
    expect(openPane(second, second.peek("old")).kinds).toEqual([
      "done",
      "open",
    ]);
  });
});

describe("a saved request", () => {
  test("a page names the collection and request, and any other page names none", async () => {
    const s = await start();
    expect(s.savedOfPage(s.savedPage("store-api", "create/order"))).toEqual({
      collection: "store-api",
      request: "create/order",
    });
    expect(s.savedOfPage(undefined)).toBeUndefined();
    expect(s.savedOfPage("https://example.test")).toBeUndefined();
  });

  test("opens a Requests tab on the Job, and a second one shows the tab it has the other request", async () => {
    const s = await start();
    s.openSavedRequest("a", "store-api", "create-order");
    const tabs = () =>
      s
        .panesOf(s.peek("a").root)
        .flatMap((pane) => pane.tabs)
        .filter((tab) => tab.kind === "request");
    expect(tabs().map((tab) => s.savedOfPage(tab.page))).toEqual([
      {
        collection: "store-api",
        request: "create-order",
      },
    ]);
    s.openSavedRequest("a", "store-api", "list-orders");
    expect(tabs().map((tab) => s.savedOfPage(tab.page))).toEqual([
      {
        collection: "store-api",
        request: "list-orders",
      },
    ]);
  });
});

describe("a terminal the Job opens itself", () => {
  /** Every Terminal tab of the Job, by the terminal each holds, and the tab in front of the pane that holds them. */
  async function terminals() {
    const s = await start();
    const tabs = (id: string) =>
      s.panesOf(s.peek(id).root).flatMap((pane) =>
        pane.tabs
          .filter((tab) => tab.kind === "terminal")
          .map((tab) => ({
            terminal: tab.terminal,
            front: pane.active === tab.id,
          })),
      );
    return {
      s,
      tabs,
    };
  }

  test("gets a tab behind the one in front, the focus stays where it was, and it is never added twice", async () => {
    const { s, tabs } = await terminals();
    const before = s.peek("a");
    s.jobOpensTerminal("a", "t2", false);
    expect(tabs("a")).toEqual([
      {
        terminal: "t2",
        front: true,
      },
    ]);
    expect(s.peek("a").focus).toBe(before.focus);
    s.jobOpensTerminal("a", "t2", false);
    s.jobOpensTerminal("a", "t3", false);
    s.jobOpensTerminal("a", "t3", false);
    expect(tabs("a")).toEqual([
      {
        terminal: "t2",
        front: true,
      },
      {
        terminal: "t3",
        front: false,
      },
    ]);
    expect(s.peek("a").focus).toBe(before.focus);
  });

  test("adds nothing for a terminal the person opened with +, which has its tab already, or the Job's first that a tab with no ID draws", async () => {
    const { s, tabs } = await terminals();
    s.rearrange("a", (now) =>
      s.addTab(now, now.focus, "terminal", undefined, undefined, undefined, undefined, "t2"),
    );
    s.rearrange("a", (now) => s.addTab(now, now.focus, "terminal"));
    s.jobOpensTerminal("a", "t2", false);
    s.jobOpensTerminal("a", "t1", true);
    expect(tabs("a").map((one) => one.terminal)).toEqual([
      "t2",
      undefined,
    ]);
  });
});

describe("a terminal the Job's agent shows", () => {
  /** The Job's Terminal tabs by terminal and whether each is in front of its pane. */
  async function terminals() {
    const s = await start();
    const tabs = (id: string) =>
      s.panesOf(s.peek(id).root).flatMap((pane) =>
        pane.tabs
          .filter((tab) => tab.kind === "terminal")
          .map((tab) => ({
            terminal: tab.terminal,
            front: pane.active === tab.id,
          })),
      );
    return {
      s,
      tabs,
    };
  }

  test("comes to the front where its tab is behind, opens none beside it, and leaves the focus where it was", async () => {
    const { s, tabs } = await terminals();
    s.rearrange("a", (now) =>
      s.addTab(now, now.focus, "terminal", undefined, undefined, undefined, undefined, "t2"),
    );
    s.rearrange("a", (now) => s.addTab(now, now.focus, "files"));
    expect(tabs("a")).toEqual([
      {
        terminal: "t2",
        front: false,
      },
    ]);
    const before = s.peek("a");
    s.jobShowsTerminal("a", "t2", false);
    expect(tabs("a")).toEqual([
      {
        terminal: "t2",
        front: true,
      },
    ]);
    expect(s.peek("a").focus).toBe(before.focus);
  });

  test("opens a tab in front where the Job has none for it, and leaves the focus where it was", async () => {
    const { s, tabs } = await terminals();
    s.rearrange("a", (now) =>
      s.addTab(now, now.focus, "terminal", undefined, undefined, undefined, undefined, "t1"),
    );
    const before = s.peek("a");
    s.jobShowsTerminal("a", "t2", false);
    expect(tabs("a")).toEqual([
      {
        terminal: "t1",
        front: false,
      },
      {
        terminal: "t2",
        front: true,
      },
    ]);
    expect(s.peek("a").focus).toBe(before.focus);
  });

  test("is the Job's own arrangement it changes, with none of its panes drawn, and no other Job's", async () => {
    const { s, tabs } = await terminals();
    s.jobShowsTerminal("away", "t2", false);
    expect(tabs("away")).toEqual([
      {
        terminal: "t2",
        front: true,
      },
    ]);
    expect(tabs("a")).toEqual([]);
  });
});
