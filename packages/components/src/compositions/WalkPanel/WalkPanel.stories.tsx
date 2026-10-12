import type { Meta, StoryObj } from "@storybook/react-vite";
import { Eye } from "lucide-react";
import { expect, fn, userEvent } from "storybook/test";

import { Badge } from "../../primitives/Badge/Badge";
import { ReviewDecision } from "../ReviewDecision/ReviewDecision";
import { SessionThread, type SessionThreadRow } from "../SessionThread/SessionThread";
import { Prose } from "../../primitives/Prose/Prose";
import { WalkPanel, WalkPanelSection } from "./WalkPanel";

/**
 * Beside a walk window's page: the Session that showed it, or the Job whose server it walks. Its kind
 * is named first, so a person answering it knows which they are answering.
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
    kind: "session",
    title: "Findings page",
    children: <SessionThread sessionId="s1" rows={ROWS} onAnswer={fn()} onOpenSession={fn()} />,
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.getByRole("region", { name: "Session: Findings page" })).toBeVisible();
    await expect(canvas.getByText("Done. It is on the page I showed you.")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Open in Bridge" }));
    await expect(args.onOpenInBridge).toHaveBeenCalled();
  },
};

/** A Job's walk: its state, and the decision JobDetail draws at the review gate. Never called a Session. */
export const Job: Story = {
  args: {
    kind: "job",
    title: "44-try-a-stacked-run-beside-the-canvas",
    status: <Badge status="awaiting-review" icon={Eye}>needs review</Badge>,
    children: (
      <>
        <WalkPanelSection head="Brief">
          <Prose text="Try a stacked run beside the canvas, so a person reads the run without leaving the plan." />
        </WalkPanelSection>
        <WalkPanelSection head="Decision">
          <ReviewDecision note="" onNote={fn()} onApprove={fn()} onRequestChanges={fn()} onReject={fn()} />
        </WalkPanelSection>
      </>
    ),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Job: 44-try-a-stacked-run-beside-the-canvas" })).toBeVisible();
    await expect(canvas.queryByText("Session")).toBeNull();
    await expect(canvas.getByText("needs review")).toBeVisible();
    await expect(canvas.getByRole("region", { name: "Brief" })).toBeVisible();
  },
};
