import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { TerminalPane, type TerminalPaneProps } from "./TerminalPane";
import { XtermScreen } from "./XtermScreen";

/**
 * A command running in a terminal of its own. The frame is the component and the body is whatever the
 * caller draws: here a plain well, and below it the terminal screen the app draws, which is xterm. One
 * story per state of the run.
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

/** What the app puts in the body: xterm, which takes the focus as it opens and sends what is typed to `write`. */
export const OnTheTerminalScreen: Story = {
  args: { phase: { state: "running" } },
  render: (args) => {
    const write = fn();
    const onSize = fn();
    return (
      <TerminalPane {...args}>
        <XtermScreen
          out={"\u001b[32m$\u001b[39m make\r\nBuilding \u001b[36mstore\u001b[39m\r\n\u001b[31merror\u001b[39m: nothing to do\r\n"}
          run={1}
          running
          write={write}
          onSize={onSize}
        />
      </TerminalPane>
    );
  },
  play: async ({ canvasElement, userEvent }) => {
    const pane = within(canvasElement);
    await expect(await pane.findByText(/nothing to do/)).toBeVisible();
    await expect(await pane.findByRole("textbox", { name: "Terminal input" })).toHaveFocus();
    await userEvent.keyboard("a");
  },
};
