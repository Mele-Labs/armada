import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, waitFor, within } from "storybook/test";

import { loadoutKeyOf, type LoadoutRef } from "../../loadout";
import { timeSaid } from "../../time-format";
import { LoadoutSheet, type LoadoutOpened, type LoadoutSheetProps } from "./LoadoutSheet";
import { BODIES, LOADOUT, TURNS } from "./LoadoutSheet.fixtures";

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
