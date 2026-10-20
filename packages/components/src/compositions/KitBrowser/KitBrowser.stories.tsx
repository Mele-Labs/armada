import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, within } from "storybook/test";

import type { KitSetupRead } from "../KitSetup/KitSetup";
import { KitBrowser } from "./KitBrowser";

const HOME = "/Users/user/.agent";

const SETUP: KitSetupRead = {
  harness: "Agent",
  home: HOME,
  present: true,
  kinds: [
    {
      kind: "skills",
      read: {
        what: "read",
        unreadable: [],
        items: [
          { name: "humanizer", says: "Rewrite AI-sounding text so it reads like the writer without changing what it says.", source: `${HOME}/skills/humanizer` },
          { name: "frontend-design", says: "Guidance for distinctive, intentional visual design.", source: `${HOME}/plugins/cache/frontend-design/skills/frontend-design`, plugin: "frontend-design" },
          { name: "skill-creator", says: "Create new skills and measure how they perform.", source: `${HOME}/plugins/cache/skill-creator/skills/skill-creator`, plugin: "skill-creator" },
        ],
      },
    },
    { kind: "commands", read: { what: "not_read", why: "The commands folder could not be opened" } },
  ],
};

const meta: Meta<typeof KitBrowser> = {
  title: "Compositions/Kit browser",
  component: KitBrowser,
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "100vh", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: { kind: "skills", label: "Skills", setup: SETUP, keep: "story-kit-browser", narrow: "", picked: undefined, onPick: () => {}, onJump: () => {} },
  render: (args) => {
    const [picked, onPick] = useState<string | undefined>(args.picked);
    return <KitBrowser {...args} picked={picked} onPick={onPick} />;
  },
};
export default meta;

type Story = StoryObj<typeof KitBrowser>;

/** The list is the kind and the right-hand column is one item, the first while none is picked. */
export const AKindRead: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: /^humanizer/ })).toBeVisible();
  },
};

/** A kind that was not read says why. It is never drawn as empty. */
export const AKindNotRead: Story = { args: { kind: "commands", label: "Commands" } };

/** Before the read lands, nothing is drawn. */
export const BeforeTheRead: Story = { args: { setup: undefined } };
