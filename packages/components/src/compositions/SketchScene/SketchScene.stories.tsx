import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, waitFor } from "storybook/test";

import { SketchScene } from "./SketchScene";
import { NO_MARKS, type Scene, type SceneMarks } from "./scene";

/** A scene drawn in Armada's frames, with pan, zoom, fit, a pen, and a press on a part to ask about it. */
const meta: Meta<typeof SketchScene> = {
  title: "Compositions/Sketch scene",
  component: SketchScene,
  decorators: [
    (Story) => (
      <div style={{ inlineSize: "100%" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SketchScene>;

const FLOW: Scene = {
  nodes: [
    { id: "g", kind: "group", x: -20, y: -20, w: 560, h: 200, title: "store crate" },
    { id: "a", kind: "box", x: 0, y: 40, title: "Caller", body: "append(batch)", icon: "terminal", step: 1 },
    { id: "b", kind: "box", x: 300, y: 40, title: "Writer", body: "write(batch)\nnow()", icon: "hard-drive", step: 2 },
    { id: "c", kind: "code", x: 300, y: 240, lang: "rust", body: "let hour = clock.now();" },
    { id: "d", kind: "wire", x: 620, y: 40, wire: "input", title: "Retry cap" },
  ],
  edges: [{ id: "a-b", from: "a", to: "b", label: "append" }],
};

const AFTER: Scene = {
  nodes: [
    { id: "g", kind: "group", x: -20, y: -20, w: 560, h: 200, title: "store crate" },
    { id: "a", kind: "box", x: 0, y: 40, title: "Caller", body: "append(batch)", icon: "terminal", step: 1 },
    { id: "b", kind: "box", x: 300, y: 40, title: "Writer", body: "write(batch, hour)", icon: "hard-drive", step: 2 },
    { id: "e", kind: "box", x: 300, y: 240, title: "Clock", body: "now() -> Hour", icon: "clock" },
  ],
  edges: [{ id: "a-b", from: "a", to: "b", label: "append" }, { id: "e-b", from: "e", to: "b", label: "hour" }],
};

const nodes = (root: HTMLElement) => root.querySelectorAll(".react-flow__node");

/** Boxes, a group, code and a wireframe part, with the arrow's words. */
export const Architecture: Story = {
  args: { scene: FLOW },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    await expect(canvas.getByRole("img", { name: "Hard drive" })).toBeInTheDocument();
    await expect(canvas.getByText("append")).toBeInTheDocument();
  },
};

/** Against the current state: added is green, removed red, changed amber, and what stays is dimmed. */
export const BeforeAndAfter: Story = {
  args: { scene: AFTER, against: FLOW },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(6));
    const change = (name: string) => [...canvasElement.querySelectorAll(`[data-change="${name}"]`)].length;
    await expect(change("added")).toBeGreaterThan(0);
    await expect(change("removed")).toBeGreaterThan(0);
    await expect(change("changed")).toBeGreaterThan(0);
    await expect(canvasElement.querySelector('[data-change="same"][data-dim]')).not.toBeNull();
  },
};

/** A scene that does not validate draws nothing. */
export const InvalidSceneDrawsNothing: Story = {
  args: { scene: { nodes: [{ id: "a", kind: "sphere", x: 0, y: 0 }], edges: [] } },
  play: async ({ canvasElement }) => {
    await new Promise((done) => setTimeout(done, 200));
    await expect(nodes(canvasElement).length).toBe(0);
  },
};

/** An arrow to a node the scene does not hold is invalid too. */
export const DanglingArrowDrawsNothing: Story = {
  args: { scene: { nodes: [{ id: "a", kind: "box", x: 0, y: 0, title: "A" }], edges: [{ id: "x", from: "a", to: "gone" }] } },
  play: async ({ canvasElement }) => {
    await new Promise((done) => setTimeout(done, 200));
    await expect(nodes(canvasElement).length).toBe(0);
  },
};

/** A press on a part opens a small ask about it; sending it tells the host. */
export const AskAboutAPart: Story = {
  args: { scene: FLOW, onAsk: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(await canvas.findByRole("group", { name: "Writer" }));
    const said = await canvas.findByRole("textbox", { name: "Ask about Writer" });
    await userEvent.type(said, "Does it flush on close?");
    await userEvent.click(canvas.getByRole("button", { name: "Ask" }));
    await expect(args.onAsk).toHaveBeenCalledWith({ id: "b", label: "Writer" }, "Does it flush on close?");
  },
};

/** A press on an arrow asks about the arrow. */
export const AskAboutAnArrow: Story = {
  args: { scene: FLOW, onAsk: fn() },
  play: async ({ canvas }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "append" }));
    await expect(await canvas.findByRole("textbox", { name: "Ask about append" })).toBeInTheDocument();
  },
};

const scale = (root: HTMLElement) => {
  const transform = root.querySelector<HTMLElement>(".react-flow__viewport")?.style.transform ?? "";
  return Number(/scale\(([\d.]+)\)/.exec(transform)?.[1] ?? "0");
};

/** Zoom in, zoom out and fit are on the rail, and the keys do the same. */
export const ZoomAndFit: Story = {
  args: { scene: FLOW },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    const start = scale(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Zoom in" }));
    await waitFor(() => expect(scale(canvasElement)).toBeGreaterThan(start));
    await userEvent.click(canvas.getByRole("button", { name: "Fit" }));
    await waitFor(() => expect(scale(canvasElement)).toBeCloseTo(start, 1));
    canvasElement.querySelector<HTMLElement>(".armada-sketch")?.focus();
    await userEvent.keyboard("-");
    await waitFor(() => expect(scale(canvasElement)).toBeLessThan(start));
  },
};

/** Arrows pulse along their direction, and Play the steps lights the nodes in order. */
export const FlowAndSteps: Story = {
  args: { scene: FLOW },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    // Edges are drawn after their nodes are measured, so the nodes being there is not the edges being there.
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-flow]")).not.toBeNull());
    await userEvent.click(canvas.getByRole("button", { name: /Play the steps|Next step/ }));
    await waitFor(() => expect(canvasElement.querySelector('[data-current] .armada-scene-node__title, [data-current]')?.textContent).toContain("Caller"));
  },
};

/** The owner's marks, held as a host holds them. */
function Held(args: React.ComponentProps<typeof SketchScene>) {
  const [marks, setMarks] = useState<SceneMarks>(NO_MARKS);
  return <SketchScene {...args} marks={marks} onMarks={setMarks} />;
}

const mine = (root: HTMLElement, what: string) => root.querySelectorAll(`[data-mine]${what}`).length;

/** The pen draws on top and Undo takes the last line back; the host keeps what was drawn. */
export const DrawOnTop: Story = {
  args: { scene: FLOW },
  render: (args) => <Held {...args} />,
  play: async ({ canvas, canvasElement }) => {
    const draw = await canvas.findByRole("button", { name: "Draw" });
    await expect(canvas.getByRole("button", { name: "Undo" })).toBeDisabled();
    await userEvent.click(draw);
    await expect(draw).toHaveAttribute("aria-pressed", "true");
    const pen = canvasElement.querySelector(".armada-sketch-pad__pen")!;
    const box = pen.getBoundingClientRect();
    pen.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: box.left + 40, clientY: box.top + 40 }));
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: box.left + 90, clientY: box.top + 70 }));
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: box.left + 140, clientY: box.top + 60 }));
    window.dispatchEvent(new PointerEvent("pointerup", {}));
    await waitFor(() => expect(canvasElement.querySelectorAll(".armada-sketch-pad__ink path[role='img']").length).toBe(1));
  },
};

/** Add a box puts one of his on the sketch, in his colour, with a field for its words. */
export const AddABox: Story = {
  args: { scene: FLOW },
  render: (args) => <Held {...args} />,
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    await userEvent.click(canvas.getByRole("button", { name: "Add a box" }));
    await waitFor(() => expect(mine(canvasElement, ".armada-scene-node")).toBe(1));
    await userEvent.type(await canvas.findByRole("textbox", { name: "The words in your box" }), "Flush here");
    await expect(canvas.getByRole("group", { name: "Your box: Flush here" })).toBeInTheDocument();
  },
};

/** Pick a box, Join, then press another: his arrow joins them. */
export const JoinTwoBoxes: Story = {
  args: { scene: FLOW },
  render: (args) => <Held {...args} />,
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    await userEvent.click(await canvas.findByRole("group", { name: "Caller" }));
    await userEvent.click(await canvas.findByRole("button", { name: "Join" }));
    await userEvent.click(canvas.getByRole("group", { name: "Retry cap" }));
    await waitFor(() => expect(mine(canvasElement, ".armada-scene-edge")).toBe(1));
  },
};

/** Remove takes his own box off, and strikes a Drone's part out until it is put back. */
export const RemoveAPart: Story = {
  args: { scene: FLOW },
  render: (args) => <Held {...args} />,
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    await userEvent.click(await canvas.findByRole("group", { name: "Writer" }));
    await userEvent.click(await canvas.findByRole("button", { name: "Remove" }));
    await waitFor(() => expect(canvasElement.querySelectorAll(".armada-scene-node[data-struck]").length).toBe(1));
    await userEvent.click(await canvas.findByRole("button", { name: "Put back" }));
    await waitFor(() => expect(canvasElement.querySelectorAll("[data-struck]").length).toBe(0));
    await userEvent.click(canvas.getByRole("button", { name: "Add a box" }));
    await waitFor(() => expect(mine(canvasElement, ".armada-scene-node")).toBe(1));
    await userEvent.click(await canvas.findByRole("group", { name: "Your box" }));
    await userEvent.click(await canvas.findByRole("button", { name: "Remove" }));
    await waitFor(() => expect(mine(canvasElement, ".armada-scene-node")).toBe(0));
  },
};
