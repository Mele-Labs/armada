import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, within } from "storybook/test";

import { turnSaid } from "../../loadout";
import { LoadoutSheet } from "./LoadoutSheet";
import { LOADOUT, TURNS } from "./LoadoutSheet.fixtures";

/** A Session's whole Loadout, in the panel over the work area: what it knows, what it was told, and what each turn used. */
const meta: Meta<typeof LoadoutSheet> = {
  title: "Compositions/Loadout sheet",
  component: LoadoutSheet,
  args: { open: true, read: { state: "read", loadout: LOADOUT }, onClose: fn() },
};
export default meta;

type Story = StoryObj<typeof LoadoutSheet>;

const latest = `Turn at ${turnSaid(TURNS[1]!.started_at)}`;
const earlier = `Turn at ${turnSaid(TURNS[0]!.started_at)}`;

/**
 * The latest turn is open and the one before is closed; a press opens it in its place. The source filter narrows all
 * three sections, and a server that failed says so on its mark.
 */
export const Read: Story = {
  play: async ({ userEvent }) => {
    const sheet = await screen.findByRole("dialog", { name: "Loadout" });
    await expect(within(sheet).getByRole("button", { name: latest })).toHaveAttribute("aria-expanded", "true");
    const used = within(sheet).getByRole("list", { name: `Used in the turn at ${turnSaid(TURNS[1]!.started_at)}` });
    await expect(within(used).getByText("armada:mechanic").parentElement).toHaveTextContent("Armada · sonnet");
    await expect(within(used).getByText("mcp__github__pull_request_read").parentElement).toHaveTextContent("github · by armada:mechanic");

    await userEvent.click(within(sheet).getByRole("button", { name: earlier }));
    await expect(within(sheet).getByRole("button", { name: earlier })).toHaveAttribute("aria-expanded", "true");
    await expect(within(sheet).getByRole("button", { name: latest })).toHaveAttribute("aria-expanded", "false");

    const servers = within(sheet).getByRole("region", { name: "MCP servers" });
    await expect(within(servers).getByRole("img", { name: "Failed" })).toBeInTheDocument();

    await userEvent.selectOptions(within(sheet).getByRole("combobox", { name: "Source" }), "Armada");
    const knows = within(sheet).getByRole("region", { name: "Knows" });
    await expect(within(knows).queryByText("work-issue")).not.toBeInTheDocument();
    await expect(within(knows).getAllByText("armada:mechanic").length).toBeGreaterThan(0);
    await expect(within(sheet).getByRole("region", { name: "Told" })).toHaveTextContent("Armada session rules");
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
