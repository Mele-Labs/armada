import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { PrOwnerRegistryProvider, RegisteredOwner, RegisteredOwnerOf, type PrOwnerRegistry } from "./RegisteredOwner";

/**
 * The owner a pull request was registered to: the kind's glyph and the short id, then the machine's
 * mark. A row holds nothing else, so nothing else is drawn.
 */
const meta: Meta<typeof RegisteredOwner> = {
  title: "Compositions/Registered owner",
  component: RegisteredOwner,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof RegisteredOwner>;

export const SessionHere: Story = {
  name: "A Session on this machine",
  args: { row: { kind: "session", id: "a3f9c21e", machine: { name: "MacBook Pro", here: true } } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Job a3f9c21e, This machine" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "This machine" })).toBeVisible();
  },
};

export const SessionElsewhere: Story = {
  name: "A Session on another machine",
  args: { row: { kind: "session", id: "7be40d15", machine: { name: "MacBook Air", here: false } } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Job 7be40d15, MacBook Air" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "MacBook Air" })).toBeVisible();
    await expect(canvas.queryByRole("img", { name: "This machine" })).toBeNull();
  },
};

const REGISTRY: PrOwnerRegistry = {
  ownerOf: (repository, number) =>
    repository === "mele-labs/armada" && number === 2209 ? { kind: "session", id: "a3f9c21e", machine: { name: "MacBook Pro", here: true } } : undefined,
  registering: true,
  setRegistering: () => {},
};

/** Read off the registry by repository and number: a pull request with no row draws nothing at all. */
export const NoRow: Story = {
  name: "A pull request with no row",
  render: () => (
    <PrOwnerRegistryProvider value={REGISTRY}>
      <RegisteredOwnerOf repository="mele-labs/armada" number={2209} />
      <RegisteredOwnerOf repository="mele-labs/armada" number={2218} />
    </PrOwnerRegistryProvider>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("group")).toHaveLength(1);
  },
};
