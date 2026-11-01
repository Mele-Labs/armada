import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { ThreadSettings } from "./ThreadSettings";
import { DEFAULT_LOOK } from "./thread-look";

/**
 * How a Job's thread reads, and the quick bar's lines: one form for the Aa panel and System › Thread. System adds
 * Voice.
 */
const meta: Meta<typeof ThreadSettings> = {
  title: "Compositions/Thread settings",
  component: ThreadSettings,
  args: { look: { ...DEFAULT_LOOK, agentName: "Xavier" }, onChange: fn(), keep: "story:thread-settings" },
};

export default meta;
type Story = StoryObj<typeof ThreadSettings>;

/** In the Aa panel, for one Job: no Voice, which System holds. */
export const InThePanel: Story = {
  args: { inPanel: true, jobId: "job-1" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByLabelText("Agent name")).toHaveValue("Xavier");
    await expect(canvas.queryByLabelText("Voice")).toBeNull();
  },
};

/** In System › Thread, with Voice and Whisper not yet installed. */
export const InSystem: Story = {
  args: { voice: { whisper: { state: "absent" }, onInstall: fn() } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByLabelText("Voice").length).toBeGreaterThan(0);
  },
};
