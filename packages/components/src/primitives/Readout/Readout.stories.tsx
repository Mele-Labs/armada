import type { Meta, StoryObj } from "@storybook/react-vite";
import { Readout } from "./Readout";

/** A figure or an id the system reports, in a well at the right of its row. */
const meta: Meta<typeof Readout> = {
  title: "Primitives/Readout",
  component: Readout,
};
export default meta;

type Story = StoryObj<typeof Readout>;

export const Default: Story = {
  args: { children: "#1899" },
};

/** A column of them in a list: one width, the figures on their right edge, and the column kept where a row has none. */
export const FixedColumn: StoryObj = {
  render: () => (
    <div style={{ display: "grid", gap: "var(--space-1)", justifyItems: "end", width: "var(--space-12)" }}>
      <Readout fixed>13:50</Readout>
      <Readout fixed>#1899</Readout>
      <Readout fixed>{""}</Readout>
      <Readout fixed>a3f29c1</Readout>
    </div>
  ),
};
