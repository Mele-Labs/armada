import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { TerminalPane, type TerminalPaneProps } from "./TerminalPane";

/**
 * A command running in a terminal of its own. The frame is the component and the body is whatever the
 * caller draws: here a plain well, in the app the terminal screen. One story per state of the run.
 */
const meta: Meta<typeof TerminalPane> = {
  title: "Primitives/Terminal pane",
  component: TerminalPane,
  args: {
    command: "cd ~/Development/armada/.armada/slots/slot-1/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy",
    onStop: fn(),
    onRerun: fn(),
    onClose: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 12)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof TerminalPane>;

const body = (text: string): TerminalPaneProps["children"] => <pre style={{ margin: 0, padding: "var(--space-2) var(--space-3)", fontFamily: "var(--font-mono)" }}>{text}</pre>;

export const Running: Story = {
  args: { phase: { state: "running" }, children: body("Attempting to login via OAuth...") },
  play: async ({ canvasElement }) => {
    const pane = within(canvasElement);
    await expect(pane.getByRole("img", { name: "Running" })).toBeVisible();
    await expect(pane.getByRole("button", { name: "Stop" })).toBeVisible();
    await expect(pane.queryByRole("button", { name: "Close" })).toBeNull();
  },
};

export const Passed: Story = {
  args: { phase: { state: "ended", code: 0 }, children: body("Current Version ID: 6f2c1e0a-4b7d-4c1e-9a55-0d3f8e2b71c4") },
  play: async ({ canvasElement }) => {
    const pane = within(canvasElement);
    await expect(pane.getByRole("img", { name: "Passed" })).toBeVisible();
    await expect(pane.getByRole("button", { name: "Run again" })).toBeVisible();
    await expect(pane.getByRole("button", { name: "Close" })).toBeVisible();
  },
};

export const Failed: Story = {
  args: { phase: { state: "ended", code: 1 }, children: body("Aborted.") },
};

export const Stopped: Story = {
  args: { phase: { state: "ended", code: null }, children: body("^C") },
};
