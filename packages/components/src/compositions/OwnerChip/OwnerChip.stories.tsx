import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ChipOwnership, OwnerChip, type OwnerSummary } from "./OwnerChip";

/**
 * A chip that names a branch, pull request, slot or Job, and the card of the
 * Session that owns it. Hovering draws the card; pressing keeps it up.
 */
const meta: Meta<typeof OwnerChip> = {
  title: "Compositions/Owner chip",
  component: OwnerChip,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-12)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof OwnerChip>;

const OWNER: OwnerSummary = {
  id: "s7",
  title: "Flaky store test",
  hue: "completed-failed",
  slots: [3],
  pullRequests: [{ number: 1843, checks: "failed", said: "Checks failed: store: 2 failed" }],
  jobs: [{ id: "J52", number: 52 }],
  lastTurn: "14:09",
};

const open = fn();

/** Owned: pressing keeps the card up, and its press opens the Session. */
export const Owned: Story = {
  args: { chip: { kind: "branch", name: "fix/52-pin-store-clock" }, children: <span>fix/52-pin-store-clock</span> },
  decorators: [
    (Story) => (
      <ChipOwnership.Provider value={{ ownerOf: () => OWNER, open }}>
        <Story />
      </ChipOwnership.Provider>
    ),
  ],
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Branch fix/52-pin-store-clock" }));
    // The card is drawn in the body, outside the story's own root.
    await userEvent.click(await within(document.body).findByRole("button", { name: "Open Session" }));
    await expect(open).toHaveBeenCalledWith("s7");
  },
};

/** Nobody owns it: the chip draws as it did, and no button is added. */
export const Unowned: Story = {
  args: { chip: { kind: "branch", name: "job/44-cap-log-reader" }, children: <span>job/44-cap-log-reader</span> },
  decorators: [
    (Story) => (
      <ChipOwnership.Provider value={{ ownerOf: () => undefined, open }}>
        <Story />
      </ChipOwnership.Provider>
    ),
  ],
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};
