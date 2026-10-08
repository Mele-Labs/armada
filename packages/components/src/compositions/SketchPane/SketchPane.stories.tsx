import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { SketchPane } from "./SketchPane";

/** A Drone's diagram, drawn where the canvas would be. */
const meta: Meta<typeof SketchPane> = {
  title: "Compositions/Sketch pane",
  component: SketchPane,
};
export default meta;

type Story = StoryObj<typeof SketchPane>;

export const Flowchart: Story = {
  args: {
    sketch: {
      source: "flowchart LR\n  Writer --> Store\n  Clock --> Store",
      svg:
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 130" width="400" height="130">' +
        '<line class="sk-edge" x1="150" y1="22" x2="230" y2="67"/><line class="sk-edge" x1="150" y1="112" x2="230" y2="67"/>' +
        '<rect class="sk-node" x="0" y="0" width="150" height="44" rx="6"/><text class="sk-text" x="75" y="22">Writer</text>' +
        '<rect class="sk-node" x="0" y="90" width="150" height="44" rx="6"/><text class="sk-text" x="75" y="112">Clock</text>' +
        '<rect class="sk-node" x="230" y="45" width="150" height="44" rx="6"/><text class="sk-text" x="305" y="67">Store</text></svg>',
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Sketch" })).toHaveTextContent("Writer");
  },
};
