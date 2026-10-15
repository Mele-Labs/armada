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

/** The first fit lands after the boxes are measured, not with them, so until then the view is where React Flow opens it: at no offset and scale 1. */
const fitted = (root: HTMLElement) => {
  const transform = root.querySelector<HTMLElement>(".react-flow__viewport")?.style.transform ?? "";
  return transform !== "" && !/^translate\(0(px)?, ?0(px)?\) scale\(1\)$/.test(transform);
};

/** Zoom in, zoom out and fit are on the rail, and the keys do the same. */
export const ZoomAndFit: Story = {
  args: { scene: FLOW },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(5));
    await waitFor(() => expect(fitted(canvasElement)).toBe(true));
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

/** A sequence: lifelines for the parts, and an arrow between two points on them for each message, in order. */
const SEQUENCE: Scene = {
  nodes: [
    { id: "left", kind: "group", x: 0, y: 0, w: 200, h: 260, title: "Bridge" },
    { id: "right", kind: "group", x: 300, y: 0, w: 200, h: 260, title: "Fleet" },
    { id: "m1-from", kind: "label", x: 96, y: 100, title: "•" },
    { id: "m1-to", kind: "label", x: 396, y: 100, title: "•" },
    { id: "m2-from", kind: "label", x: 396, y: 180, title: "•" },
    { id: "m2-to", kind: "label", x: 96, y: 180, title: "•" },
  ],
  edges: [
    { id: "m1", from: "m1-from", to: "m1-to", label: "start", flow: false, step: 1 },
    { id: "m2", from: "m2-from", to: "m2-to", label: "changed", flow: false, step: 2 },
  ],
};

/** An arrow with a step sends a dot along itself and lights where it ends; the panel plays as it opens. */
export const StepsOnArrows: Story = {
  args: { scene: SEQUENCE, autoplay: true },
  parameters: { motion: "on" },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-active] .armada-scene-edge__dot")).not.toBeNull());
    expect(canvasElement.querySelector("[data-current]")?.textContent).toContain("•");
    // Each step lasts the pulse token, and the second arrow takes the first's place.
    await waitFor(() => expect(canvasElement.querySelector('.armada-scene-edge[data-active] [aria-label="Changed"], .armada-scene-edge[data-active] [aria-label="changed"]')).not.toBeNull(), { timeout: 4000 });
    expect(canvas.getByRole("button", { name: "Stop the steps" })).toBeEnabled();
    expect(canvas.getByRole("button", { name: "Restart the steps" })).toBeEnabled();
  },
};

/** An edge-only scene can be played, which a scene with a step on no node could not before. */
export const StepsOnArrowsPlayByHand: Story = {
  args: { scene: SEQUENCE },
  play: async ({ canvas, canvasElement }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Play the steps|Next step/ }));
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-active]")).not.toBeNull());
  },
};

const running = (root: HTMLElement) => root.getAnimations({ subtree: true }).filter((one): one is CSSAnimation => one instanceof CSSAnimation && one.playState === "running").map((one) => one.animationName);

/** A preview is the same drawing in a card's box: still until it is told to play, and nothing in it is operable. */
export const PreviewStill: Story = {
  args: { scene: SEQUENCE, preview: true, playing: false },
  parameters: { motion: "on" },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(6));
    expect(canvasElement.querySelector(".armada-scene-edge[data-active]")).toBeNull();
    expect(canvasElement.querySelector("figure")).toHaveAttribute("inert");
    expect(canvas.queryAllByRole("button")).toHaveLength(0);
    expect(canvasElement.querySelectorAll('[tabindex]:not([tabindex="-1"])')).toHaveLength(0);
  },
};

export const PreviewPlaying: Story = {
  args: { scene: SEQUENCE, preview: true, playing: true },
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-active]")).not.toBeNull());
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

/** The hub an agent drew for how data moves through Fleet: converging labelled arrows, and a body longer than the `h` it asked for. */
const HUB: Scene = {
  nodes: [
    { id: "you", kind: "label", x: 118, y: -80, title: "you", step: 1 },
    { id: "bridge", kind: "box", x: 0, y: 0, w: 240, h: 100, title: "Bridge", icon: "globe", body: "Electron, apps/desktop.\nOne connection, never talks to a Drone", step: 2 },
    { id: "sessions", kind: "box", x: 520, y: 0, w: 240, h: 100, title: "Agent sessions", icon: "bot", body: "armada-fleet MCP. No stream: poll with a cursor", step: 9 },
    { id: "check", kind: "box", x: -420, y: 300, w: 240, h: 100, title: "Check", icon: "hammer", body: "The repo's own Check, run in the Job's worktree", step: 5 },
    { id: "fleet", kind: "box", x: 0, y: 300, w: 240, h: 100, title: "Fleet", icon: "server", body: "Rust daemon: fleet + api + ipc.\nProtocol ID must match Bridge's or no socket opens; /v0 lifeboat stays up for a refused Bridge", step: 3 },
    { id: "sqlite", kind: "box", x: 520, y: 300, w: 240, h: 100, title: "SQLite", icon: "hard-drive", body: "crates/store", step: 7 },
    { id: "drone-a", kind: "box", x: -120, y: 590, w: 240, h: 90, title: "Drone", icon: "terminal", body: "Own worktree, --strict-mcp-config, no push", step: 4 },
    { id: "drone-b", kind: "box", x: 240, y: 590, w: 240, h: 90, title: "Drone", icon: "terminal", body: "Own worktree, --strict-mcp-config, no push", step: 4 },
  ],
  edges: [
    { id: "e0", from: "you", to: "bridge", step: 1 },
    { id: "e1", from: "bridge", to: "fleet", label: "WS /jobs/:id/observe and run observe, HTTP for commands", step: 8 },
    { id: "e2", from: "sessions", to: "fleet", label: "MCP: get_events_since(upto), get_job, start_run, show_sketch", step: 9 },
    { id: "e3", from: "fleet", to: "check", label: "code: advances, retry, fails", step: 5 },
    { id: "e4", from: "fleet", to: "sqlite", label: "every transition", step: 7 },
    { id: "e5", from: "drone-a", to: "fleet", label: "evidence via Armada's tool", step: 4 },
    { id: "e6", from: "drone-b", to: "fleet", label: "evidence via Armada's tool", step: 4 },
  ],
};

const rectOf = (el: Element) => el.getBoundingClientRect();
const crosses = (a: DOMRect, b: DOMRect) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

/** What a person sees of the arrows' words: each on its own backing, clear of every other and of every box, and a box tall enough for all of its. */
export const LabelsClearAndBoxesFit: Story = {
  args: { scene: HUB },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(8));
    await waitFor(() => expect(fitted(canvasElement)).toBe(true));
    // The words are placed from the boxes' measured sizes, a beat after the fit.
    await new Promise((done) => setTimeout(done, 300));
    const backings = [...canvasElement.querySelectorAll(".armada-scene-edge__backing")];
    await expect(backings).toHaveLength(6);
    const boxes = [...canvasElement.querySelectorAll(".armada-scene-node:not([data-kind='label'])")];
    for (const [i, one] of backings.entries()) {
      for (const other of backings.slice(i + 1)) await expect(crosses(rectOf(one), rectOf(other))).toBe(false);
      for (const box of boxes) await expect(crosses(rectOf(one), rectOf(box))).toBe(false);
    }
    // The words stay inside their backing, and the full text is the arrow's name.
    for (const word of canvasElement.querySelectorAll(".armada-scene-edge__label")) {
      const backing = rectOf(word.closest(".armada-scene-edge__words")!.querySelector(".armada-scene-edge__backing")!);
      const text = rectOf(word);
      await expect(text.left).toBeGreaterThanOrEqual(backing.left - 1);
      await expect(text.right).toBeLessThanOrEqual(backing.right + 1);
    }
    await expect(canvas.getByRole("button", { name: "MCP: get_events_since(upto), get_job, start_run, show_sketch" })).toBeInTheDocument();
    // A body longer than the box's `h` grows the box rather than being cut.
    for (const box of canvasElement.querySelectorAll(".armada-scene-node")) await expect(box.scrollHeight).toBeLessThanOrEqual(box.clientHeight);
    await expect(canvas.getByText(/lifeboat stays up/)).toBeVisible();
    const fleet = canvas.getByRole("group", { name: "Fleet" }).querySelector(".armada-scene-node")!;
    await expect(fleet.getBoundingClientRect().height / scale(canvasElement)).toBeGreaterThan(100);
  },
};

/** A card the thread is not playing is still: its arrows do not crawl, and no dot travels. */
export const PreviewStillDoesNotMove: Story = {
  args: { scene: HUB, preview: true, playing: false },
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(8));
    await waitFor(() => expect(canvasElement.querySelectorAll(".armada-scene-edge__line")).toHaveLength(7));
    await expect(running(canvasElement)).toEqual([]);
  },
};

/** Played, a card moves the step's dot and nothing else: the other arrows do not crawl under it. */
export const PreviewPlayingMovesOnlyTheDot: Story = {
  args: { scene: HUB, preview: true, playing: true },
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-active]")).not.toBeNull());
    await expect(new Set(running(canvasElement))).toEqual(new Set(["armada-scene-travel"]));
  },
};
