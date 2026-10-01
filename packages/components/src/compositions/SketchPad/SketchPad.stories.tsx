import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, waitFor } from "storybook/test";

import {
  SketchPad,
  type SketchBox,
  type SketchLine,
  type SketchPicture,
  type SketchPoint,
  type SketchStroke,
} from "./SketchPad";

const meta: Meta<typeof SketchPad> = {
  title: "Compositions/Sketch pad",
  component: SketchPad,
  parameters: { layout: "padded" },
};
export default meta;

type Story = StoryObj<typeof SketchPad>;

/** What the arc's person drew: the stat, the panel it opens, and the read behind it. */
const BOXES: SketchBox[] = [
  { id: "b1", x: 0, y: 0, body: "Drones 1 of 2" },
  { id: "b2", x: 0, y: 160, body: "a panel under it, one Drone to a line" },
  { id: "b3", x: 300, y: 160, body: "the Job and the step each Drone is on" },
];

const LINES: SketchLine[] = [
  { id: "b1-b2", from: "b1", to: "b2" },
  { id: "b2-b3", from: "b2", to: "b3" },
];

/** A ring round the two boxes the panel is made of — what no box and no join says. */
const STROKES: SketchStroke[] = [
  {
    id: "s1",
    points: [
      { x: -40, y: 126 },
      { x: 280, y: 108 },
      { x: 570, y: 132 },
      { x: 588, y: 214 },
      { x: 280, y: 248 },
      { x: -30, y: 230 },
      { x: -46, y: 160 },
      { x: -40, y: 126 },
    ],
  },
];

const SAID = "The panel opens under the stat, with the Drone's Job on the first line.";

/**
 * The pad, with a caller holding what is on it.
 *
 * **Every edit is the caller's**, the way `DispatchJob` holds the drawing: the
 * pad reports and draws, and a story that let it keep its own boxes would be
 * proving something the app does not do.
 */
function Held({
  boxes: open,
  lines: drawn,
  strokes: inked,
  pictures: pasted,
  ...rest
}: Parameters<typeof SketchPad>[0]) {
  const [boxes, setBoxes] = useState<readonly SketchBox[]>(open);
  const [lines, setLines] = useState<readonly SketchLine[]>(drawn);
  const [strokes, setStrokes] = useState<readonly SketchStroke[]>(inked);
  const [pictures, setPictures] = useState<readonly SketchPicture[]>(pasted);
  const [said, setSaid] = useState(rest.said);

  return (
    <SketchPad
      {...rest}
      boxes={boxes}
      lines={lines}
      strokes={strokes}
      pictures={pictures}
      onDraw={(points: readonly SketchPoint[]) =>
        setStrokes((current) => [...current, { id: `s${String(current.length + 1)}`, points }])
      }
      onUndo={() => setStrokes((current) => current.slice(0, -1))}
      said={said}
      onSaid={setSaid}
      onAdd={(at, body) =>
        setBoxes((current) => [
          ...current,
          { id: `b${String(current.length + 1)}`, x: at.x, y: at.y, body },
        ])
      }
      onPicture={(picture, at) =>
        setPictures((current) => [
          ...current,
          { id: `p${String(current.length + 1)}`, ...at, ...picture },
        ])
      }
      onBody={(id, body) =>
        setBoxes((current) => current.map((box) => (box.id === id ? { ...box, body } : box)))
      }
      onMove={(id, at) => {
        setBoxes((current) => current.map((box) => (box.id === id ? { ...box, ...at } : box)));
        setPictures((current) => current.map((one) => (one.id === id ? { ...one, ...at } : one)));
      }}
      onRemove={(ids) => {
        setBoxes((current) => current.filter((box) => !ids.includes(box.id)));
        setPictures((current) => current.filter((one) => !ids.includes(one.id)));
        setLines((current) =>
          current.filter((line) => !ids.includes(line.from) && !ids.includes(line.to)),
        );
      }}
      onJoin={(from, to) =>
        setLines((current) => [...current, { id: `${from}-${to}`, from, to }])
      }
    />
  );
}

const args = {
  label: "What the Drones stat should open into",
  boxes: BOXES,
  lines: LINES,
  strokes: STROKES,
  pictures: [],
  said: SAID,
  from: "rail-stats",
  onMove: () => undefined,
  onBody: () => undefined,
  onAdd: () => undefined,
  onPicture: () => undefined,
  onRemove: () => undefined,
  onJoin: () => undefined,
  onDraw: () => undefined,
  onUndo: () => undefined,
  onSaid: () => undefined,
};

/**
 * A picture already drawn, made from a Studio node: boxes, the joins between
 * them, and a ring round the two the panel is made of.
 */
export const Drawn: Story = { args, render: (props) => <Held {...props} /> };

/** A Studio's Sketch in its sheet: the pad fills its box, with no line about a request. */
export const Filling: Story = {
  args: { ...args, said: undefined, onSaid: undefined, from: undefined, fills: true },
  render: (props) => (
    <div style={{ height: "calc(var(--h-sketch-pad) * 2)" }}>
      <Held {...props} />
    </div>
  ),
};

/** The boxes on their own, which is every pad drawn before the pen existed. */
export const NoHand: Story = {
  args: { ...args, strokes: [] },
  render: (props) => <Held {...props} />,
};

/**
 * A pad nobody has drawn on. **The note is the whole state** — a blank canvas
 * under the controls says nothing about what it is for.
 */
export const Nothing: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
};

/** Nothing may be drawn while the connection is not live. */
export const NotLive: Story = {
  args: { ...args, disabled: true },
  render: (props) => <Held {...props} />,
};

/**
 * The acts on a box, which hover over the box they act on — the owner's note of
 * 28 Sep 2026. **They are nowhere at all while nothing is picked**, which is
 * the half a still cannot show and the half that replaced a permanent toolbar.
 *
 * **Join is the one worth asserting.** It takes exactly two boxes, so the
 * control is off with one picked and the reason is on it — a dead control with
 * no reason reads as broken.
 */
export const Drawing: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "" },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, userEvent, step }) => {
    const add = canvas.getByRole("button", { name: "Add a box" });
    const join = () => canvas.getByRole("button", { name: "Join" });
    const remove = () => canvas.getByRole("button", { name: "Remove" });

    await step("nothing is picked, so the bar is not drawn at all", async () => {
      await expect(canvas.queryByRole("button", { name: "Join" })).toBeNull();
      await expect(canvas.queryByRole("button", { name: "Remove" })).toBeNull();
    });

    await step("a box added is empty, and writing in it names it", async () => {
      await userEvent.click(add);
      const box = await canvas.findByRole("group", { name: "An empty box" });
      await expect(box).toBeVisible();
      await userEvent.type(canvas.getByRole("textbox", { name: "The words in this box" }), "the stat");
      await waitFor(() =>
        expect(canvas.getByRole("group", { name: "Box: the stat" })).toBeVisible(),
      );
    });

    await step("one box picked offers Remove, and Join says what it wants", async () => {
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await waitFor(() => expect(remove()).toBeEnabled());
      await expect(join()).toBeDisabled();
      await expect(join()).toHaveAccessibleDescription("Pick two boxes to join them.");
    });

    await step("two boxes picked draw a line between them", async () => {
      await userEvent.click(add);
      const second = await canvas.findByRole("group", { name: "An empty box" });
      await userEvent.click(second);
      await userEvent.keyboard("{Meta>}");
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await userEvent.keyboard("{/Meta}");
      await waitFor(() => expect(join()).toBeEnabled());
      await userEvent.click(join());
      await waitFor(() => expect(canvas.getAllByRole("group", { name: /^A line from / })).toHaveLength(1));
    });

    await step("a box taken off takes the line that hung on it", async () => {
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await waitFor(() => expect(remove()).toBeEnabled());
      await userEvent.click(remove());
      await waitFor(() =>
        expect(canvas.queryByRole("group", { name: "Box: the stat" })).toBeNull(),
      );
      await expect(canvas.queryAllByRole("group", { name: /^A line from / })).toHaveLength(0);
    });
  },
};

/** What a hand's line is called, and what a still cannot show it doing. */
const A_HAND_LINE = "Drawn by hand";

/**
 * One drag across the pad, in pointer events.
 *
 * **Dispatched by hand, and the layer found by what is under the point.** The
 * layer that catches the pen carries no role — a surface to draw on is not a
 * control — so there is nothing to locate it by, and `elementFromPoint` finds
 * what a real pointer would have hit. The move and the lift go to the window,
 * where the pad listens, so a line drawn off the pad still ends.
 */
function drag(graph: HTMLElement, through: readonly SketchPoint[]): void {
  const at = graph.getBoundingClientRect();
  const place = (point: SketchPoint) => ({ x: at.x + point.x, y: at.y + point.y });
  const start = place(through[0]!);
  const pen = document.elementFromPoint(start.x, start.y);
  if (pen === null) throw new Error("Nothing is under the pen.");
  const event = (kind: string, where: { x: number; y: number }) =>
    new PointerEvent(kind, { clientX: where.x, clientY: where.y, bubbles: true, button: 0 });

  pen.dispatchEvent(event("pointerdown", start));
  for (const point of through.slice(1)) window.dispatchEvent(event("pointermove", place(point)));
  window.dispatchEvent(event("pointerup", place(through[through.length - 1]!)));
}

/**
 * The pen: a line drawn by hand, and the undo that takes it back.
 *
 * **The reason this is a `play` and not a still.** A stroke exists only after a
 * drag, so no rendering can show that dragging makes one; Undo's scope — the
 * last line and nothing else — is a rule about what does *not* happen; and a
 * press with no travel making nothing is the defect a thinned freehand line
 * invites, an invisible stroke sitting in the picture.
 */
export const ByHand: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, userEvent, step }) => {
    const draw = canvas.getByRole("button", { name: "Draw" });
    const undo = canvas.getByRole("button", { name: "Undo" });
    const graph = canvas.getByLabelText(args.label);

    await step("nothing is drawn by hand, so Undo is off and says so", async () => {
      await expect(draw).toHaveAttribute("aria-pressed", "false");
      await expect(undo).toBeDisabled();
      // The rail is icon-only, so the reason is the tooltip rather than a
      // `title` — and the tooltip stays in the document while it is shut.
      await expect(canvas.getByText("Nothing has been drawn by hand.")).toBeInTheDocument();
    });

    await step("the pen says it is down", async () => {
      await userEvent.click(draw);
      await waitFor(() => expect(draw).toHaveAttribute("aria-pressed", "true"));
    });

    await step("a drag leaves a line, and Undo comes alive", async () => {
      drag(graph, [
        { x: 60, y: 60 },
        { x: 110, y: 90 },
        { x: 160, y: 60 },
        { x: 210, y: 120 },
      ]);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1));
      await expect(undo).toBeEnabled();
    });

    await step("a press that goes nowhere is not a line", async () => {
      drag(graph, [{ x: 240, y: 200 }]);
      await expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1);
    });

    await step("a second line goes on top, and Undo takes back only the last", async () => {
      drag(graph, [
        { x: 80, y: 190 },
        { x: 150, y: 190 },
        { x: 220, y: 175 },
      ]);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(2));
      await userEvent.click(undo);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1));
      await expect(undo).toBeEnabled();
    });

    await step("the last line back leaves the pad empty and Undo off again", async () => {
      await userEvent.click(undo);
      await waitFor(() => expect(canvas.queryAllByRole("img", { name: A_HAND_LINE })).toHaveLength(0));
      await expect(undo).toBeDisabled();
    });

    await step("any other act puts the pen down", async () => {
      // The pen has been down throughout: nothing above it puts it away.
      await expect(draw).toHaveAttribute("aria-pressed", "true");
      await userEvent.click(canvas.getByRole("button", { name: "Add a box" }));
      await waitFor(() => expect(draw).toHaveAttribute("aria-pressed", "false"));
    });
  },
};

/**
 * A paste, dispatched the way `DispatchRequest`'s own paste story dispatches
 * one: a real `DataTransfer` on a real `ClipboardEvent`, because
 * `fireEvent.paste` rebuilds `clipboardData` empty.
 */
function paste(target: Element, fill: (data: DataTransfer) => void): void {
  const data = new DataTransfer();
  fill(data);
  target.dispatchEvent(
    new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: data }),
  );
}

/** A real PNG, as a screenshot is: wider than the pad and decodable. */
async function screenshot(): Promise<File> {
  const canvas = new OffscreenCanvas(2880, 1800);
  const pen = canvas.getContext("2d");
  if (pen === null) throw new Error("No 2D context to draw the screenshot with.");
  pen.fillStyle = "gray";
  pen.fillRect(0, 0, 2880, 1800);
  const png = await canvas.convertToBlob({ type: "image/png" });
  return new File([png], "image.png", { type: "image/png" });
}

/**
 * ⌘V on the pad — the owner's note of 1 Oct 2026. **Text is a box holding it**
 * and a screenshot is a picture on the pad, both landing at once; a paste into
 * a box's own field is that field's, and makes nothing.
 *
 * **A `play`, because no rendering shows a paste**, and the third half is a
 * rule about what does *not* happen.
 */
export const Pasted: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, userEvent, step }) => {
    const graph = canvas.getByLabelText(args.label);

    await step("text pasted on the pad is a box holding it", async () => {
      paste(graph, (data) => data.setData("text/plain", "  https://example.com/ticket/42 \n"));
      await waitFor(() =>
        expect(
          canvas.getByRole("group", { name: "Box: https://example.com/ticket/42" }),
        ).toBeVisible(),
      );
    });

    await step("a screenshot pasted on the pad is a picture on it, not an attachment", async () => {
      const shot = await screenshot();
      paste(graph, (data) => data.items.add(shot));
      const picture = await canvas.findByRole("group", { name: "A pasted picture" });
      await expect(picture).toBeVisible();
      // Scaled down to sit inside the pad, rather than drawn at its own size.
      const pad = graph.getBoundingClientRect();
      const drawn = picture.getBoundingClientRect();
      await expect(drawn.width).toBeLessThan(pad.width);
      await expect(drawn.height).toBeLessThan(pad.height);
    });

    await step("a paste into a box's own field makes nothing new", async () => {
      const field = canvas.getByRole("textbox", { name: "The words in this box" });
      await userEvent.click(field);
      paste(field, (data) => data.setData("text/plain", "more words"));
      // A second paste on the pad, waited for, so a box the field's paste made
      // would be drawn by now too — an absence read straight after a paste
      // reads before React has drawn anything.
      paste(graph, (data) => data.setData("text/plain", "the panel"));
      await waitFor(() => expect(canvas.getByRole("group", { name: "Box: the panel" })).toBeVisible());
      await expect(canvas.queryByRole("group", { name: "Box: more words" })).toBeNull();
      await expect(canvas.getAllByRole("group", { name: /^Box: / })).toHaveLength(2);
      await expect(canvas.getAllByRole("group", { name: "A pasted picture" })).toHaveLength(1);
    });

    await step("the picture comes off under Remove, like a box", async () => {
      await userEvent.click(canvas.getByRole("group", { name: "A pasted picture" }));
      const remove = await canvas.findByRole("button", { name: "Remove" });
      await userEvent.click(remove);
      await waitFor(() =>
        expect(canvas.queryByRole("group", { name: "A pasted picture" })).toBeNull(),
      );
    });
  },
};

/**
 * What a real ⌘V hands the page on macOS, measured in Bridge's own Electron
 * (the decision `2026-10-01-a-paste-lands-at-once.md`, on `studios/paste`).
 * Each text shape carries `text/plain`; what differs is what rides beside it.
 */
const CLIPBOARDS: { from: string; text: string; beside: Record<string, string> }[] = [
  { from: "an address bar", text: "https://example.com/tickets/1545", beside: {} },
  {
    from: "a page's text",
    text: "the count goes stale",
    beside: { "text/html": "<p>the count goes stale</p>" },
  },
  {
    from: "a terminal",
    text: "crates/fleet/src/briefing.rs",
    beside: {
      "text/html": "<pre>crates/fleet/src/briefing.rs</pre>",
      "text/rtf": "{\\rtf1 crates/fleet/src/briefing.rs}",
    },
  },
];

/** Whether a node is drawn inside the pad on show and is the top thing at its own middle. */
function inView(graph: HTMLElement, node: HTMLElement): boolean {
  const pad = graph.getBoundingClientRect();
  const at = node.getBoundingClientRect();
  const inside =
    at.left >= pad.left && at.top >= pad.top && at.right <= pad.right && at.bottom <= pad.bottom;
  const top = document.elementFromPoint(at.left + at.width / 2, at.top + at.height / 2);
  return inside && top !== null && node.contains(top);
}

/**
 * ⌘V on a pad that already holds a drawing — **the owner's "not allowing me to
 * paste links or anything else"**, 1 Oct 2026. Every paste landed a box, but
 * placed a box-width down and across from each box it came near, which on
 * this pad put it below the pad's own height: in the draft, out of sight. A
 * picture skipped that rule, which is why only screenshots seemed to work.
 *
 * **So what is asserted is that each lands where a person can see it**, on
 * top of whatever is there, for each shape a real clipboard carries — and
 * that a screenshot is still a picture.
 */
export const PastedOnADrawing: Story = {
  args: { ...args, strokes: [] },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, step }) => {
    const graph = canvas.getByLabelText(args.label);

    for (const clipboard of CLIPBOARDS) {
      await step(`text copied from ${clipboard.from} lands in view, as a box`, async () => {
        paste(graph, (data) => {
          data.setData("text/plain", clipboard.text);
          for (const [type, value] of Object.entries(clipboard.beside)) data.setData(type, value);
        });
        const box = await canvas.findByRole("group", { name: `Box: ${clipboard.text}` });
        await waitFor(() => expect(inView(graph, box)).toBe(true));
      });
    }

    await step("a screenshot is still a picture, and lands in view", async () => {
      const shot = await screenshot();
      paste(graph, (data) => data.items.add(shot));
      const picture = await canvas.findByRole("group", { name: "A pasted picture" });
      const pad = graph.getBoundingClientRect();
      await waitFor(() => {
        const drawn = picture.getBoundingClientRect();
        expect(drawn.top >= pad.top && drawn.bottom <= pad.bottom).toBe(true);
      });
    });

    await step("text pasted after a picture is drawn over it, not hidden under it", async () => {
      paste(graph, (data) => data.setData("text/plain", "over the picture"));
      const box = await canvas.findByRole("group", { name: "Box: over the picture" });
      await waitFor(() => expect(inView(graph, box)).toBe(true));
    });
  },
};
