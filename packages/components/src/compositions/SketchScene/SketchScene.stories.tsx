import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, screen, userEvent, waitFor } from "storybook/test";

import { SketchScene } from "./SketchScene";
import { SketchShareContext } from "./SketchShare";
import type { SketchShareFile, SketchShareHost, SketchShareSaved } from "./share";
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

/** Played in the panel, the dot is the only thing that moves: the other arrows stop their crawl. */
export const PanelPlayingMovesOnlyTheDot: Story = {
  args: { scene: HUB, autoplay: true },
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-active]")).not.toBeNull());
    await expect(new Set(running(canvasElement))).toEqual(new Set(["armada-scene-travel"]));
  },
};

/** Not playing, the panel's flow arrows crawl as before. */
export const PanelCrawlsWhenNothingPlays: Story = {
  args: { scene: HUB },
  parameters: { motion: "on" },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.querySelector(".armada-scene-edge[data-flow]")).not.toBeNull());
    await expect(canvasElement.querySelector("[data-playing]")).toBeNull();
    await waitFor(() => expect(running(canvasElement)).toContain("armada-scene-flow"));
  },
};

/** The host the sketch is shared through, stubbed: what each item asks of it is what these stories read. */
const host = {
  save: fn(async (files: readonly SketchShareFile[]): Promise<SketchShareSaved> => ({ saved: true, name: files[0]!.name })),
  copy: fn(async (_text: string) => undefined),
  copied: fn(),
  said: fn(),
} satisfies SketchShareHost;

const shared: Story = {
  args: { scene: HUB, title: "How data moves" },
  render: (args) => (
    <SketchShareContext.Provider value={host}>
      <SketchScene {...args} />
    </SketchShareContext.Provider>
  ),
};

const choose = async (canvas: Parameters<NonNullable<Story["play"]>>[0]["canvas"], item: string) => {
  for (const one of Object.values(host)) one.mockClear();
  await userEvent.click(await canvas.findByRole("button", { name: "Share" }));
  await userEvent.click(await screen.findByRole("menuitem", { name: item }));
};

const text = (data: string | Uint8Array) => (typeof data === "string" ? data : new TextDecoder().decode(data));

/** Share opens a menu of three, and Save as web page hands the host one page named for the sketch, with the drawing and its steps in it. */
export const ShareAsWebPage: Story = {
  ...shared,
  play: async ({ canvas }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Share" }));
    await expect(screen.getAllByRole("menuitem").map((one) => one.textContent)).toEqual(["Save as web page", "Save as image", "Copy as Mermaid"]);
    await userEvent.keyboard("{Escape}");
    await choose(canvas, "Save as web page");
    await waitFor(() => expect(host.save).toHaveBeenCalledTimes(1));
    const files = host.save.mock.calls[0]![0];
    await expect(files.map((one) => [one.kind, one.name])).toEqual([["html", "How data moves.html"]]);
    const page = text(files[0]!.data);
    await expect(page.startsWith("<!doctype html>")).toBe(true);
    await expect(page).toContain("WS /jobs/:id/observe and run observe, HTTP for commands");
    await expect(page).toContain('id="sketch-steps"');
    // Every box's words are all there, broken into lines as the canvas broke them.
    const drawn = new DOMParser().parseFromString(page, "text/html");
    for (const node of HUB.nodes) {
      if (node.body === undefined) continue;
      const lines = [...drawn.querySelectorAll(`[data-node="${node.id}"] .sk-body tspan`)].map((one) => one.textContent ?? "");
      await expect(lines.join("").replace(/\s/g, "")).toBe(node.body.replace(/\s/g, ""));
    }
    await expect(page).not.toMatch(/https?:/);
    await waitFor(() => expect(host.said).toHaveBeenCalledWith("Saved How data moves.html."));
  },
};

/** Save as image offers the same drawing as a PNG at twice its size, and as an SVG, for the person to choose between in the save dialog. */
export const ShareAsImage: Story = {
  ...shared,
  play: async ({ canvas }) => {
    await choose(canvas, "Save as image");
    await waitFor(() => expect(host.save).toHaveBeenCalledTimes(1));
    const [png, svg] = host.save.mock.calls[0]![0] as [SketchShareFile, SketchShareFile];
    await expect([png.kind, png.name, svg.kind, svg.name]).toEqual(["png", "How data moves.png", "svg", "How data moves.svg"]);
    await expect([...(png.data as Uint8Array).slice(0, 4)]).toEqual([0x89, 0x50, 0x4e, 0x47]);
    const view = new DataView((png.data as Uint8Array).buffer, (png.data as Uint8Array).byteOffset);
    const [w, h] = /viewBox="[-\d.]+ [-\d.]+ ([\d.]+) ([\d.]+)"/.exec(text(svg.data))!.slice(1).map(Number) as [number, number];
    await expect([view.getUint32(16), view.getUint32(20)]).toEqual([Math.round(w * 2), Math.round(h * 2)]);
    await expect(text(svg.data)).toContain("Fleet");
  },
};

/** A sign in screen with a step inside it and an arrow to the box it opens. */
const SIGN_IN: Scene = {
  nodes: [
    {
      id: "login",
      kind: "screen",
      x: 0,
      y: 0,
      w: 420,
      title: "Sign in",
      screen: {
        type: "window",
        chrome: "desktop",
        title: "Tally",
        children: [
          {
            type: "column",
            gap: "md",
            pad: "md",
            size: "fill",
            children: [
              { type: "text", style: "heading", text: "Welcome back" },
              { type: "button", id: "submit", text: "Sign in", variant: "primary", step: 1 },
            ],
          },
        ],
      },
    },
    { id: "home", kind: "box", x: 600, y: 0, w: 240, h: 100, title: "Home", step: 2 },
  ],
  edges: [{ id: "go", from: "login", to: "home", label: "Signed in", step: 2 }],
};

/** A screen is saved as the web page draws it, once at rest and once at the step its button carries, with the rules that style it and the player's controls. */
export const ShareScreenAsWebPage: Story = {
  ...shared,
  args: { scene: SIGN_IN, title: "Sign in flow" },
  play: async ({ canvas }) => {
    await choose(canvas, "Save as web page");
    await waitFor(() => expect(host.save).toHaveBeenCalledTimes(1));
    const page = text(host.save.mock.calls[0]![0][0]!.data);
    const drawn = new DOMParser().parseFromString(page, "text/html");
    await expect([...drawn.querySelectorAll("[data-node='login'] .sk-screen")].map((one) => one.getAttribute("data-at"))).toEqual(["rest", "1"]);
    await expect(drawn.querySelector("[data-node='login'] [data-at='rest'] .armada-wireframe--button")!.textContent).toBe("Sign in");
    await expect(drawn.querySelector("[data-node='login'] [data-at='1'] .armada-wireframe--button")!.hasAttribute("data-current")).toBe(true);
    await expect(drawn.querySelector("svg style")!.textContent).toContain(".armada-wireframe");
    await expect(drawn.getElementById("sketch-play")).not.toBeNull();
    await expect(page).not.toMatch(/https?:/);
  },
};

/** A screen is saved as an image with its sheet drawn, under its name, in a picture a browser reads back. */
export const ShareScreenAsImage: Story = {
  ...shared,
  args: { scene: SIGN_IN, title: "Sign in flow" },
  play: async ({ canvas }) => {
    await choose(canvas, "Save as image");
    await waitFor(() => expect(host.save).toHaveBeenCalledTimes(1));
    const [png, svg] = host.save.mock.calls[0]![0] as [SketchShareFile, SketchShareFile];
    await expect([...(png.data as Uint8Array).slice(0, 4)]).toEqual([0x89, 0x50, 0x4e, 0x47]);
    const drawn = new DOMParser().parseFromString(text(svg.data), "image/svg+xml");
    await expect(drawn.querySelector("parsererror")).toBeNull();
    await expect(drawn.querySelector(".sk-caption")!.textContent).toBe("Sign in");
    await expect(drawn.querySelector("foreignObject .armada-wireframe--button")!.textContent).toBe("Sign in");
    await expect(drawn.querySelectorAll(".sk-screen")).toHaveLength(1);
  },
};

/** Copy as Mermaid puts a flowchart on the clipboard with the arrows' words escaped, and says what was copied. */
export const ShareAsMermaid: Story = {
  ...shared,
  play: async ({ canvas }) => {
    await choose(canvas, "Copy as Mermaid");
    await waitFor(() => expect(host.copy).toHaveBeenCalledTimes(1));
    const mermaid = host.copy.mock.calls[0]![0];
    await expect(mermaid.startsWith("flowchart ")).toBe(true);
    await expect(mermaid).toContain('-- "WS /jobs/:id/observe and run observe, HTTP for commands" -->');
    await waitFor(() => expect(host.copied).toHaveBeenCalledWith("The Mermaid text"));
  },
};

/** A save the host could not make is said, and one the person cancelled is not. */
export const ShareSaveThatFails: Story = {
  ...shared,
  play: async ({ canvas }) => {
    host.save.mockResolvedValueOnce({ saved: false, why: "failed" });
    await choose(canvas, "Save as web page");
    await waitFor(() => expect(host.said).toHaveBeenCalledWith("How data moves.html was not saved."));
    host.save.mockResolvedValueOnce({ saved: false, why: "cancelled" });
    await choose(canvas, "Save as web page");
    await waitFor(() => expect(host.save).toHaveBeenCalledTimes(1));
    await expect(host.said).not.toHaveBeenCalled();
  },
};

/** A card in a thread has no rail, so no Share. */
export const PreviewHasNoShare: Story = {
  args: { scene: HUB, preview: true },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(8));
    await expect(canvas.queryByRole("button", { name: "Share" })).toBeNull();
  },
};

/** The sketch from the owner's note (20 Oct 2026): placed for 240 by 90 on a three-column grid, with bodies far longer than that. */
const NOTICE: Scene = {
  nodes: [
    { id: "queues", kind: "box", x: 0, y: 0, title: "Agent queues #2645", body: "gh pr merge --auto, 02:12", icon: "git-merge" },
    { id: "auto", kind: "box", x: 320, y: 0, title: "Fleet: auto-merge on", body: "Told once per pull request; the wait is taken off", icon: "bot" },
    { id: "left", kind: "box", x: 640, y: 0, title: "Left the queue", body: "conflicts, checks failed, 02:23 to 06:36", icon: "activity" },
    { id: "again", kind: "box", x: 0, y: 170, title: "Queues it again", body: "Agent pushes the fix and states a new wait, 11:32", icon: "git-merge" },
    { id: "reads", kind: "box", x: 320, y: 170, title: "Fleet reads: in the queue, auto-merge null", body: "pull_notices.rs:219 reads auto_merge only, and the notice is already told: nothing takes the wait off", icon: "eye" },
    { id: "needs", kind: "box", x: 640, y: 170, title: "Needs you shows the row", body: "Nothing for him to do", icon: "scale" },
    { id: "fix", kind: "box", x: 320, y: 340, title: "Fix", body: "Queue or auto-merge, at every reading: take the wait off, tell the Session only where something came off; agents never state it", icon: "wrench" },
  ],
  edges: [
    { id: "a", from: "queues", to: "auto" },
    { id: "b", from: "auto", to: "left" },
    { id: "c", from: "auto", to: "again" },
    { id: "d", from: "again", to: "reads" },
    { id: "e", from: "reads", to: "needs" },
    { id: "f", from: "reads", to: "fix", label: "becomes" },
  ],
};

/** Whether a point on the screen is inside a box, by more than its border. */
const inside = (p: DOMPoint, r: DOMRect) => p.x > r.left + 3 && p.x < r.right - 3 && p.y > r.top + 3 && p.y < r.bottom - 3;

/** What a person sees of an agent's sketch: no box on another, every arrow clear of the boxes it passes, words they can read, and never blown up past full size. */
export const AutoMergeNotice: Story = {
  args: { scene: NOTICE },
  play: async ({ canvas, canvasElement }) => {
    await waitFor(() => expect(nodes(canvasElement).length).toBe(7));
    await waitFor(() => expect(fitted(canvasElement)).toBe(true));
    // Boxes are moved apart once measured, a beat after the first fit.
    await new Promise((done) => setTimeout(done, 300));
    const box = (id: string) => canvasElement.querySelector(`.react-flow__node[data-id="${id}"] .armada-scene-node`)!.getBoundingClientRect();
    const ids = NOTICE.nodes.map((one) => one.id);
    for (const [i, one] of ids.entries()) for (const other of ids.slice(i + 1)) await expect(crosses(box(one), box(other)), `${one} on ${other}`).toBe(false);
    for (const edge of NOTICE.edges) {
      const line = canvasElement.querySelector<SVGPathElement>(`.react-flow__edge[data-id="${edge.id}"] .armada-scene-edge__line`)!;
      const toScreen = line.getScreenCTM()!;
      const length = line.getTotalLength();
      for (let at = 0; at <= length; at += 4) {
        const p = line.getPointAtLength(at).matrixTransform(toScreen);
        for (const id of ids.filter((one) => one !== edge.from && one !== edge.to)) await expect(inside(p, box(id)), `${edge.id} runs through ${id}`).toBe(false);
      }
    }
    await expect(getComputedStyle(canvas.getByText(/pull_notices.rs:219/)).fontFamily).toContain("IBM Plex Sans");
    await expect(scale(canvasElement)).toBeLessThanOrEqual(1);
  },
};

