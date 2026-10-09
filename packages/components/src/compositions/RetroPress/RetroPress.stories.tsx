import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { RetroPress } from "./RetroPress";

/** The press that writes a Session's retro. An icon, named by its tooltip. */
const meta: Meta<typeof RetroPress> = {
  title: "Compositions/Retro press",
  component: RetroPress,
  args: { onPress: fn() },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof RetroPress>;

export const AtRest: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Retro" }));
    await expect(args.onPress).toHaveBeenCalledTimes(1);
  },
};

/** The mark moves while the retro is written, and the press holds. */
export const Writing: Story = {
  args: { writing: true },
  play: async ({ canvas, args }) => {
    const press = canvas.getByRole("button", { name: "Writing the retro" });
    await expect(press).toBeDisabled();
    await expect(press).toHaveAttribute("data-writing", "true");
    await expect(args.onPress).not.toHaveBeenCalled();
  },
};
