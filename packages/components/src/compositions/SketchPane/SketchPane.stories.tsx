import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";

import { SketchPane } from "./SketchPane";
import { cleaned, safeCss } from "./sketch-render";

/** A Drone's diagram, drawn from its mermaid source where the canvas would be. */
const meta: Meta<typeof SketchPane> = {
  title: "Compositions/Sketch pane",
  component: SketchPane,
};
export default meta;

type Story = StoryObj<typeof SketchPane>;

const drawing = (canvas: HTMLElement) => canvas.querySelector(".armada-sketch__drawing")?.shadowRoot ?? null;

export const Flowchart: Story = {
  args: { sketch: { source: "flowchart LR\n  Writer --> Store\n  Clock --> Store" } },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(drawing(canvasElement)?.textContent).toContain("Writer"));
  },
};

export const StateDiagram: Story = {
  args: { sketch: { source: "stateDiagram-v2\n  [*] --> Waiting\n  Waiting --> Retrying\n  Retrying --> Capped" } },
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(drawing(canvasElement)?.textContent).toContain("Retrying"));
  },
};

/** A source that does not parse leaves the frame empty. */
export const NotMermaid: Story = {
  args: { sketch: { source: "this is not a diagram ->" } },
  play: async ({ canvasElement }) => {
    await new Promise((done) => setTimeout(done, 300));
    await expect(drawing(canvasElement)?.textContent ?? "").toBe("");
  },
};

const hostile =
  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 10 10" onload="x()">' +
  "<style>.a{fill:red}@import url(https://e.test/x.css);.b{background:url(https://e.test/p.png)}</style>" +
  '<script>alert(1)</script><foreignObject><div>h</div></foreignObject>' +
  '<a xlink:href="https://e.test"><rect class="a" style="fill:blue" onclick="x()" width="1" height="1"/></a>' +
  '<use href="#k"/><text>Writer</text></svg>';


/** Mermaid's output is stripped again before it is inserted: no script, event or style attribute, outward link or fetching rule. */
export const HostileSvgIsCleaned: Story = {
  args: { sketch: { source: "flowchart LR\n  A --> B" } },
  play: async () => {
    const out = cleaned(hostile)!;
    await expect(out.svg).not.toMatch(/script|foreignObject|onload|onclick|style=|e\.test|<use/);
    await expect(out.svg).toContain("Writer");
    await expect(out.css).toContain(".a{fill:red}");
    await expect(out.css).not.toMatch(/@import|e\.test/);
    await expect(safeCss(".m{marker-end:url(#arrow)}")).toBe(".m{marker-end:url(#arrow)}");
    await expect(cleaned("<div>no</div>")).toBeUndefined();
  },
};
