import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";

import type { KitSetupRead } from "../KitSetup/KitSetup";
import { SharedHomeProvider } from "../KitSetup/SharedHome";
import type { SharedHome, HomeSyncItem } from "../KitSetup/SharedHome";
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
    {
      kind: "plugins",
      read: {
        what: "read",
        unreadable: [],
        items: [
          { name: "impeccable@impeccable", says: "version 4.7.0", source: `${HOME}/plugins/cache/impeccable/impeccable/4.7.0` },
          { name: "github@official", says: "version 1.0.0", source: `${HOME}/plugins/cache/official/github/1.0.0` },
        ],
      },
    },
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

const setKind = fn();
const setItem = fn();

/** A home where What syncs has Skills on and every plugin off, which `setKind` and `setItem` switch. */
function WithHome({ children }: { children: React.ReactNode }) {
  const [kinds, setKinds] = useState<readonly string[]>(["skills"]);
  const [items, setItems] = useState<readonly HomeSyncItem[]>([]);
  const home: SharedHome = {
    account: "nickmele",
    repository: "armada-home",
    rows: items.map((one) => ({ ...one, state: "here" as const })),
    syncing: false,
    syncs: { kinds, items },
    sync: () => {},
    add: () => {},
    bring: () => {},
    choose: () => {},
    setKind: (kind, on) => {
      setKind(kind, on);
      setKinds((was) => (on ? [...was, kind] : was.filter((one) => one !== kind)));
    },
    setItem: (item, on) => {
      setItem(item, on);
      setItems((was) => (on ? [...was, item] : was.filter((one) => one.name !== item.name)));
    },
  };
  return <SharedHomeProvider value={home}>{children}</SharedHomeProvider>;
}

/** The open kind's switch sits beside Sync, and turning it on sends that kind. */
export const SyncAllOfAKind: Story = {
  args: { kind: "plugins", label: "Plugins" },
  decorators: [(Story) => <WithHome><Story /></WithHome>],
  play: async ({ canvasElement }) => {
    const page = within(canvasElement);
    const all = page.getByRole("switch", { name: "Sync all" });
    await expect(all).not.toBeChecked();
    await userEvent.click(all);
    await expect(setKind).toHaveBeenCalledWith("plugins", true);
    await expect(all).toBeChecked();
  },
};

/** One plugin is switched on by itself from its detail, by the name the home knows it by; while its kind is on the switch is on and fixed. */
export const SyncOneItem: Story = {
  args: { kind: "plugins", label: "Plugins" },
  decorators: [(Story) => <WithHome><Story /></WithHome>],
  play: async ({ canvasElement }) => {
    const page = within(canvasElement);
    const one = page.getByRole("switch", { name: "Sync to home" });
    await expect(one).not.toBeChecked();
    await userEvent.click(one);
    await expect(setItem).toHaveBeenCalledWith({ kind: "plugins", name: "impeccable@impeccable" }, true);
    await expect(page.getByRole("img", { name: "Only on this machine" })).toBeVisible();
    await userEvent.click(page.getByRole("switch", { name: "Sync all" }));
    await expect(one).toBeChecked();
    await expect(one).toBeDisabled();
  },
};
