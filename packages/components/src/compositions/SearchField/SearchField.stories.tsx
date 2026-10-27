import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { SearchField } from "./SearchField";

/** The one search bar: empty, and holding a word. */
const meta: Meta<typeof SearchField> = {
  title: "Compositions/SearchField",
  component: SearchField,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-settings-index)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SearchField>;

function Held({ start }: { start: string }) {
  const [value, setValue] = useState(start);
  return <SearchField label="Search the list" value={value} onValue={setValue} />;
}

export const Empty: Story = { render: () => <Held start="" /> };

export const Holding: Story = { render: () => <Held start="deploy" /> };
