import type { Meta, StoryObj } from "@storybook/react-vite";

import { RepositoryChip } from "./RepositoryChip";

/** A repository as a chip, where a surface names the repository a thing belongs to. */
const meta: Meta<typeof RepositoryChip> = {
  title: "Compositions/Repository chip",
  component: RepositoryChip,
  args: { name: "storefront" },
};
export default meta;

type Story = StoryObj<typeof RepositoryChip>;

export const Named: Story = {};

/** A long name clips, and the tooltip and the accessible name keep it whole. */
export const Long: Story = { args: { name: "payments-reconciliation-service" } };
