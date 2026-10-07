import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionThread, type SessionThreadRow } from "./SessionThread";

/**
 * A Session's conversation: Helm's rows, the first write as a filled row, a
 * message from another Session under a band of its own that opens the sender,
 * and the permission the agent is held on.
 */
const meta: Meta<typeof SessionThread> = {
  title: "Compositions/Session thread",
  component: SessionThread,
  args: { rows: [], onAnswer: fn(), onOpenSession: fn() },
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
  { id: "1", at: "14:03:07", kind: "message", from: "you", text: "Fix the flaky store test.", sketches: [{ id: "k1", title: "Store clock" }], tags: [{ kind: "session", id: "s2", title: "Release notes script" }, { kind: "job", id: "j55", title: "55 Cap the retry backoff" }] },
  { id: "2", at: "14:03:14", kind: "tool", text: "Read crates/store/tests/flaky.rs" },
  { id: "3", at: "14:03:21", kind: "lease", slot: 3, branch: "fix/flaky-store" },
  { id: "4", at: "14:03:28", kind: "message", from: "agent", text: "The test reads the wall clock." },
  { id: "5", at: "14:04:45", kind: "message", from: "session", sender: { id: "s2", title: "Release notes script" }, text: "your branch broke main" },
];

/** Nothing said: nothing drawn. */
export const Blank: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("note")).toBeNull();
  },
};

/** The first write is its own row, and another Session's message has a band that opens its sender. */
export const FirstWriteAndASender: Story = {
  args: { rows: ROWS },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("region", { name: "Leased on first write" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Session Release notes script" }));
    await expect(args.onOpenSession).toHaveBeenCalledWith("s2");
  },
};

/** Held on a command: the answers sit under the thread. */
export const Held: Story = {
  args: { rows: ROWS, asked: { command: "git push --force-with-lease origin fix/flaky-store" } },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Allow once" }));
    await expect(args.onAnswer).toHaveBeenCalled();
  },
};

/** Fleet's own offers, in its order: each answer is named and hands its id back. */
export const HeldWithFleetsOffers: Story = {
  args: {
    rows: ROWS,
    asked: {
      command: "git push origin fix/flaky-store",
      offers: [
        { id: "allow_once", label: "Allow once", means: "Runs it now. Nothing is written down." },
        { id: "allow_and_remember", label: "Allow and remember", means: "Runs it now, and the rule goes into the repository's own agent settings." },
        { id: "refuse", label: "Refuse", means: "The agent is told no." },
      ],
    },
  },
  play: async ({ canvas, args }) => {
    const answers = within(canvas.getByRole("group", { name: "Answers" }));
    await expect(answers.getAllByRole("button").map((one) => one.textContent)).toEqual(["Allow once", "Allow and remember", "Refuse"]);
    await userEvent.click(answers.getByRole("button", { name: "Allow and remember" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("allow_and_remember");
  },
};

/** A Job taken over from a gate or an escalation: no Drone said what it was stuck on, so the Drone's account is not drawn, and neither is any other group with nothing in it. */
export const HandedOverWithNoAccount: Story = {
  args: {
    rows: [
      {
        id: "h1",
        at: "14:03:07",
        kind: "handoff",
        job: { number: 55, title: "Cap the retry backoff" },
        slot: 3,
        branch: "fix/retry-backoff",
        step: { id: "regression_verify", label: "Verify the fix" },
        attempts: 3,
        refusals: [],
        plan: { outside: ["crates/retry/src/loop.rs"], unwritten: [] },
      },
    ],
  },
  play: async ({ canvas }) => {
    const handoff = canvas.getByRole("region", { name: "Handed over: Job 55" });
    await expect(within(handoff).getByText("Stopped on")).toBeInTheDocument();
    await expect(within(handoff).getByText("written, not declared")).toBeInTheDocument();
    await expect(within(handoff).queryByText("Judge refused")).toBeNull();
    await expect(within(handoff).queryByText("Trying to")).toBeNull();
  },
};
