import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor } from "storybook/test";
import { Tabs } from "./Tabs";

const meta: Meta<typeof Tabs> = {
  title: "Primitives/Tabs",
  component: Tabs,
};
export default meta;

type Story = StoryObj<typeof Tabs>;

/**
 * The state the component sheet draws: sections of one object, with no count,
 * because these are views of one job and there is nothing to tally.
 */
export const SectionsOfOneObject: Story = {
  args: {
    defaultValue: "diff",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" },
    ],
  },
  /**
   * The arrows, and the wrap at the end of the strip. `Tabs` computes the next
   * index itself rather than taking one from a library, so the wrap is
   * arithmetic somebody wrote — and arithmetic on an index is where an
   * off-by-one lives.
   *
   * Left twice from `Diff` is the case worth pressing: once to `Log` through
   * the wrap, and the strip is three long, so a modulo that lost its `+
   * items.length` would land on nothing instead.
   */
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Diff" }));
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Diff");

    await userEvent.keyboard("{ArrowRight}");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Evidence");

    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("Log");
  },
};

/** The last section active, so the fill can be read away from the left edge. */
export const LastActive: Story = {
  args: {
    defaultValue: "log",
    items: [
      { id: "diff", label: "Diff" },
      { id: "evidence", label: "Evidence" },
      { id: "log", label: "Log" },
    ],
  },
};

/**
 * **A tab whose label is the short form of something longer.** An Epic's wave
 * strip names each wave by the first sentence of its plan's approach; hovering
 * the tab reads the whole paragraph. A tab with no hint draws no tooltip, and
 * the hint never becomes part of the tab's name — it is its description.
 */
export const WithAHint: Story = {
  args: {
    defaultValue: "2",
    items: [
      {
        id: "1",
        label: "Wave 1 · The seam as one Job",
        hint: "The seam as one Job. Every refusal is read in one place before any surface draws it, so the surfaces have one shape to follow.",
      },
      {
        id: "2",
        label: "Wave 2 · The seam first, then every surface that reads it",
        hint: "The seam first, then every surface that reads it. The roll-up sent the first pass back as too large to review, so the seam lands alone and each surface follows it.",
      },
      { id: "3", label: "Wave 3" },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const first = canvas.getByRole("tab", { name: "Wave 1 · The seam as one Job" });
    await expect(first).toHaveAccessibleDescription(/Every refusal is read in one place/);
    await expect(canvas.getByRole("tab", { name: "Wave 3" })).not.toHaveAttribute("aria-describedby");

    await userEvent.hover(first);
    await waitFor(() => expect(canvas.getByText(/Every refusal is read in one place/)).toBeVisible());

    // Still a tab strip: the arrows move the selection through a hinted tab.
    await userEvent.click(first);
    await userEvent.keyboard("{ArrowRight}");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName(
      "Wave 2 · The seam first, then every surface that reads it",
    );
  },
};
