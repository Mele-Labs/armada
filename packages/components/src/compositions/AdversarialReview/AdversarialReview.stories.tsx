import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { AdversarialReview, type AdversarialReviewState, type Finding } from "./AdversarialReview";

/**
 * One story per state: idle, running, done with findings, done with none, failed, stale, and a list part acted on.
 * It knows nothing of what was read: the host gives it a state and the callbacks.
 */
const meta: Meta<typeof AdversarialReview> = {
  title: "Compositions/Adversarial review",
  component: AdversarialReview,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-dialog)" }}>
        <Story />
      </div>
    ),
  ],
  args: { onRun: () => undefined, onSend: () => undefined, onDismiss: () => undefined, shortcut: "R" },
};
export default meta;

type Story = StoryObj<typeof AdversarialReview>;

const FINDINGS: readonly Finding[] = [
  {
    id: "f1",
    consequence: "A lease waits ten minutes when every slot is held",
    expected: "The warm skips a held slot and tries the next",
    found: "Task W2 warms the slot the pool picked, held or not, and the lease waits on that warm to end",
    cites: ["crates/fleet/src/pool.rs:212"],
  },
  {
    id: "f2",
    consequence: "A bay shows warming for good after Fleet restarts",
    expected: "A warm that was running when Fleet stopped is cleared on the next start",
    found: "Task W4 adds warming to the bay reply and nothing in the plan clears it, so the flag outlives the warm it described",
    cites: ["crates/ipc/src/bay.rs:48", "apps/desktop/src/renderer/src/cockpit/deck/BayTile.tsx:96"],
  },
];

export const Idle: Story = { args: { review: { state: "idle" } } };

export const Running: Story = { args: { review: { state: "running" } } };

export const DoneWithFindings: Story = { args: { review: { state: "done", findings: FINDINGS } } };

export const DoneWithNone: Story = { args: { review: { state: "done", findings: [] } } };

export const Failed: Story = { args: { review: { state: "failed", said: "The adversary stopped before it finished reading" } } };

export const Stale: Story = { args: { review: { state: "stale", read: 1, findings: FINDINGS } } };

export const OneActedOn: Story = { args: { review: { state: "done", findings: [{ ...FINDINGS[0]!, settled: "sent" }, FINDINGS[1]!] } } };

/** Sending and dismissing fold a finding to a line, and focus passes to the next. */
export const Acting: Story = {
  render: (args) => {
    const [review, setReview] = useState<AdversarialReviewState>({ state: "done", findings: FINDINGS });
    const settle = (id: string, settled: "sent" | "dismissed") =>
      setReview((was) => (was.state === "done" ? { ...was, findings: was.findings.map((one) => (one.id === id ? { ...one, settled } : one)) } : was));
    return <AdversarialReview {...args} review={review} onSend={(id) => settle(id, "sent")} onDismiss={(id) => settle(id, "dismissed")} />;
  },
};
