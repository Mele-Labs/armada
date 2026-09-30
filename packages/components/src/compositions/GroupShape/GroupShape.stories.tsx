import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { GroupShape } from "./GroupShape";

const meta: Meta<typeof GroupShape> = {
  title: "Compositions/Group shape",
  component: GroupShape,
};
export default meta;

type Story = StoryObj<typeof GroupShape>;

/**
 * Every task circle, and the two marks the run sits between.
 *
 * **Neither end is a circle**, which is the claim: a round mark means a task,
 * so the opening bar and the boundary diamond are what keep the count honest.
 */
function parts(canvas: ReturnType<typeof within>) {
  const shape = canvas.getByRole("img");
  return {
    tasks: shape.querySelectorAll(".armada-group-shape__task").length,
    circles: shape.querySelectorAll("circle").length,
    starts: shape.querySelectorAll(".armada-group-shape__start").length,
    gates: shape.querySelectorAll(".armada-group-shape__gate").length,
  };
}

/** Tasks in a line: one circle each, between a start and the boundary. */
export const Sequential: Story = {
  args: { tasks: 3, concurrent: false, label: "3 tasks, one after another" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "3 tasks, one after another" })).toBeVisible();
    expect(parts(canvas)).toEqual({ tasks: 3, circles: 3, starts: 1, gates: 1 });
  },
};

/**
 * Tasks side by side. **The same count draws the same number of circles** —
 * what changes is where they sit, which is the whole of what the drawing says.
 */
export const Concurrent: Story = {
  args: { tasks: 3, concurrent: true, label: "3 tasks, at the same time" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "3 tasks, at the same time" })).toBeVisible();
    expect(parts(canvas)).toEqual({ tasks: 3, circles: 3, starts: 1, gates: 1 });
  },
};

/** One task is neither ordered nor concurrent, and draws as one circle. */
export const One: Story = {
  args: { tasks: 1, concurrent: false, label: "1 task, on its own" },
  play: async ({ canvasElement }) => {
    expect(parts(within(canvasElement))).toEqual({ tasks: 1, circles: 1, starts: 1, gates: 1 });
  },
};

/**
 * **Past five the dots stop being countable**, so the drawing elides rather
 * than growing — the label is what carries the real number, and a row of
 * twelve circles is texture rather than a count.
 */
export const Elided: Story = {
  args: { tasks: 12, concurrent: false, label: "12 tasks, one after another" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    expect(parts(canvas).tasks).toBe(5);
    await expect(canvas.getByRole("img", { name: "12 tasks, one after another" })).toBeVisible();
  },
};
