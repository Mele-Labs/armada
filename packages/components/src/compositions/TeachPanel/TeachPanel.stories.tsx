import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { TeachPanel } from "./TeachPanel";

const meta: Meta<typeof TeachPanel> = {
  title: "Compositions/Teach panel",
  component: TeachPanel,
  args: {
    source: { kind: "notice", text: "Branch is 3 commits behind main" },
    say: "Rebase quietly, and tell me only when it conflicts",
    onSay: fn(),
    keep: "story:teach-panel",
    repository: "armada",
    reach: "repository",
    stage: "writing",
    onReach: fn(),
    onTeach: fn(),
    onChange: fn(),
    onSave: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof TeachPanel>;

export const Writing: Story = {
  play: async ({ canvasElement, args }) => {
    const panel = within(canvasElement);
    await expect(panel.getByRole("region", { name: "Notice" })).toBeVisible();
    await userEvent.click(panel.getByRole("button", { name: /Teach/ }));
    await expect(args.onTeach).toHaveBeenCalledTimes(1);
  },
};

/** Nothing said, so there is nothing to teach. */
export const NothingSaid: Story = {
  args: { say: "" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: /Teach/ })).toBeDisabled();
  },
};

export const EveryRepository: Story = {
  args: { reach: "all" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: "Every repository" })).toHaveAttribute("aria-pressed", "true");
  },
};

export const Reading: Story = {
  args: { stage: "reading" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("status", { name: "Reading what you wrote" })).toBeVisible();
  },
};

/** Agents, Bridge with a link whose address was found, and Fleet with a condition and a line it cannot test. */
export const Shown: Story = {
  args: {
    stage: "shown",
    reading: {
      agents: "Rebase onto main before reporting. Report a conflict, not a clean rebase.",
      bridge: {
        name: "Issue numbers",
        pattern: "#\\d+",
        example: "#2822",
        address: "https://tracker.example/pulls/2822",
        found: "Found in the origin remote",
      },
      fleet: {
        notice: "Branch is behind main",
        matches: "\\d+ commits behind",
        does: "Rebases the branch when it is idle",
        scope: "armada",
        untestable: "Quietly cannot be tested",
      },
    },
  },
  play: async ({ canvasElement, args }) => {
    const panel = within(canvasElement);
    await expect(panel.getByRole("region", { name: "Agents" })).toBeVisible();
    const bridge = panel.getByRole("region", { name: "Bridge" });
    await expect(within(bridge).getByText("Found in the origin remote")).toBeVisible();
    const fleet = panel.getByRole("region", { name: "Fleet" });
    await expect(within(fleet).getByText("Matches")).toBeVisible();
    await expect(within(fleet).getByText("Quietly cannot be tested")).toBeVisible();
    await userEvent.click(panel.getByRole("button", { name: "Change" }));
    await expect(args.onChange).toHaveBeenCalledTimes(1);
    await userEvent.click(panel.getByRole("button", { name: /Save/ }));
    await expect(args.onSave).toHaveBeenCalledTimes(1);
  },
};

/** A pattern with no address found draws the pattern alone. */
export const BridgeWithoutAnAddress: Story = {
  args: {
    stage: "shown",
    reading: { bridge: { name: "Issue numbers", pattern: "#\\d+", example: "#2822" } },
  },
  play: async ({ canvasElement }) => {
    const bridge = within(canvasElement).getByRole("region", { name: "Bridge" });
    await expect(within(bridge).queryByText("#2822")).toBeNull();
  },
};

/** A notice of the word, with no pattern over its text. */
export const FleetEveryNotice: Story = {
  args: {
    stage: "shown",
    reading: { fleet: { notice: "Branch is behind main", does: "Rebases the branch when it is idle", scope: "armada" } },
  },
  play: async ({ canvasElement }) => {
    const fleet = within(canvasElement).getByRole("region", { name: "Fleet" });
    await expect(within(fleet).queryByText("Matches")).toBeNull();
  },
};

/** Teach on Bridge itself: a shared colour, laid over whichever theme is chosen. No reach, and no note for agents. */
export const ThemeTweak: Story = {
  args: {
    source: { kind: "panel", text: "Cockpit" },
    say: "The accent is too quiet",
    stage: "shown",
    reading: {
      tweak: {
        colour: "Accent",
        token: "--accent",
        was: "var(--accent-muted)",
        now: "var(--accent-hover)",
        places: ["Job row", "Tab strip", "Primary button"],
      },
    },
  },
  play: async ({ canvasElement }) => {
    const panel = within(canvasElement);
    await expect(panel.getByRole("region", { name: "Theme tweak" })).toBeVisible();
    await expect(panel.getByRole("list", { name: "Also drawn in" })).toBeVisible();
    await expect(panel.queryByRole("button", { name: "Every repository" })).toBeNull();
  },
};

export const LayoutChange: Story = {
  args: {
    source: { kind: "row", text: "Hold on failed checks" },
    say: "Hide the branch column",
    stage: "shown",
    reading: { layout: { hides: "Branch column", where: "Jobs list" } },
  },
  play: async ({ canvasElement }) => {
    const layout = within(canvasElement).getByRole("region", { name: "Layout" });
    await expect(within(layout).getByText("Branch column")).toBeVisible();
  },
};
