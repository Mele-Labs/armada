// The pane tree's operations: every one is pure, so each test hands an arrangement in and reads the next one out.

import { describe, expect, test } from "vitest";

import {
  MIN_PANE_H,
  MIN_PANE_W,
  addTab,
  arrive,
  build,
  closePane,
  closeTab,
  holderOf,
  markTabMod,
  moveTabToEdge,
  moveTabToEnd,
  moveTabToGap,
  moveTabToPane,
  panesOf,
  rectsOf,
  resizeSplit,
  shortfall,
  splitPane,
  type Arrangement,
  type Node,
} from "./tree";

const kindsOf = (arrangement: Arrangement): string[][] =>
  panesOf(arrangement.root).map((pane) => pane.tabs.map((tab) => tab.kind));
const tabOfKind = (arrangement: Arrangement, kind: string) =>
  panesOf(arrangement.root)
    .flatMap((pane) => pane.tabs)
    .find((tab) => tab.kind === kind)!;
const row = () =>
  build({
    dir: "row",
    ratio: 0.5,
    a: [
      "thread",
    ],
    b: [
      "terminal",
      "browser",
    ],
  });

describe("split and close", () => {
  test("a pane splits to its right with an empty pane that takes the focus", () => {
    const one = build([
      "thread",
    ]);
    const next = splitPane(one, one.focus, "right");
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [],
    ]);
    expect(next.root).toMatchObject({
      type: "split",
      dir: "row",
      ratio: 0.5,
    });
    expect(next.focus).toBe(panesOf(next.root)[1]!.id);
  });

  test("a split below stacks, and an unknown pane changes nothing", () => {
    const one = build([
      "thread",
    ]);
    expect(splitPane(one, one.focus, "down").root).toMatchObject({
      dir: "column",
    });
    expect(splitPane(one, "nope", "down")).toBe(one);
  });

  test("closing a pane's last tab removes the pane and the sibling takes the split's place", () => {
    const arrangement = build({
      dir: "row",
      ratio: 0.5,
      a: [
        "thread",
      ],
      b: [
        "terminal",
      ],
    });
    const next = closeTab(arrangement, tabOfKind(arrangement, "terminal").id);
    expect(next.root.type).toBe("pane");
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
    ]);
  });

  test("the last tab of the last pane leaves an empty pane, and the last pane is never closed", () => {
    const arrangement = build([
      "thread",
    ]);
    const next = closeTab(arrangement, tabOfKind(arrangement, "thread").id);
    expect(kindsOf(next)).toEqual([
      [],
    ]);
    expect(closePane(arrangement, arrangement.focus)).toBe(arrangement);
  });

  test("closing the front tab brings the next one forward, and the last brings the one before", () => {
    const arrangement = build(
      [
        "thread",
        "terminal",
        "browser",
      ],
      "terminal",
    );
    const closed = closeTab(arrangement, tabOfKind(arrangement, "terminal").id);
    expect(panesOf(closed.root)[0]!.active).toBe(tabOfKind(arrangement, "browser").id);
    const two = build(
      [
        "thread",
        "terminal",
      ],
      "terminal",
    );
    const last = closeTab(two, tabOfKind(two, "terminal").id);
    expect(panesOf(last.root)[0]!.active).toBe(tabOfKind(two, "thread").id);
  });

  test("closing the focused pane puts the focus on the first pane left", () => {
    const arrangement = row();
    const second = panesOf(arrangement.root)[1]!;
    const focused = {
      ...arrangement,
      focus: second.id,
    };
    expect(closePane(focused, second.id).focus).toBe(panesOf(arrangement.root)[0]!.id);
  });
});

describe("moving a tab", () => {
  test("into another pane at an index", () => {
    const arrangement = row();
    const thread = tabOfKind(arrangement, "thread");
    const target = panesOf(arrangement.root)[1]!;
    const next = moveTabToPane(arrangement, thread.id, target.id, 1);
    expect(kindsOf(next)).toEqual([
      [
        "terminal",
        "thread",
        "browser",
      ],
    ]);
    expect(next.root.type).toBe("pane");
  });

  test("within its own pane it only changes its place", () => {
    const arrangement = row();
    const pane = panesOf(arrangement.root)[1]!;
    const next = moveTabToPane(arrangement, tabOfKind(arrangement, "terminal").id, pane.id);
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [
        "browser",
        "terminal",
      ],
    ]);
  });

  test("onto an edge of a pane it makes a pane there, and a lone tab dropped on its own edge stays", () => {
    const arrangement = row();
    const next = moveTabToEdge(
      arrangement,
      tabOfKind(arrangement, "browser").id,
      panesOf(arrangement.root)[0]!.id,
      "left",
    );
    expect(kindsOf(next)).toEqual([
      [
        "browser",
      ],
      [
        "thread",
      ],
      [
        "terminal",
      ],
    ]);
    const thread = tabOfKind(arrangement, "thread");
    expect(moveTabToEdge(arrangement, thread.id, panesOf(arrangement.root)[0]!.id, "top")).toBe(
      arrangement,
    );
  });

  test("into the gap of a split it makes a pane between the two sides and gives each a third", () => {
    const arrangement = row();
    const split = rectsOf(arrangement.root).splits[0]!;
    const next = moveTabToGap(arrangement, tabOfKind(arrangement, "browser").id, split.id);
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [
        "browser",
      ],
      [
        "terminal",
      ],
    ]);
    const widths = [
      ...rectsOf(next.root).panes.values(),
    ].map((rect) => Math.round(rect.w * 100) / 100);
    expect(widths).toEqual([
      0.33,
      0.33,
      0.33,
    ]);
  });

  test("a gap beside a pane's only tab changes nothing", () => {
    const arrangement = row();
    const split = rectsOf(arrangement.root).splits[0]!;
    expect(moveTabToGap(arrangement, tabOfKind(arrangement, "thread").id, split.id)).toBe(
      arrangement,
    );
  });

  test("a gap in a column stacks the new pane", () => {
    const arrangement = build({
      dir: "column",
      ratio: 0.5,
      a: [
        "thread",
      ],
      b: [
        "terminal",
        "browser",
      ],
    });
    const next = moveTabToGap(
      arrangement,
      tabOfKind(arrangement, "browser").id,
      rectsOf(arrangement.root).splits[0]!.id,
    );
    expect(rectsOf(next.root).splits.every((split) => split.dir === "column")).toBe(true);
  });

  test("to an outer edge it makes a root split with a third, and a lone tab carried to the far edge swaps the panes", () => {
    const arrangement = row();
    const next = moveTabToEnd(arrangement, tabOfKind(arrangement, "browser").id, "right");
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [
        "terminal",
      ],
      [
        "browser",
      ],
    ]);
    expect(rectsOf(next.root).panes.get(panesOf(next.root)[2]!.id)!.w).toBeCloseTo(1 / 3);
    const swapped = moveTabToEnd(arrangement, tabOfKind(arrangement, "thread").id, "right");
    expect(kindsOf(swapped)).toEqual([
      [
        "terminal",
        "browser",
      ],
      [
        "thread",
      ],
    ]);
  });

  test("a drop that leaves the panes as they were is nothing, and the only tab has nowhere to go", () => {
    const arrangement = row();
    expect(moveTabToEnd(arrangement, tabOfKind(arrangement, "thread").id, "left")).toBe(
      arrangement,
    );
    const alone = build([
      "thread",
    ]);
    expect(moveTabToEnd(alone, tabOfKind(alone, "thread").id, "right")).toBe(alone);
  });
});

describe("minimum sizes", () => {
  test("two panes in an area as wide as both minimums have no shortfall", () => {
    expect(
      shortfall(row(), {
        w: MIN_PANE_W * 2 + 1,
        h: MIN_PANE_H,
      }),
    ).toBe(0);
  });

  test("the shortfall is how far the panes fall under their minimums", () => {
    const wide = shortfall(row(), {
      w: MIN_PANE_W * 2 + 1,
      h: MIN_PANE_H,
    });
    const narrow = shortfall(row(), {
      w: MIN_PANE_W,
      h: MIN_PANE_H,
    });
    expect(narrow).toBeGreaterThan(wide);
  });

  test("a ratio is held to the least a side keeps", () => {
    const arrangement = row();
    const split = resizeSplit(arrangement, rectsOf(arrangement.root).splits[0]!.id, 0.01)
      .root as Extract<
      Node,
      {
        type: "split";
      }
    >;
    expect(split.ratio).toBeGreaterThan(0.01);
    expect(split.ratio).toBeLessThan(0.2);
  });
});

describe("arriving", () => {
  test("a tab a Job opens joins the pane that holds that kind", () => {
    const next = arrive(row(), "browser", "localhost:5173");
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [
        "terminal",
        "browser",
        "browser",
      ],
    ]);
    expect(holderOf(next, "browser")!.tabs.at(-1)).toMatchObject({
      kind: "browser",
      label: "localhost:5173",
    });
  });

  test("with no pane holding the kind it opens as a split on the right of everything", () => {
    const arrangement = build([
      "thread",
    ]);
    const next = arrive(arrangement, "terminal", "zsh");
    expect(kindsOf(next)).toEqual([
      [
        "thread",
      ],
      [
        "terminal",
      ],
    ]);
    expect(next.root).toMatchObject({
      type: "split",
      dir: "row",
    });
    expect(next.focus).toBe(arrangement.focus);
  });

  test("the same label a second time is numbered", () => {
    const once = arrive(
      build([
        "thread",
      ]),
      "terminal",
      "zsh",
    );
    const twice = addTab(once, holderOf(once, "terminal")!.id, "terminal", "zsh");
    expect(holderOf(twice, "terminal")!.tabs.map((tab) => tab.label)).toEqual([
      "zsh",
      "zsh 2",
    ]);
  });
});

describe("a Custom tab's mod", () => {
  test("is kept on the tab, and marking it again changes nothing", () => {
    const arrangement = build({
      dir: "row",
      ratio: 0.5,
      a: [
        "thread",
      ],
      b: [
        "custom",
      ],
    });
    const tab = tabOfKind(arrangement, "custom");
    const marked = markTabMod(arrangement, tab.id, "review-queue");
    expect(tabOfKind(marked, "custom").mod).toBe("review-queue");
    expect(markTabMod(marked, tab.id, "review-queue")).toBe(marked);
    expect(markTabMod(arrangement, "nope", "review-queue")).toBe(arrangement);
  });
});
