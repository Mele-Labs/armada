import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, userEvent, within } from "storybook/test";

import { SystemPage, SystemSidebar } from "./SystemSidebar";
import type { SystemGroup } from "./SystemSidebar";

const GROUPS: SystemGroup[] = [
  { id: "kit", label: "Kit", entries: [{ id: "skills", label: "Skills" }, { id: "plugins", label: "Plugins" }, { id: "models", label: "Models" }] },
  { id: "fleet", label: "Fleet", entries: [{ id: "limits", label: "Limits", alert: "Weekly limit nearing" }, { id: "checks", label: "Checks" }] },
  { id: "bridge", label: "Bridge", entries: [{ id: "theme", label: "Theme" }] },
];

const meta: Meta<typeof SystemSidebar> = {
  title: "Compositions/System sidebar",
  component: SystemSidebar,
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "100vh", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: { groups: GROUPS, value: "skills", query: "", onValue: () => {}, onQuery: () => {} },
  render: (args) => {
    const [value, onValue] = useState(args.value);
    const [query, onQuery] = useState(args.query);
    const kept = GROUPS.map((group) => ({ ...group, entries: group.entries.filter((one) => one.label.toLowerCase().includes(query.toLowerCase())) }));
    return (
      <SystemPage side={<SystemSidebar groups={kept} value={value} onValue={onValue} query={query} onQuery={onQuery} />}>
        <p>{value}</p>
      </SystemPage>
    );
  },
};
export default meta;

type Story = StoryObj<typeof SystemSidebar>;

export const ThreeCaptions: Story = {
  play: async ({ canvasElement }) => {
    const side = within(canvasElement).getByRole("navigation", { name: "System" });
    await userEvent.click(within(side).getByRole("button", { name: /^Models/ }));
    await expect(within(side).getByRole("button", { name: /^Models/ })).toHaveAttribute("aria-current", "page");
  },
};

export const Searched: Story = { args: { query: "s" } };
