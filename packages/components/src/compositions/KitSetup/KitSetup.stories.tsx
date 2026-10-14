import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { KitSetup, type KitSetupRead } from "./KitSetup";
import { SharedHomeProvider, type SharedHome, type SharedRow } from "./SharedHome";

/**
 * The setup a person already works with, drawn beside what Armada itself holds
 * — #1491. A reading and not a control: nothing here reaches a drone, and
 * nothing here can be switched on.
 */
const meta: Meta<typeof KitSetup> = {
  title: "Compositions/Kit setup",
  component: KitSetup,
};
export default meta;

type Story = StoryObj<typeof KitSetup>;

const FULL: KitSetupRead = {
  harness: "An agent CLI",
  home: "/Users/user/.agent",
  present: true,
  kinds: [
    {
      kind: "skills",
      read: {
        what: "read",
        items: [
          {
            name: "humanizer",
            says: "Rewrite AI-sounding text so it reads like the writer without changing what it says.",
            source: "/Users/user/.agent/skills/humanizer",
          },
          {
            name: "impact-analysis",
            says: "Use when the user wants to know what will break if they change something.",
            source: "/Users/user/.agent/skills/impact-analysis",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "plugins",
      read: {
        what: "read",
        items: [
          {
            name: "code-simplifier@official",
            says: "version 1.0.0",
            source: "/Users/user/.agent/plugins/cache/code-simplifier/1.0.0",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "agent_file",
      read: {
        what: "read",
        items: [
          {
            name: "AGENTS.md",
            says: "21 lines, opening # Global Instructions",
            source: "/Users/user/.agent/AGENTS.md",
          },
        ],
        unreadable: [],
      },
    },
    { kind: "sub_agents", read: { what: "read", items: [], unreadable: [] } },
    { kind: "commands", read: { what: "read", items: [], unreadable: [] } },
    {
      kind: "mcp_servers",
      read: {
        what: "read",
        items: [
          {
            name: "tracker",
            says: "tracker-server",
            source: "/Users/user/.agent.json",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "allowlist",
      read: { what: "not_read", why: "allowed-commands would not read: line 3 is not a command" },
    },
    {
      kind: "models",
      read: { what: "not_read", why: "which models a Job may use is resolved against a Manifest, and that is #41" },
    },
  ],
};

/** Nothing read yet. Not an empty setup — nobody has asked one — and **nothing is drawn**. */
export const Reading: Story = {
  name: "Reading",
  args: { setup: undefined },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.textContent).toBe("");
  },
};

/**
 * A setup with things in it. **The counts are the point**: this screen used to
 * open on *Nothing in your Kit yet* with all of this one directory away.
 */
export const WhatSomebodyHas: Story = {
  name: "What somebody has",
  args: { setup: FULL },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText("humanizer", { exact: true })).toBeVisible();
    // A kind nothing reads yet is named, never drawn as empty.
    await expect(kit.getAllByText(/Not read yet/)).toHaveLength(2);
    // The server is seen and is handed to nobody.
    await expect(kit.getByText(/handed none of them/)).toBeVisible();
  },
};

/** A file the person has and Armada cannot describe. Named, with the reason. */
export const SomethingWillNotRead: Story = {
  name: "Something will not read",
  args: {
    setup: {
      ...FULL,
      kinds: [
        {
          kind: "skills",
          read: {
            what: "read",
            items: [],
            unreadable: [
              {
                source: "/Users/user/.agent/skills/broken/SKILL.md",
                why: "front matter opens and never closes",
              },
            ],
          },
        },
      ],
    },
  },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText(/would not read/)).toBeVisible();
  },
};

/** A machine where the harness has never run: the head and where it looked, and **no sentence under it**. */
export const NothingThereYet: Story = {
  name: "Nothing there yet",
  args: { setup: { ...FULL, present: false, kinds: [] } },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByRole("heading", { name: "An agent CLI" })).toBeVisible();
    await expect(kit.queryByText(/Nothing is there yet/)).toBeNull();
  },
};

/**
 * The allowlist, read. **Each command says where it came from in words and has
 * a Remove**, with no count beside the list.
 */
export const AllowlistRead: Story = {
  name: "An allowlist that was read",
  args: {
    onRemoveAllowed: fn(),
    setup: {
      ...FULL,
      kinds: [
        {
          kind: "allowlist",
          read: {
            what: "read",
            items: [
              { name: "grep -n", source: "retro item 01M2LESSON3GREPASKED" },
              { name: "gh issue view", source: "always allow" },
              { name: "make check", source: "written by hand" },
            ],
            unreadable: [],
          },
        },
      ],
    },
  },
  play: async ({ canvasElement, args }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText("Retro item", { exact: true })).toBeVisible();
    await expect(kit.getByText("Always allow", { exact: true })).toBeVisible();
    await expect(kit.getByText("Written by hand", { exact: true })).toBeVisible();
    await expect(canvasElement.querySelector(".armada-kit-setup__count")).toBeNull();
    kit.getByRole("button", { name: "Remove gh issue view" }).click();
    await expect(args.onRemoveAllowed).toHaveBeenCalledWith("gh issue view");
  },
};

/** An allowlist that was read and holds nothing: its name, and **nothing under it**. */
export const AllowlistEmpty: Story = {
  name: "An allowlist that holds nothing",
  args: {
    setup: { ...FULL, kinds: [{ kind: "allowlist", read: { what: "read", items: [], unreadable: [] } }] },
  },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByRole("heading", { name: "Allowlist" })).toBeVisible();
    await expect(canvasElement.querySelector(".armada-kit-setup__none")).toBeNull();
    await expect(kit.queryByRole("listitem")).toBeNull();
  },
};

const ACTS = { choose: fn(), add: fn(), bring: fn(), sync: fn() };
const row = (kind: string, name: string, state: SharedRow["state"], more: Partial<SharedRow> = {}): SharedRow => ({ kind, name, state, ...more });
const HOME_ROWS: SharedRow[] = [
  row("skills", "humanizer", "same"),
  row("mcp_servers", "gitnexus", "same", { keyHere: true }),
  row("allowlist", "git log --oneline", "here"),
  row("allowlist", "gh run view", "home", { source: "always allow" }),
  row("global_instructions", "AGENTS.md", "differs", {
    says: "21 lines",
    source: "/Users/user/.agent/AGENTS.md",
    here: "# Global Instructions\nQuestions go last.",
    there: "# Global Instructions\nQuestions go last, in the prompt.",
  }),
];
const HOME: SharedHome = { account: "nickmele", repository: "armada-home", rows: HOME_ROWS, syncing: false, ...ACTS };

/**
 * **Each row says how it stands against the home in a glyph with a tooltip and no word on
 * screen.** The acts are icons too, named by their tooltip and the row. A row that differs is the
 * one mark that opens the two versions, and picking a side is the only way it settles.
 */
export const AgainstASharedHome: Story = {
  name: "Against a shared home",
  args: {
    setup: {
      ...FULL,
      kinds: [
        { kind: "skills", read: { what: "read", items: [{ name: "humanizer", source: "/Users/user/.agent/skills/humanizer" }], unreadable: [] } },
        // Global instructions is drawn beside the agent file, so the read has to carry one.
        FULL.kinds.find((one) => one.kind === "agent_file")!,
        { kind: "mcp_servers", read: { what: "read", items: [{ name: "gitnexus", says: "gitnexus-mcp", source: "/Users/user/.agent.json" }], unreadable: [] } },
        { kind: "allowlist", read: { what: "read", items: [{ name: "git log --oneline", source: "always allow" }], unreadable: [] } },
      ],
    },
  },
  decorators: [
    (Story) => (
      <SharedHomeProvider value={HOME}>
        <Story />
      </SharedHomeProvider>
    ),
  ],
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    await expect(kit.getByText("nickmele")).toBeVisible();
    await expect(within(kit.getByRole("region", { name: "Skills" })).getByRole("img", { name: "Same as the home" })).toBeVisible();
    await expect(kit.getByRole("img", { name: "Needs its key on this machine" })).toBeVisible();
    await expect(kit.getByRole("img", { name: "Only on this machine" })).toBeVisible();
    await expect(kit.getByRole("img", { name: "Only in the home" })).toBeVisible();

    // An act is an icon: named, and carrying no text.
    const up = kit.getByRole("button", { name: "Add to the home git log --oneline" });
    await expect(up.textContent).toBe("");
    await userEvent.click(up);
    await expect(ACTS.add).toHaveBeenCalledWith(expect.objectContaining({ name: "git log --oneline" }));
    await userEvent.click(kit.getByRole("button", { name: "Add to this machine gh run view" }));
    await expect(ACTS.bring).toHaveBeenCalledWith(expect.objectContaining({ name: "gh run view" }));
    await userEvent.click(kit.getByRole("button", { name: "Sync" }));
    await expect(ACTS.sync).toHaveBeenCalled();

    // A row that differs opens both versions, and a side is picked, never merged.
    await userEvent.click(kit.getByRole("button", { name: "Differs from the home, AGENTS.md" }));
    const sheet = within(await within(document.body).findByRole("dialog", { name: "AGENTS.md" }));
    await expect(sheet.getByRole("region", { name: "This machine" })).toBeVisible();
    await expect(sheet.getByRole("region", { name: "The home" })).toBeVisible();
    await userEvent.click(sheet.getByRole("button", { name: /^Use the home.s$/ }));
    await expect(ACTS.choose).toHaveBeenCalledWith(expect.objectContaining({ name: "AGENTS.md" }), "there");
    await waitFor(() => expect(within(document.body).queryByRole("dialog", { name: "AGENTS.md" })).toBeNull());
  },
};

const MANY = Array.from({ length: 50 }, (_, at) => {
  const name = `skill-${String(at + 1).padStart(2, "0")}`;
  return { name, source: `/Users/user/.agent/skills/${name}` };
});

/**
 * **A long list scrolls inside its tile and the tile keeps its place.** The list is a Tab stop
 * only while it has more than it shows; a tile whose rows fit has no second stop in it.
 */
export const ALongListScrollsInsideItsTile: Story = {
  name: "A long list scrolls inside its tile",
  args: {
    setup: {
      ...FULL,
      kinds: [
        { kind: "skills", read: { what: "read", items: MANY, unreadable: [] } },
        { kind: "plugins", read: { what: "read", items: [{ name: "code-simplifier@official", source: "/Users/user/.agent/plugins/cache" }], unreadable: [] } },
      ],
    },
  },
  play: async ({ canvasElement }) => {
    const kit = within(canvasElement);
    const skills = kit.getByRole("region", { name: "Skills" });
    skills.focus();
    await userEvent.tab();
    const scroller = document.activeElement as HTMLElement;
    await expect(skills.contains(scroller) && scroller !== skills).toBe(true);
    await expect(scroller.scrollHeight).toBeGreaterThan(scroller.clientHeight);

    const plugins = kit.getByRole("region", { name: "Plugins" });
    plugins.focus();
    await userEvent.tab();
    await expect(plugins.contains(document.activeElement)).toBe(false);
  },
};
