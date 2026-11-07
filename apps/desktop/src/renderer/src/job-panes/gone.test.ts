// When a gone Job's panes are dropped: the rule over Fleet's list, and the sweep that reads it on a fresh read at most every 10 minutes.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// As in store.test.ts: only the kept values are loaded afresh, over a Storage this test holds.

/** A Storage whose keys list as its own properties, as the browser's does. */
function storage(): Storage {
  const items = new Map<string, string>();
  const target: Record<string, unknown> = {};
  const methods = {
    get length() {
      return items.size;
    },
    key: (at: number) => [...items.keys()][at] ?? null,
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

const KEEP = decodeURIComponent(new URL("../../../../../../packages/components/src/keep.ts", import.meta.url).pathname);

let kept: Storage;

beforeEach(() => {
  kept = storage();
  vi.stubGlobal("window", {
    localStorage: kept,
    addEventListener: () => undefined,
  });
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});

/** The sweep, the store and the tree as a freshly started window loads them. */
async function start() {
  vi.resetModules();
  vi.doMock("@armada/components", () => import(/* @vite-ignore */ KEEP));
  const [gone, store, tree, custom] = await Promise.all([
    import("./gone"),
    import("./store"),
    import("./tree"),
    import("./custom"),
  ]);
  gone.resetSweep();
  return {
    ...gone,
    ...store,
    ...tree,
    ...custom,
  };
}

const NOW = Date.parse("2026-11-06T12:00:00Z");
const DAY = 24 * 60 * 60 * 1000;

type Row = {
  id: string;
  state: "live" | "ended";
  ended_at?: string;
};
const live = (id: string): Row => ({
  id,
  state: "live",
});
const ended = (id: string, daysAgo?: number): Row => ({
  id,
  state: "ended",
  ...(daysAgo === undefined
    ? {}
    : {
        ended_at: new Date(NOW - daysAgo * DAY).toISOString(),
      }),
});
const read = (sessions: Row[]) =>
  ({
    state: "read",
    sessions,
  }) as unknown as Parameters<Awaited<ReturnType<typeof start>>["sweepOnRead"]>[0];

const keyOf = (id: string) => `armada:kept:job-panes:job:${id}`;

describe("a gone Job", () => {
  test("is one Fleet's list no longer has, or one ended more than 7 days ago", async () => {
    const s = await start();
    const records = [
      live("live"),
      ended("recent", 6),
      ended("old", 8),
      ended("undated"),
    ];
    expect(s.isGone("live", records, NOW)).toBe(false);
    expect(s.isGone("recent", records, NOW)).toBe(false);
    expect(s.isGone("old", records, NOW)).toBe(true);
    // With no instant to count from, its age cannot be told.
    expect(s.isGone("undated", records, NOW)).toBe(false);
    expect(s.isGone("absent", records, NOW)).toBe(true);
  });

  test("a live Job is never gone, however long ago it started", async () => {
    const s = await start();
    expect(
      s.isGone(
        "a",
        [
          {
            ...live("a"),
            ended_at: new Date(NOW - 30 * DAY).toISOString(),
          },
        ],
        NOW,
      ),
    ).toBe(false);
  });
});

describe("the sweep", () => {
  test("drops the panes of gone Jobs, kept or in memory, and keeps the rest", async () => {
    const s = await start();
    for (const id of [
      "live",
      "recent",
      "old",
      "absent",
    ])
      s.rearrange(id, (x) => s.addTab(x, x.focus, "files"));
    const dropped = s.sweepOnRead(
      read([
        live("live"),
        ended("recent", 1),
        ended("old", 8),
      ]),
      NOW,
      NOW,
    );
    expect(dropped.sort()).toEqual([
      "absent",
      "old",
    ]);
    expect(kept.getItem(keyOf("live"))).not.toBeNull();
    expect(kept.getItem(keyOf("recent"))).not.toBeNull();
    expect(kept.getItem(keyOf("old"))).toBeNull();
    expect(kept.getItem(keyOf("absent"))).toBeNull();
    expect(s.jobPanes.ownerIds().sort()).toEqual([
      "live",
      "recent",
    ]);
  });

  test("reads keys kept by an earlier window, before any of them is opened", async () => {
    const first = await start();
    first.rearrange("old", (x) => first.addTab(x, x.focus, "files"));
    const s = await start();
    expect(s.sweepOnRead(read([]), NOW, NOW)).toEqual([
      "old",
    ]);
    expect(kept.getItem(keyOf("old"))).toBeNull();
  });

  test("never drops a Job drawn in the window, and picks it up once it is closed", async () => {
    const s = await start();
    s.rearrange("open", (x) => s.addTab(x, x.focus, "files"));
    const drawn = vi.spyOn(s.jobPanes, "isDrawn").mockImplementation((id) => id === "open");
    expect(s.sweepGoneJobs([], NOW)).toEqual([]);
    expect(kept.getItem(keyOf("open"))).not.toBeNull();
    drawn.mockReturnValue(false);
    expect(s.sweepGoneJobs([], NOW)).toEqual([
      "open",
    ]);
  });

  test("never runs on the picture from disk, before Fleet answers, or on a failed read", async () => {
    const s = await start();
    s.rearrange("old", (x) => s.addTab(x, x.focus, "files"));
    // The picture Bridge launches on arrives as read, with no readAt.
    expect(s.sweepOnRead(read([]), null, NOW)).toEqual([]);
    expect(
      s.sweepOnRead(
        {
          state: "none",
        },
        NOW,
        NOW,
      ),
    ).toEqual([]);
    expect(
      s.sweepOnRead(
        {
          state: "failed",
        } as unknown as Parameters<typeof s.sweepOnRead>[0],
        NOW,
        NOW,
      ),
    ).toEqual([]);
    expect(kept.getItem(keyOf("old"))).not.toBeNull();
  });

  test("runs at most once every 10 minutes", async () => {
    const s = await start();
    s.sweepOnRead(read([]), NOW, NOW);
    s.rearrange("later", (x) => s.addTab(x, x.focus, "files"));
    expect(s.sweepOnRead(read([]), NOW, NOW + s.SWEEP_EVERY_MS - 1)).toEqual([]);
    expect(kept.getItem(keyOf("later"))).not.toBeNull();
    expect(s.sweepOnRead(read([]), NOW, NOW + s.SWEEP_EVERY_MS)).toEqual([
      "later",
    ]);
  });

  test("a disk picture does not spend the throttle: the first fresh read still sweeps", async () => {
    const s = await start();
    s.rearrange("old", (x) => s.addTab(x, x.focus, "files"));
    s.sweepOnRead(read([]), null, NOW);
    expect(s.sweepOnRead(read([]), NOW, NOW + 1)).toEqual([
      "old",
    ]);
  });

  test("drops a gone Job's Custom tab state and drafts, and leaves a live Job's", async () => {
    const s = await start();
    for (const id of ["live", "old"]) {
      s.rearrange(id, (x) => s.addTab(x, x.focus, "files"));
      s.seedBuilt(id, "t1", "review-queue");
      kept.setItem(`armada:kept:session:custom:${id}:t1`, "draft");
    }
    s.sweepOnRead(read([live("live")]), NOW, NOW);
    expect(s.peekCustom("old", "t1").stage).toBe("compose");
    expect(kept.getItem("armada:kept:session:custom:old:t1")).toBeNull();
    expect(s.peekCustom("live", "t1").stage).toBe("built");
    expect(kept.getItem("armada:kept:session:custom:live:t1")).not.toBeNull();
  });

  test("forgets drafts kept in the old shape on the first fresh sweep, not on the disk picture", async () => {
    const s = await start();
    kept.setItem("armada:kept:session:custom:t7", "draft");
    s.sweepOnRead(read([]), null, NOW);
    expect(kept.getItem("armada:kept:session:custom:t7")).not.toBeNull();
    s.sweepOnRead(read([]), NOW, NOW + 1);
    expect(kept.getItem("armada:kept:session:custom:t7")).toBeNull();
  });
});
