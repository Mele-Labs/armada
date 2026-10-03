import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { GUIDE_PULSE } from "../../guides";
import { Button } from "../../primitives/Button/Button";
import { DestinationCard } from "./DestinationCard";

const meta: Meta<typeof DestinationCard> = {
  title: "Compositions/Destination card",
  component: DestinationCard,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof DestinationCard>;

const BODY = <p style={{ margin: 0 }}>2 Drones are working on this Job.</p>;

/** `chevron-right` is `aria-hidden`, so no role carries it; lucide names its glyph on the svg. */
function chevron(card: HTMLElement): Element | null {
  return card.querySelector("svg.lucide-chevron-right");
}

/** The name, then what the card holds. Nothing to press, and no chevron. */
export const Plain: Story = {
  args: { label: "Stats", children: BODY },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const card = canvas.getByRole("region", { name: "Stats" });
    await expect(canvas.getByRole("heading", { name: "Stats" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Open Stats" })).toBeNull();
    await expect(chevron(card)).toBeNull();
  },
};

/** Holding nothing: **the head alone**, and no empty body under it. */
export const HoldingNothing: Story = {
  args: { label: "Plan", onOpen: fn(), children: null },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const card = canvas.getByRole("region", { name: "Plan" });
    await expect(card).toHaveTextContent(/^Plan$/);
    // No body under the head: the one structural fact no role carries.
    await expect(card.children).toHaveLength(1);
  },
};

/** A `?` beside the name, and a figure and an act at the trailing edge. */
export const WithTrailing: Story = {
  args: {
    label: "Pulse",
    guide: GUIDE_PULSE,
    trailing: (
      <>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)" }}>1.2 GiB</span>
        <Button variant="ghost" size="sm">
          Refresh
        </Button>
      </>
    ),
    children: BODY,
  },
};

/**
 * **The head is a press that opens the destination**, and the chevron trails
 * everything the head says. The trailing slot keeps its own press: Refresh
 * refreshes and does not open, because the press lies behind the head rather
 * than around it.
 */
export const Opens: Story = {
  args: {
    label: "Pulse",
    guide: GUIDE_PULSE,
    trailing: (
      <Button variant="ghost" size="sm" onClick={fn().mockName("refresh")}>
        Refresh
      </Button>
    ),
    onOpen: fn(),
    children: BODY,
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const card = canvas.getByRole("region", { name: "Pulse" });

    await userEvent.click(canvas.getByRole("button", { name: "Refresh" }));
    await expect(args.onOpen).not.toHaveBeenCalled();

    await userEvent.click(canvas.getByRole("button", { name: "Open Pulse" }));
    await expect(args.onOpen).toHaveBeenCalledTimes(1);

    // Present, and last in the head: after `trailing`, not before it.
    const mark = chevron(card);
    await expect(mark).not.toBeNull();
    await expect(mark!.parentElement!.lastElementChild).toBe(mark);
  },
};
