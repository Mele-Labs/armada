import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SessionThread, type SessionThreadRow } from "./SessionThread";

/**
 * A Session's conversation: Helm's rows, the first write as a filled row, a
 * message from another Session named by its sender, and the permission the
 * agent is held on above the composer.
 */
const meta: Meta<typeof SessionThread> = {
  title: "Compositions/Session thread",
  component: SessionThread,
  args: { rows: [], working: false, onSend: fn(), onAnswer: fn() },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 8)", width: "calc(var(--space-12) * 10)", background: "var(--bg-raised)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionThread>;

const ROWS: SessionThreadRow[] = [
  { id: "1", at: "14:03:07", kind: "message", from: "you", text: "Fix the flaky store test." },
  { id: "2", at: "14:03:14", kind: "tool", text: "Read crates/store/tests/flaky.rs" },
  { id: "3", at: "14:03:21", kind: "lease", slot: 3, branch: "fix/flaky-store" },
  { id: "4", at: "14:03:28", kind: "message", from: "agent", text: "The test reads the wall clock." },
  { id: "5", at: "14:04:45", kind: "message", from: "session", sender: { id: "s2", title: "Release notes script" }, text: "your branch broke main" },
];

/** Nothing said: nothing drawn but the composer. */
export const Blank: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("note")).toBeNull();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
  },
};

/** The first write is its own row, and the sender of a Session's message is named. */
export const FirstWriteAndASender: Story = {
  args: { rows: ROWS },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Leased on first write" })).toBeInTheDocument();
    await expect(canvas.getByRole("region", { name: "Message from Release notes script" })).toBeInTheDocument();
  },
};

/** Held on a command: the answers sit over the message box. */
export const Held: Story = {
  args: { rows: ROWS, asked: { command: "git push --force-with-lease origin fix/flaky-store" } },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Allow once" }));
    await expect(args.onAnswer).toHaveBeenCalled();
  },
};
