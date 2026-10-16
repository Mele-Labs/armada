import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { SubagentMarks } from "./SubagentMarks";

/** A subagent's type and model, as the head of its reading sheet states them. */
const meta: Meta<typeof SubagentMarks> = {
  title: "Compositions/Subagent marks",
  component: SubagentMarks,
};
export default meta;

type Story = StoryObj<typeof SubagentMarks>;

/** Both known: the type bare, and the model worded as the Session's composer words it. */
export const TypeAndModel: Story = {
  args: { type: "bridge-engineer", model: "vendor-opus-5-5" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("bridge-engineer")).toBeVisible();
    await expect(canvas.getByText("Opus 5.5")).toBeVisible();
  },
};

/** Nothing has answered yet, so no model is known: its slot stays empty. */
export const TypeOnly: Story = {
  args: { type: "general-purpose" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("general-purpose")).toBeVisible();
    await expect(canvas.queryByText("Opus 5.5")).not.toBeInTheDocument();
  },
};

/** Neither is known: nothing is drawn. */
export const Neither: Story = {
  play: async ({ canvasElement }) => {
    await expect(canvasElement).toBeEmptyDOMElement();
  },
};
