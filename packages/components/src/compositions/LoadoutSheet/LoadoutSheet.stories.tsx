import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, waitFor, within } from "storybook/test";

import { loadoutKeyOf, type LoadoutRef } from "../../loadout";
import { timeSaid } from "../../time-format";
import { LoadoutSheet, type LoadoutOpened, type LoadoutSheetProps } from "./LoadoutSheet";
import { BODIES, LOADOUT, MCP_SERVERS, OWN_SKILL, PLUGIN_SKILL, SKILL_ACTS, TURNS } from "./LoadoutSheet.fixtures";

/** A Session's whole Loadout, in the panel over the work area: what it knows, what it was told, and what each turn used. */
const meta: Meta<typeof LoadoutSheet> = {
  title: "Compositions/Loadout sheet",
  component: LoadoutSheet,
  args: { open: true, read: { state: "read", loadout: LOADOUT }, onClose: fn() },
};
export default meta;

type Story = StoryObj<typeof LoadoutSheet>;

const latest = `Turn at ${timeSaid(TURNS[1]!.started_at)}`;
const earlier = `Turn at ${timeSaid(TURNS[0]!.started_at)}`;

/**
 * The latest turn is open and the one before is closed; a press opens it in its place. The source filter narrows all
 * three sections, and a server that failed says so on its mark.
 */
export const Read: Story = {
  play: async ({ userEvent }) => {
    const sheet = await screen.findByRole("dialog", { name: "Loadout" });
    await expect(within(sheet).getByRole("button", { name: latest })).toHaveAttribute("aria-expanded", "true");
    const used = within(sheet).getByRole("list", { name: `Used in the turn at ${timeSaid(TURNS[1]!.started_at)}` });
    await expect(within(used).getByText("armada:mechanic").parentElement).toHaveTextContent("Armada · sonnet");
    await expect(within(used).getByText("mcp__forge__pull_request_read").parentElement).toHaveTextContent("forge · by armada:mechanic");
    await expect(within(used).getByText("mcp__forge__pull_request_read").parentElement).toHaveTextContent("× 40");
    await expect(within(used).getByText("× 40")).toHaveClass("armada-loadout__count");
    await expect(within(used).getByText("armada:mechanic").parentElement).not.toHaveTextContent("×");

    await userEvent.click(within(sheet).getByRole("button", { name: earlier }));
    await expect(within(sheet).getByRole("button", { name: earlier })).toHaveAttribute("aria-expanded", "true");
    await expect(within(sheet).getByRole("button", { name: latest })).toHaveAttribute("aria-expanded", "false");

    const servers = within(sheet).getByRole("region", { name: "MCP servers" });
    await expect(within(servers).getByRole("img", { name: "Failed" })).toBeInTheDocument();

    await userEvent.selectOptions(within(sheet).getByRole("combobox", { name: "Source" }), "Armada");
    const knows = within(sheet).getByRole("region", { name: "Knows" });
    await expect(within(knows).queryByText("work-issue")).not.toBeInTheDocument();
    await expect(within(knows).getAllByText("armada:mechanic").length).toBeGreaterThan(0);
    await expect(within(sheet).getByRole("region", { name: "Told" })).toHaveTextContent("Armada job rules");
    await expect(within(sheet).getByRole("region", { name: "Told" })).not.toHaveTextContent("Explanatory");
  },
};

/** While it is read, the latest turn the Session's record already holds is drawn, and nothing stands in for the rest. */
export const Reading: Story = {
  args: { read: { state: "reading", latest: TURNS[1]! } },
  play: async () => {
    const sheet = await screen.findByRole("dialog", { name: "Loadout" });
    await expect(within(sheet).getByRole("button", { name: latest })).toHaveAttribute("aria-expanded", "true");
    await expect(within(sheet).queryByRole("region", { name: "Knows" })).not.toBeInTheDocument();
  },
};

/** Fleet did not answer: the latest turn still, and what to do. */
export const Failed: Story = { args: { read: { state: "failed", latest: TURNS[1]! } } };

/** A Session that reported an empty Loadout. */
export const Empty: Story = { args: { read: { state: "read", loadout: { items: [], turns: [] } } } };

/** The host's side of a press: the item opens at once and its read lands, an MCP server with no read at all. */
function Opening(args: LoadoutSheetProps) {
  const [opened, setOpened] = useState<LoadoutOpened | undefined>(args.opened);
  const open = (item: LoadoutRef | null) => {
    args.onOpenItem?.(item);
    if (item === null) return setOpened(undefined);
    if (item.kind === "mcp_server") return setOpened({ item });
    setOpened({ item, read: BODIES[loadoutKeyOf(item)] ?? { state: "not_read", why: "Built into the harness. There is no file to read." } });
  };
  return <LoadoutSheet {...args} {...(opened === undefined ? {} : { opened })} onOpenItem={open} />;
}

/**
 * Every row is a button. A press opens the item to the left, the row marked as the one open, and the same item in a
 * turn's uses marked with it; another press replaces it. `Esc` closes that panel and leaves the Loadout open.
 */
export const OpensAnItem: Story = {
  args: { onOpenItem: fn() },
  render: (args) => <Opening {...args} />,
  play: async ({ args, userEvent }) => {
    const sheet = await screen.findByRole("dialog", { name: "Loadout" });
    const knows = within(sheet).getByRole("region", { name: "Knows" });
    const skill = within(within(knows).getByRole("region", { name: "Skills" })).getByRole("button", { name: /^work-issue/ });
    await userEvent.click(skill);
    const panel = await screen.findByRole("dialog", { name: "work-issue" });
    await expect(panel).toHaveTextContent(".harness/skills/work-issue/SKILL.md");
    await expect(within(panel).getByText("Work one issue")).toBeInTheDocument();
    await expect(skill).toHaveAttribute("aria-current", "true");

    await userEvent.click(within(within(sheet).getByRole("region", { name: "Told" })).getByRole("button", { name: /Armada job rules/ }));
    const rules = await screen.findByRole("dialog", { name: "Armada job rules" });
    await expect(screen.queryByRole("dialog", { name: "work-issue" })).not.toBeInTheDocument();
    await expect(rules).toHaveTextContent("Only the start is shown.");
    await expect(skill).not.toHaveAttribute("aria-current");

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Armada job rules" })).not.toBeInTheDocument());
    await expect(screen.getByRole("dialog", { name: "Loadout" })).toBeInTheDocument();
    await expect(args.onOpenItem).toHaveBeenLastCalledWith(null);
    await expect(args.onClose).not.toHaveBeenCalled();
  },
};

/** An MCP server reads nothing: its panel lists the tools the Loadout gave it, each with its state. */
export const OpensAServer: Story = {
  args: { onOpenItem: fn(), opened: { item: { kind: "mcp_server", name: "forge", source: "mcp", from: "forge" } } },
  render: (args) => <Opening {...args} />,
  play: async () => {
    const panel = await screen.findByRole("dialog", { name: "forge" });
    const tools = within(panel).getByRole("region", { name: "Tools" });
    await expect(within(tools).getByText("mcp__forge__search_code").parentElement).toHaveTextContent("Deferred");
    await expect(within(tools).queryByText("Read")).not.toBeInTheDocument();
  },
};

/** A built in tool: Fleet says why there is nothing to read, and the panel says it plainly. */
export const NotRead: Story = {
  args: { onOpenItem: fn(), opened: { item: { kind: "tool", name: "Read", source: "built_in" }, read: { state: "not_read", why: "Built into the harness. There is no file to read." } } },
  render: (args) => <Opening {...args} />,
};

/** While the read is out, the name and source are drawn and only the text stands in. */
export const ItemReading: Story = {
  args: { onOpenItem: fn(), opened: { item: { kind: "subagent", name: "armada:mechanic", source: "armada" }, read: { state: "reading" } } },
};

/** The row a live server is drawn on, found by the press that opens it. */
const serverRow = (servers: HTMLElement, name: string): HTMLElement => within(servers).getByRole("button", { name: new RegExp(`^${name.replace(/[()]/g, "\\$&")}`) }).closest("li")!;

/** The acts a row's menu holds, by name, once its caret is pressed. */
async function menuOf(row: HTMLElement, name: string, userEvent: { click: (element: Element) => Promise<void> }): Promise<string[]> {
  await userEvent.click(within(row).getByRole("button", { name: `More for ${name}` }));
  const menu = within(row).getByRole("menu", { name: `More for ${name}` });
  return within(menu).getAllByRole("menuitem").map((one) => one.textContent ?? "");
}

/**
 * A hosted Job's servers read live, one in each state: a mark and the CLI's words where it is not connected, its
 * state's act on the face and the others its state takes in the menu. Connected draws no mark and offers Turn off,
 * with Reconnect behind it; off offers Turn on alone; connecting offers nothing.
 */
export const LiveServers: Story = {
  args: { onOpenItem: fn(), mcp: { servers: MCP_SERVERS, onAct: fn() } },
  play: async ({ args, userEvent }) => {
    const servers = within(await screen.findByRole("dialog", { name: "Loadout" })).getByRole("region", { name: "MCP servers" });
    const fleet = serverRow(servers, "armada-fleet");
    await expect(within(fleet).queryByRole("img")).not.toBeInTheDocument();
    await expect(fleet).toHaveTextContent("71 tools");
    await expect(within(fleet).getByRole("button", { name: "Turn off armada-fleet" })).toBeInTheDocument();
    await expect(await menuOf(fleet, "armada-fleet", userEvent)).toEqual(["Reconnect"]);
    await userEvent.click(within(fleet).getByRole("menuitem", { name: "Reconnect" }));
    await expect(args.mcp!.onAct).toHaveBeenCalledWith("armada-fleet", "reconnect");

    const devtools = serverRow(servers, "chrome-devtools");
    await expect(within(devtools).getByRole("img", { name: "Failed" })).toBeInTheDocument();
    await expect(devtools).toHaveTextContent("timed out after 30s");
    await userEvent.click(within(devtools).getByRole("button", { name: "Reconnect chrome-devtools" }));
    await expect(args.mcp!.onAct).toHaveBeenCalledWith("chrome-devtools", "reconnect");
    await expect(await menuOf(devtools, "chrome-devtools", userEvent)).toEqual(["Turn off"]);

    const rovo = serverRow(servers, "Atlassian Rovo (2)");
    await expect(within(rovo).getByRole("img", { name: "Connecting" })).toBeInTheDocument();
    await expect(within(rovo).getAllByRole("button")).toHaveLength(1);

    const mailroom = serverRow(servers, "mailroom");
    await expect(within(mailroom).getByRole("img", { name: "Needs sign-in" })).toBeInTheDocument();
    await expect(within(mailroom).getByRole("button", { name: "Sign in mailroom" })).toBeInTheDocument();
    await expect(await menuOf(mailroom, "mailroom", userEvent)).toEqual(["Reconnect", "Turn off"]);

    const ledger = serverRow(servers, "ledger");
    await expect(within(ledger).getByRole("img", { name: "Off" })).toBeInTheDocument();
    await expect(within(ledger).getByRole("button", { name: "Turn on ledger" })).toBeInTheDocument();
    await expect(within(ledger).queryByRole("button", { name: "More for ledger" })).not.toBeInTheDocument();
  },
};

/**
 * An act out shows on its row's face, the one from the menu too, and takes no second press. A refusal is Fleet's words
 * on its row. A server whose sign-in page was opened offers Reconnect, with Sign in behind it again, since a connector
 * page sends nothing back.
 */
export const LiveActs: Story = {
  args: {
    onOpenItem: fn(),
    mcp: {
      servers: [
        { name: "chrome-devtools", state: "failed", error: "timed out after 30s", tools: 0, acting: "reconnect" },
        { name: "forge", state: "failed", error: "timed out after 30s", tools: 0, acting: "turn_off" },
        { name: "mailroom", state: "needs_sign_in", tools: 0, signedIn: true },
        { name: "ledger", state: "off", tools: 0, refused: "The Job's process did not answer." },
      ],
      onAct: fn(),
    },
  },
  play: async ({ userEvent }) => {
    const servers = within(await screen.findByRole("dialog", { name: "Loadout" })).getByRole("region", { name: "MCP servers" });
    const reconnecting = within(serverRow(servers, "chrome-devtools")).getByRole("button", { name: "Reconnecting chrome-devtools" });
    await expect(reconnecting).toHaveAttribute("aria-busy", "true");
    await expect(reconnecting).toHaveAttribute("aria-disabled", "true");
    await expect(within(serverRow(servers, "chrome-devtools")).getByRole("button", { name: "More for chrome-devtools" })).toBeDisabled();
    const turningOff = within(serverRow(servers, "forge")).getByRole("button", { name: "Turning off forge" });
    await expect(turningOff).toHaveAttribute("aria-busy", "true");
    const mailroom = serverRow(servers, "mailroom");
    await expect(within(mailroom).getByRole("button", { name: "Reconnect mailroom" })).toBeInTheDocument();
    await expect(await menuOf(mailroom, "mailroom", userEvent)).toEqual(["Sign in", "Turn off"]);
    await expect(serverRow(servers, "ledger")).toHaveTextContent("The Job's process did not answer.");
  },
};

/** The row a skill is drawn on in Knows, found by the press that opens it. */
const skillRow = (name: string): HTMLElement => {
  const knows = within(screen.getByRole("dialog", { name: "Loadout" })).getByRole("region", { name: "Knows" });
  return within(within(knows).getByRole("region", { name: "Skills" })).getByRole("button", { name: new RegExp(`^${name}`) }).closest("li")!;
};

/**
 * A skill Kit's inventory names offers Edit on its face, for its own SKILL.md in the file panel: one of the person's
 * own and one a plugin brought. A repository's skill offers none.
 */
export const SkillEdit: Story = {
  args: { onOpenItem: fn(), skills: { rows: SKILL_ACTS, onAct: fn() } },
  play: async ({ args, userEvent }) => {
    await screen.findByRole("dialog", { name: "Loadout" });
    await userEvent.click(within(skillRow("commit-message")).getByRole("button", { name: "Edit commit-message" }));
    await expect(args.skills!.onAct).toHaveBeenCalledWith({ kind: "skill", name: "commit-message", source: "kit" }, "edit");
    await expect(within(skillRow("productivity:memory-management")).getByRole("button", { name: "Edit productivity:memory-management" })).toBeInTheDocument();
    await expect(within(skillRow("work-issue")).getAllByRole("button")).toHaveLength(1);
  },
};

/** The Loadout with two more of the person's own skills, for the rows a removal leaves. */
const OWN_SKILLS = { state: "read" as const, loadout: { items: [...LOADOUT.items, { kind: "skill" as const, name: "deep-research", source: "kit" as const }, { kind: "skill" as const, name: "pdf", source: "kit" as const }], turns: TURNS } };

/**
 * One of the person's own skills offers Remove behind Edit. **Nothing is sent before the confirm**, which says every
 * Job on this Mac loses it from its next start and Restore brings it back from the Trash.
 */
export const SkillRemove: Story = {
  args: { onOpenItem: fn(), skills: { rows: { ...SKILL_ACTS, "skill:kit::commit-message": { edits: OWN_SKILL, removes: true } }, onAct: fn() } },
  play: async ({ args, userEvent }) => {
    await screen.findByRole("dialog", { name: "Loadout" });
    const row = skillRow("commit-message");
    await expect(await menuOf(row, "commit-message", userEvent)).toEqual(["Remove"]);
    await userEvent.click(within(row).getByRole("menuitem", { name: "Remove" }));
    const confirm = await screen.findByRole("dialog", { name: "Remove commit-message from this Mac" });
    await expect(confirm).toHaveTextContent("Every Job on this Mac loses it from its next start.");
    await expect(confirm).toHaveTextContent("Its folder goes to the Trash. Restore on its row brings it back.");
    await expect(args.skills!.onAct).not.toHaveBeenCalled();
    await userEvent.click(within(confirm).getByRole("button", { name: "Remove" }));
    await expect(args.skills!.onAct).toHaveBeenCalledWith({ kind: "skill", name: "commit-message", source: "kit" }, "remove");
  },
};

/**
 * A skill removed stays on its row, marked removed, with Restore on its face and nothing else. A removal out shows on
 * its row, and one Fleet refused says why there.
 */
export const SkillRemoved: Story = {
  args: {
    onOpenItem: fn(),
    read: OWN_SKILLS,
    skills: {
      rows: {
        "skill:kit::commit-message": { removed: { restorable: true } },
        "skill:kit::deep-research": { edits: "/Users/user/.harness/skills/deep-research/SKILL.md", removes: true, acting: "remove" },
        "skill:kit::pdf": { edits: "/Users/user/.harness/skills/pdf/SKILL.md", removes: true, refused: "Nothing was removed. The skill's folder is not where Kit last read it." },
      },
      onAct: fn(),
    },
  },
  play: async ({ args, userEvent }) => {
    await screen.findByRole("dialog", { name: "Loadout" });
    const removed = skillRow("commit-message");
    await expect(within(removed).getByRole("img", { name: "Removed" })).toBeInTheDocument();
    await expect(within(removed).queryByRole("button", { name: "Edit commit-message" })).not.toBeInTheDocument();
    await expect(within(removed).queryByRole("button", { name: "More for commit-message" })).not.toBeInTheDocument();
    await userEvent.click(within(removed).getByRole("button", { name: "Restore commit-message" }));
    await expect(args.skills!.onAct).toHaveBeenCalledWith({ kind: "skill", name: "commit-message", source: "kit" }, "restore");
    await expect(within(skillRow("deep-research")).getByRole("button", { name: "Removing deep-research" })).toHaveAttribute("aria-busy", "true");
    await expect(skillRow("pdf")).toHaveTextContent("Nothing was removed. The skill's folder is not where Kit last read it.");
  },
};

/**
 * A plugin's skill offers to remove the plugin it came with. The confirm names the command that uninstalls it and every
 * skill it takes, and says there is no Undo; nothing is sent before it.
 */
export const SkillRemovePlugin: Story = {
  args: {
    onOpenItem: fn(),
    skills: {
      rows: {
        ...SKILL_ACTS,
        "skill:plugin:productivity:productivity:memory-management": {
          edits: PLUGIN_SKILL,
          plugin: { name: "productivity", uninstall: "harness plugin uninstall productivity@toolshed", takes: ["memory-management", "summarise"] },
        },
      },
      onAct: fn(),
    },
  },
  play: async ({ args, userEvent }) => {
    await screen.findByRole("dialog", { name: "Loadout" });
    const row = skillRow("productivity:memory-management");
    await expect(await menuOf(row, "productivity:memory-management", userEvent)).toEqual(["Remove plugin productivity"]);
    await userEvent.click(within(row).getByRole("menuitem", { name: "Remove plugin productivity" }));
    const confirm = await screen.findByRole("dialog", { name: "Remove plugin productivity from this Mac" });
    await expect(confirm).toHaveTextContent("harness plugin uninstall productivity@toolshed");
    await expect(confirm).toHaveTextContent("Removes its skills: memory-management, summarise.");
    await expect(confirm).toHaveTextContent("Every Job on this Mac loses them from its next start.");
    await expect(confirm).toHaveTextContent("There is no Undo.");
    await expect(args.skills!.onAct).not.toHaveBeenCalled();
    await userEvent.click(within(confirm).getByRole("button", { name: "Remove plugin" }));
    await expect(args.skills!.onAct).toHaveBeenCalledWith({ kind: "skill", name: "productivity:memory-management", source: "plugin", from: "productivity" }, "remove_plugin");
  },
};

/**
 * A repository's, a built in and a managed skill are not Kit's to change, so their rows offer no Edit and no Remove
 * even where the host's match names them.
 */
export const NoKitActs: Story = {
  args: {
    onOpenItem: fn(),
    read: { state: "read", loadout: { items: [...LOADOUT.items, { kind: "skill", name: "house-style", source: "managed" }], turns: TURNS } },
    skills: {
      rows: {
        "skill:repository::work-issue": { edits: "/Users/user/armada/.harness/skills/work-issue/SKILL.md", removes: true },
        "skill:built_in::code-review": { edits: "/Users/user/.harness/skills/code-review/SKILL.md", removes: true },
        "skill:managed::house-style": { edits: "/Library/Harness/skills/house-style/SKILL.md", removes: true },
      },
      onAct: fn(),
    },
  },
  play: async () => {
    await screen.findByRole("dialog", { name: "Loadout" });
    for (const name of ["work-issue", "code-review", "house-style"]) {
      await expect(within(skillRow(name)).getAllByRole("button")).toHaveLength(1);
      await expect(within(skillRow(name)).queryByRole("button", { name: `Edit ${name}` })).not.toBeInTheDocument();
    }
  },
};

/** Opened by `/mcp`: the agent's report while the first live read is out, and Fleet's words where the read was refused. */
export const LiveNotRead: Story = {
  args: { at: "mcp", mcp: { said: "The Job's process is not running.", onAct: fn() } },
  play: async () => {
    const servers = within(await screen.findByRole("dialog", { name: "Loadout" })).getByRole("region", { name: "MCP servers" });
    await expect(servers).toHaveTextContent("The Job's process is not running.");
    await expect(within(servers).getByRole("img", { name: "Failed" })).toBeInTheDocument();
  },
};
