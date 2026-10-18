import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { SessionRepository } from "./SessionRepository";

/**
 * The repository a Session works in, in its head. Pressed, it lists the others; a pick shows what stays
 * behind as a caution; Move acts. With no other repository, or once moved, the name is text.
 */
const meta: Meta<typeof SessionRepository> = {
  title: "Compositions/Session repository",
  component: SessionRepository,
  args: {
    name: "shop",
    to: [
      { id: "armada", name: "armada" },
      { id: "billing", name: "billing" },
    ],
    stays: { branch: "fix/pin-store-clock" },
    ends: true,
    onMove: fn(async () => undefined),
  },
  decorators: [(Story) => <div style={{ minHeight: 220 }}><Story /></div>],
};
export default meta;

type Story = StoryObj<typeof SessionRepository>;

export const Menu: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Repository shop" }));
    await expect(canvas.getByRole("menuitem", { name: "armada" })).toBeVisible();
    await expect(canvas.getByRole("menuitem", { name: "billing" })).toBeVisible();
    await expect(canvas.queryByRole("menuitem", { name: "shop" })).toBeNull();
  },
};

export const Confirm: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Repository shop" }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "armada" }));
    await expect(canvas.getByText(/and its files stay in shop/)).toBeVisible();
    await expect(canvas.getByText("fix/pin-store-clock")).toBeVisible();
    await expect(canvas.getByText("Ends this Job")).toBeVisible();
    // The facts are a caution, with the warning glyph.
    await expect(canvas.getByRole("status")).toHaveClass("armada-alert--caution");
    await userEvent.click(canvas.getByRole("button", { name: "Move to armada" }));
    await waitFor(() => expect(args.onMove).toHaveBeenCalledWith("armada"));
  },
};

export const ConfirmWithoutABranch: Story = {
  args: { stays: {}, ends: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Repository shop" }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "billing" }));
    await expect(canvas.getByText("Its files stay in shop")).toBeVisible();
    await expect(canvas.queryByText("Ends this Job")).toBeNull();
  },
};

export const InATerminal: Story = {
  args: { held: "Runs in a terminal that is still listening" },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Repository shop" }));
    const item = canvas.getByRole("menuitem", { name: "armada" });
    await expect(item).toHaveAttribute("aria-disabled", "true");
    await expect(canvas.getAllByText("Runs in a terminal that is still listening").length).toBeGreaterThan(0);
    // A press on one drawn off does nothing: no confirm, no move. Its pointer events are off, so the click is sent to it.
    item.click();
    await expect(canvas.queryByRole("button", { name: "Move to armada" })).toBeNull();
    await expect(args.onMove).not.toHaveBeenCalled();
  },
};

export const OneRepository: Story = {
  args: { to: [] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "Repository shop" })).toBeVisible();
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};
