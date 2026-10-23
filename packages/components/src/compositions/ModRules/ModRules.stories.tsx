import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { Card, CardContent } from "../../primitives/Card/Card";
import { ModRules } from "./ModRules";

const meta: Meta<typeof ModRules> = {
  title: "Compositions/Mod rules",
  component: ModRules,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Card>
          <CardContent>
            <Story />
          </CardContent>
        </Card>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ModRules>;

/** A word mod: what it finds, and the address it was found to go to. */
export const Words: Story = {
  args: {
    name: "Issue links",
    parts: [
      {
        part: "Words",
        facts: [
          { label: "Pattern", value: "#\\d+", code: true },
          { label: "Opens", value: "https://tracker.example/issues/{n}", code: true, from: "origin remote" },
        ],
      },
    ],
  },
  play: async ({ canvasElement }) => {
    const group = within(canvasElement).getByRole("group", { name: "Details of Issue links" });
    await expect(within(group).getByRole("region", { name: "Words" })).toBeVisible();
    await expect(within(group).getByText("origin remote")).toBeVisible();
  },
};

/** A fleet mod: what it matches and what it does. */
export const Fleet: Story = {
  args: {
    name: "Hold on failed checks",
    parts: [
      {
        part: "Fleet",
        facts: [
          { label: "Notice", value: "Checks failed" },
          { label: "Matches", value: "^lint", code: true },
          { label: "Does", value: "Holds the Job" },
        ],
      },
    ],
  },
};

/** A theme: a fact with no label continues the one above, as the places a colour appears do. */
export const Theme: Story = {
  args: {
    name: "Dusk",
    parts: [
      {
        part: "Colour",
        facts: [
          { label: "Accent", value: "--accent-muted", code: true },
          { label: "", value: "Job row" },
          { label: "", value: "Tab strip" },
        ],
      },
    ],
  },
  play: async ({ canvasElement }) => {
    const part = within(canvasElement).getByRole("region", { name: "Colour" });
    await expect(within(part).getByText("Job row")).toBeVisible();
    await expect(within(part).getByText("Accent")).toBeVisible();
  },
};

/** One mod changing two things holds a part for each. */
export const TwoParts: Story = {
  args: {
    name: "Plain notices",
    parts: [
      { part: "Words", facts: [{ label: "Pattern", value: "out of date", code: true }] },
      { part: "Fleet", facts: [{ label: "Does", value: "Holds the Job" }] },
    ],
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole("region")).toHaveLength(2);
  },
};
