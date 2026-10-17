// How a screen is laid out and how its steps play. **Layout is the browser's**, so a tree is proved by the classes, styles and
// attributes ScreenView writes for the stylesheet to read, and by a measurement where sizing and stretching are the claim,
// which this project's real browser can take. The component is `packages/components/src/compositions/SketchScene/ScreenView.tsx`.
// A part with an id or a step is a group named for its words, else its title, else its id: that is how a test finds it.

import { afterEach, describe, expect, test } from "vitest";

import { ScreenView } from "@armada/components/src/compositions/SketchScene/ScreenView";
import type { ScreenElement } from "@armada/components/src/compositions/SketchScene/screen";

import { mount, unmount } from "./mounted";

afterEach(unmount);

/** A screen on a page 600 wide. */
async function draw(screen: ScreenElement, at?: number) {
  mount(
    <div style={{ inlineSize: 600 }}>
      <ScreenView screen={screen} at={at} />
    </div>,
  );
  // React 19 renders a root on its own time, and the first test of a cold browser can take more than a frame.
  await expect.poll(() => document.querySelector(".armada-wireframe")).not.toBeNull();
  const part = (name: string): HTMLElement => {
    const found = document.querySelector<HTMLElement>(`[aria-label="${name}"]`);
    if (found === null) throw new Error(`no part named ${name}`);
    return found;
  };
  return { part, box: (name: string) => part(name).getBoundingClientRect(), style: (name: string) => getComputedStyle(part(name)) };
}

const near = (got: number, want: number) => expect(Math.abs(got - want)).toBeLessThan(1);

describe("a part that fills", () => {
  const row = (children: ScreenElement[]): ScreenElement => ({ type: "row", gap: "none", children });

  test("takes what the others leave, in a row", async () => {
    const { box, style } = await draw(row([{ type: "button", id: "Go", text: "Go", step: 1 }, { type: "spacer", id: "rest", size: "fill" }]));
    expect(style("rest").flexGrow).toBe("1");
    expect(style("rest").flexShrink).toBe("1");
    expect(style("Go").flexGrow).toBe("0");
    near(box("Go").width + box("rest").width, 600);
    expect(box("rest").width).toBeGreaterThan(400);
  });

  test("shares it by weight, where a number is the weight of its share", async () => {
    const { box, style } = await draw(row([{ type: "spacer", id: "one", size: 1 }, { type: "spacer", id: "three", size: 3 }]));
    expect(style("one").flexGrow).toBe("1");
    expect(style("three").flexGrow).toBe("3");
    near(box("one").width, 150);
    near(box("three").width, 450);
  });

  test("takes the height left in a column, as it takes the width left in a row", async () => {
    const tall: ScreenElement = {
      type: "column",
      gap: "none",
      id: "tall",
      children: [{ type: "text", text: "Top" }, { type: "spacer", id: "gap", size: "fill" }, { type: "text", text: "Bottom" }],
    };
    const { box, style } = await draw(row([tall, { type: "panel", id: "Side", title: "Side", children: [{ type: "text", lines: 6 }] }]));
    expect(style("gap").flexGrow).toBe("1");
    expect(box("gap").height).toBeGreaterThan(0);
    near(box("tall").height, box("Side").height);
  });

  test("is no flex share in a grid, whatever size it names", async () => {
    const { part } = await draw({
      type: "grid",
      cols: 2,
      children: [{ type: "spacer", id: "cell", size: "fill" }, { type: "spacer", id: "other", size: 4 }],
    });
    expect(part("cell").style.flex).toBe("");
    expect(part("other").style.flex).toBe("");
  });

  test("is no flex share when it is auto or says nothing", async () => {
    const { part } = await draw(row([{ type: "spacer", id: "named", size: "auto" }, { type: "spacer", id: "bare" }]));
    expect(part("named").style.flex).toBe("");
    expect(part("bare").style.flex).toBe("");
  });
});

describe("a part across its parent", () => {
  const column = (children: ScreenElement[]): ScreenElement => ({ type: "column", children });

  test("stretches by default where it has room in it, and a control keeps its own size", async () => {
    const { part, box } = await draw(column([{ type: "panel", id: "Panel", title: "Panel" }, { type: "button", id: "Save", text: "Save" }, { type: "badge", id: "New", text: "New" }]));
    expect(part("Panel").hasAttribute("data-align")).toBe(false);
    expect(part("Panel").dataset.axis).toBe("column");
    near(box("Panel").width, 600);
    expect(getComputedStyle(part("Save")).alignSelf).toBe("flex-start");
    expect(box("Save").width).toBeLessThan(200);
    expect(box("New").width).toBeLessThan(200);
  });

  test("stretches in a row to the row's height, and a control sits in the middle of it", async () => {
    const { part, box } = await draw({
      type: "row",
      children: [
        { type: "panel", id: "Tall", title: "Tall", children: [{ type: "text", lines: 6 }] },
        { type: "panel", id: "Short", title: "Short" },
        { type: "button", id: "Go", text: "Go" },
      ],
    });
    expect(part("Short").dataset.axis).toBe("row");
    near(box("Short").height, box("Tall").height);
    expect(getComputedStyle(part("Go")).alignSelf).toBe("center");
    expect(box("Go").height).toBeLessThan(box("Tall").height);
  });

  test("goes where it says, and what it says wins over the default for its kind", async () => {
    const { part, box } = await draw(
      column([
        { type: "button", id: "Start", text: "Start", align: "start" },
        { type: "button", id: "Middle", text: "Middle", align: "center" },
        { type: "button", id: "End", text: "End", align: "end" },
        { type: "button", id: "Stretch", text: "Stretch", align: "stretch" },
      ]),
    );
    for (const [name, value] of [["Start", "start"], ["Middle", "center"], ["End", "end"], ["Stretch", "stretch"]] as const) {
      expect(part(name).dataset.align).toBe(value);
    }
    const host = part("Start").parentElement!.getBoundingClientRect();
    near(box("Start").left, host.left);
    near(box("Middle").left + box("Middle").width / 2, host.left + host.width / 2);
    near(box("End").right, host.right);
    near(box("Stretch").width, host.width);
  });

  test("wins in a row too, over the centring a control gets there", async () => {
    const { part, box } = await draw({
      type: "row",
      children: [
        { type: "panel", id: "Tall", title: "Tall", children: [{ type: "text", lines: 6 }] },
        { type: "button", id: "Go", text: "Go", align: "stretch" },
        { type: "button", id: "Bottom", text: "Bottom", align: "end" },
      ],
    });
    expect(getComputedStyle(part("Go")).alignSelf).toBe("stretch");
    near(box("Go").height, box("Tall").height);
    near(box("Bottom").bottom, box("Tall").bottom);
  });
});

describe("a modal with a step", () => {
  const screen: ScreenElement = {
    type: "window",
    chrome: "desktop",
    children: [
      { type: "button", id: "Invite", text: "Invite", step: 1 },
      { type: "text", text: "Members" },
      { type: "modal", id: "Invite someone", title: "Invite someone", step: 2, children: [{ type: "field", label: "Email" }] },
    ],
  };

  test("is hidden until its step plays, and keeps its place", async () => {
    const { part } = await draw(screen, 1);
    expect(part("Invite someone").dataset.hidden).toBe("true");
    expect(getComputedStyle(part("Invite someone")).visibility).toBe("hidden");
  });

  test("comes up with its step, its dialog ringed rather than the scrim round it", async () => {
    const { part } = await draw(screen, 2);
    const modal = part("Invite someone");
    expect(modal.hasAttribute("data-hidden")).toBe(false);
    expect(getComputedStyle(modal).visibility).toBe("visible");
    expect(modal.hasAttribute("data-current")).toBe(false);
    expect(modal.querySelector(".armada-wireframe__dialog")!.hasAttribute("data-current")).toBe(true);
  });

  test("is there when no step is playing, and stays where the steps have passed it", async () => {
    expect((await draw(screen)).part("Invite someone").hasAttribute("data-hidden")).toBe(false);
    expect((await draw(screen, 3)).part("Invite someone").hasAttribute("data-hidden")).toBe(false);
  });

  test("is always there with no step of its own", async () => {
    const { part } = await draw({ type: "window", chrome: "desktop", children: [{ type: "button", id: "Open", text: "Open", step: 1 }, { type: "modal", id: "Always", title: "Always", children: [] }] }, 1);
    expect(part("Always").hasAttribute("data-hidden")).toBe(false);
  });
});

describe("a step inside a screen", () => {
  const screen: ScreenElement = {
    type: "column",
    children: [
      { type: "button", id: "First", text: "First", step: 1 },
      { type: "text", id: "plain", text: "Plain" },
      {
        type: "panel",
        title: "Panel",
        step: 4,
        children: [
          { type: "button", id: "Second", text: "Second", step: 2 },
          { type: "text", id: "beside" },
        ],
      },
      { type: "panel", id: "Held", title: "Held", step: 3, children: [{ type: "text", id: "within" }, { type: "button", id: "Also", text: "Also", step: 5 }] },
    ],
  };
  const ALL = ["First", "Plain", "Panel", "Second", "beside", "Held", "within", "Also"];
  const flags = (part: (name: string) => HTMLElement) =>
    Object.fromEntries(ALL.map((name) => [name, part(name).hasAttribute("data-current") ? "current" : part(name).hasAttribute("data-dim") ? "dim" : "plain"]));

  test("rings the element and stands the rest of the screen back", async () => {
    const { part } = await draw(screen, 1);
    expect(flags(part)).toMatchObject({ First: "current", Plain: "dim", Panel: "dim", Held: "dim" });
  });

  test("decides nothing under a part that stands back: it is one faded thing", async () => {
    const { part } = await draw(screen, 1);
    expect(flags(part)).toMatchObject({ Second: "plain", beside: "plain", within: "plain", Also: "plain" });
  });

  test("keeps the parent of the element in step and stands back its siblings", async () => {
    const { part } = await draw(screen, 2);
    expect(flags(part)).toEqual({ First: "dim", Plain: "dim", Panel: "plain", Second: "current", beside: "dim", Held: "dim", within: "plain", Also: "plain" });
  });

  test("leaves what is under the ringed element as it is", async () => {
    const { part } = await draw(screen, 4);
    expect(flags(part)).toMatchObject({ Panel: "current", Second: "plain", beside: "plain", First: "dim", Held: "dim" });
  });

  test("rings an element under a part that holds its step, and stands that part's other children back", async () => {
    const { part } = await draw(screen, 5);
    expect(flags(part)).toMatchObject({ Held: "plain", within: "dim", Also: "current", First: "dim", Panel: "dim" });
  });

  test("names the elements that carry a step or an id, so a reader and a walk can find them", async () => {
    const { part } = await draw(screen, 1);
    for (const name of ["First", "Held", "beside"]) expect(part(name).getAttribute("role")).toBe("group");
  });

  test("marks nothing when no step is playing", async () => {
    const { part } = await draw(screen);
    expect(flags(part)).toEqual(Object.fromEntries(ALL.map((name) => [name, "plain"])));
  });

  test("stands the whole screen back as one faded thing for a step it does not hold", async () => {
    const { part } = await draw({ type: "column", id: "screen", children: [{ type: "button", id: "Only", text: "Only", step: 1 }] }, 9);
    expect(part("screen").hasAttribute("data-dim")).toBe(true);
    expect(part("Only").hasAttribute("data-dim") || part("Only").hasAttribute("data-current")).toBe(false);
  });
});
