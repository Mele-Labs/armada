import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, userEvent, within } from "storybook/test";

import { KitModels } from "./KitModels";
import type { KitModelChoices, KitModelSlot, KitModelsRead } from "./KitModels";

const LEVELS = ["low", "medium", "high", "xhigh", "max"];

const MODELS = [
  { name: "opus", display: "Opus", resolved: "opus-5-5", says: "For complex work and everyday tasks", effort_levels: LEVELS, on: true },
  { name: "sonnet", display: "Sonnet", resolved: "sonnet-5-5", says: "Most efficient for simpler tasks", effort_levels: LEVELS, on: true },
  { name: "haiku", display: "Haiku", resolved: "haiku-5-5", says: "Fastest for quick answers", effort_levels: LEVELS, on: false },
];

const CHOICES: KitModelChoices = {
  jobs: "opus",
  drones: "sonnet",
  reviews: "opus",
  effort: "high",
  judge: "sonnet",
  judgeEffort: "harness default",
  proposer: "sonnet",
  proposerEffort: "harness default",
  retro: "sonnet",
  secondOpinion: "opus",
  downshift: "sonnet",
  helmEffort: "high",
};

const meta: Meta<typeof KitModels> = {
  title: "Compositions/Kit models",
  component: KitModels,
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "100vh", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: { models: { read: "read", models: MODELS, choices: CHOICES }, keep: "story-kit-models", picked: undefined, onPick: () => {}, onOn: () => {}, onChoose: () => {} },
  render: (args) => {
    const [held, setHeld] = useState<KitModelsRead | undefined>(args.models);
    const choose = (slot: KitModelSlot, value: string) => setHeld((now) => (now?.read === "read" ? { ...now, choices: { ...now.choices, [slot]: value } } : now));
    return <KitModels {...args} models={held} onChoose={choose} />;
  },
};
export default meta;

type Story = StoryObj<typeof KitModels>;

export const EveryRun: Story = {
  play: async ({ canvasElement }) => {
    const page = within(canvasElement);
    for (const name of ["Jobs", "Drones", "Reviews", "Judge", "Proposer", "Retro", "Second opinion", "Step down to"]) await expect(page.getByRole("combobox", { name })).toBeVisible();
    await userEvent.selectOptions(page.getByRole("combobox", { name: "Judge" }), "opus");
    await expect(page.getByRole("combobox", { name: "Judge" })).toHaveValue("opus");
  },
};

export const NotRead: Story = { args: { models: { read: "not_read", why: "The agent did not answer in time" } } };

export const BeforeTheRead: Story = { args: { models: undefined } };
