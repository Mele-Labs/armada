import type { Meta, StoryObj } from "@storybook/react-vite";
import { SelectMark } from "./SelectMark";

const meta: Meta<typeof SelectMark> = {
  title: "Primitives/Select mark",
  component: SelectMark,
};
export default meta;

type Story = StoryObj<typeof SelectMark>;

const row = { display: "flex", gap: "var(--space-3)", padding: "var(--space-3)", borderRadius: "var(--radius-md)", background: "var(--bg-raised)" } as const;

/** Reticle: four corners in `--fg-subtle`; selected, they turn the accent and close on a lit core. */
export const Reticle: Story = {
  render: () => (
    <div style={row}>
      <SelectMark look="reticle" checked={false} label="Select Sign the build" onToggle={() => undefined} />
      <SelectMark look="reticle" checked label="Select Pick the app icon" onToggle={() => undefined} />
    </div>
  ),
};

/** Plate: the row's place as a mono plate on `--bg-sunken`; selected, it fills with the accent. */
export const Plate: Story = {
  render: () => (
    <div style={row}>
      <SelectMark look="plate" index={3} checked={false} label="Select Sign the build" onToggle={() => undefined} />
      <SelectMark look="plate" index={4} checked label="Select Pick the app icon" onToggle={() => undefined} />
    </div>
  ),
};
