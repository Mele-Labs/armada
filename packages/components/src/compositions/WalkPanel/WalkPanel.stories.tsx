import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SessionThread, type SessionThreadRow } from "../SessionThread/SessionThread";
import { WalkPanel } from "./WalkPanel";

/**
 * Beside a walk window's page: the Session that showed it. Its kind is named first.
 */
const meta: Meta<typeof WalkPanel> = {
  title: "Compositions/Walk panel",
  component: WalkPanel,
  args: { onOpenInBridge: fn() },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 10)", width: "var(--w-sheet)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof WalkPanel>;

const ROWS: SessionThreadRow[] = [
  { id: "1", at: "14:03:07", kind: "message", from: "you", text: "Put the count beside the findings title." },
  { id: "2", at: "14:03:28", kind: "message", from: "agent", text: "Done. It is on the page I showed you." },
  { id: "3", at: "14:03:31", kind: "window", title: "Findings page", url: "http://localhost:5173/findings" },
];

/** A Session's walk: its thread, live, beside the page it showed. Open in Bridge goes to the whole Session. */
export const Session: Story = {
  args: {
    title: "Findings page",
    children: <SessionThread sessionId="s1" rows={ROWS} onAnswer={fn()} onOpenSession={fn()} />,
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.getByRole("region", { name: "Job: Findings page" })).toBeVisible();
    await expect(canvas.getByText("Done. It is on the page I showed you.")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Open in Bridge" }));
    await expect(args.onOpenInBridge).toHaveBeenCalled();
  },
};
