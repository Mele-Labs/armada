import type { Meta, StoryObj } from "@storybook/react-vite";
import { SelectMark } from "./SelectMark";

const meta: Meta<typeof SelectMark> = {
  title: "Primitives/Select mark",
  component: SelectMark,
};
export default meta;

type Story = StoryObj<typeof SelectMark>;

const row = { display: "flex", gap: "var(--space-3)", padding: "var(--space-3)", borderRadius: "var(--radius-md)", background: "var(--bg-raised)" } as const;

/** Four corners in `--fg-subtle`; selected, they turn the accent and close on a lit core. */
export const Default: Story = {
  render: () => (
    <div style={row}>
      <SelectMark checked={false} label="Select Sign the build" onToggle={() => undefined} />
      <SelectMark checked label="Select Pick the app icon" onToggle={() => undefined} />
    </div>
  ),
};
