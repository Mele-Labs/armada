import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, waitFor } from "storybook/test";

import { RunLinkLine, RunPinned } from "./RunLinkLine";
import { FOUND_IN_LOG, type RunLink } from "../RunControl/runs";

/**
 * A found link between two run processes: which expects which on what port, where Fleet found it, and the two ways out of it.
 * Neither act is needed to run.
 */
const meta: Meta<typeof RunLinkLine> = {
  title: "Compositions/Run link line",
  component: RunLinkLine,
};
export default meta;

type Story = StoryObj<typeof RunLinkLine>;

const LINK: RunLink = { from: "frontend", to: "backend", port: 8731, found: "frontend/vite.config.js" };

/**
 * **The fact, where it was found, and Fix with a Job and Pin the port.** The mark says on hover that the other process runs on a leased
 * port, and Pin the port says on hover what pinning does. Each press answers its own callback once.
 */
export const Found: Story = {
  args: { link: LINK, from: "frontend", to: "backend", onFix: fn(), onPin: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("group", { name: "frontend expects backend" })).toHaveTextContent("frontend expects backend on 8731");
    await expect(canvas.getByText("frontend/vite.config.js")).toBeVisible();
    await expect(canvas.getByRole("img", { name: "backend runs on a leased port, not 8731" })).toBeVisible();

    await userEvent.hover(canvas.getByRole("button", { name: "Pin the port" }));
    await waitFor(() => expect(screen.getByText("Always start backend on 8731 in this repository. A Job's bay can't run it at the same time")).toBeVisible());

    await userEvent.click(canvas.getByRole("button", { name: "Fix with a Job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Pin the port" }));
    await expect(args.onFix).toHaveBeenCalledTimes(1);
    await expect(args.onPin).toHaveBeenCalledTimes(1);
  },
};

/** **A pinned port offers Unpin, and a row above carrying the mark leaves this line without one.** A link seen in a log says `log`. */
export const PinnedUnmarked: Story = {
  args: { link: { ...LINK, found: FOUND_IN_LOG }, from: "frontend", to: "backend", pinned: true, marked: false, onFix: fn(), onPin: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Unpin the port" })).toBeVisible();
    await expect(canvas.queryByRole("img")).toBeNull();
    await expect(canvas.getByText("log")).toBeVisible();
  },
};

/** **A pinned process's mark: the pin and its number, a press undoing it.** Without `onUnpin` it is a fact and no button. */
export const Pinned: StoryObj<typeof RunPinned> = {
  render: (args) => <RunPinned {...args} />,
  args: { port: 8731, name: "backend", onUnpin: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Unpin backend" }));
    await expect(args.onUnpin).toHaveBeenCalledTimes(1);
  },
};
