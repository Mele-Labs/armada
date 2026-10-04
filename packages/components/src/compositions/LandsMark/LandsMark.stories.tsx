import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { LandsMark } from "./LandsMark";

/** Where a retro item's fix lands: Armada, the Kit, or the Manifest's repository. */
const meta: Meta<typeof LandsMark> = {
  title: "Compositions/Lands mark",
  component: LandsMark,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LandsMark>;

/** A bare glyph, named by its tooltip and by nothing drawn. */
export const Armada: Story = {
  name: "A retro fix that lands in Armada",
  args: { lands: "armada" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Lands in Armada" })).toBeVisible();
  },
};

export const Kit: Story = { name: "A retro fix that lands in Kit", args: { lands: "kit" } };

export const Manifest: Story = { name: "A retro fix that lands in the Manifest", args: { lands: "manifest" } };
