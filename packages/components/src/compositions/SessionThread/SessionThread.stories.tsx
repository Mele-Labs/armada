import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
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

const WORKED: SessionThreadRow[] = [
  { id: "w1", at: "14:03:07", kind: "message", from: "you", text: "Fix the flaky store test." },
  { id: "w2", at: "14:03:14", kind: "tool", text: "Read crates/store/tests/flaky.rs" },
  { id: "w3", at: "14:03:15", kind: "tool", text: "Bash cat /tmp/out.txt" },
  { id: "w4", at: "14:03:16", kind: "tool", text: "Read crates/store/src/clock.rs" },
  { id: "w5", at: "14:03:28", kind: "message", from: "agent", text: "The test reads the wall clock." },
  { id: "w6", at: "14:03:40", kind: "command", text: "/reload-plugins" },
  { id: "w7", at: "14:03:50", kind: "compaction", text: "This session is being continued from a previous conversation." },
];

/** Calls that ran one after another are one closed row naming the tools, and pressing it shows each call. */
export const ToolCallsFoldIntoOneRow: Story = {
  args: { rows: WORKED },
  play: async ({ canvas }) => {
    const group = canvas.getByLabelText("Tool calls: Read, Bash").closest("details");
    await expect(group).not.toHaveAttribute("open");
    await expect(canvas.getByText("Read, Bash")).toBeVisible();
    await userEvent.click(canvas.getByLabelText("Tool calls: Read, Bash"));
    await expect(group).toHaveAttribute("open");
    await expect(canvas.getByText("Bash cat /tmp/out.txt")).toBeVisible();
  },
};

/** A command is a line as typed, a compaction is a quiet row that opens to its text, and neither is "You". */
export const CommandAndCompaction: Story = {
  args: { rows: WORKED },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("/reload-plugins")).toBeVisible();
    await expect(canvas.getAllByText("You")).toHaveLength(1);
    await userEvent.click(canvas.getByText("Conversation compacted"));
    await expect(canvas.getByText(/continued from a previous conversation/)).toBeVisible();
  },
};

/** Every message is left aligned, yours too, and sits on the panel with no fill of its own. */
export const MessagesAreLeftAlignedWithNoFill: Story = {
  args: { rows: WORKED },
  play: async ({ canvas }) => {
    for (const text of ["Fix the flaky store test.", "The test reads the wall clock."]) {
      const message = canvas.getByText(text).closest(".armada-helm-thread__message") as HTMLElement;
      const row = message.closest("li") as HTMLElement;
      await expect(getComputedStyle(row).textAlign).not.toBe("right");
      await expect(getComputedStyle(row).alignItems).not.toBe("flex-end");
      await expect(getComputedStyle(message).backgroundColor).toBe("rgba(0, 0, 0, 0)");
    }
  },
};

const LONG: SessionThreadRow[] = Array.from({ length: 40 }, (_, at): SessionThreadRow => ({ id: `m${at}`, at: "14:03:07", kind: "message", from: at % 2 === 0 ? "you" : "agent", text: `Message ${at + 1}` }));

/** A long thread opens at its newest message, and a row that arrives keeps it there. */
export const OpensAtTheNewest: Story = {
  args: { rows: LONG, sessionId: "s1" },
  render: (args) => {
    const [rows, setRows] = useState(args.rows);
    return (
      <>
        <button onClick={() => setRows((was) => [...was, { id: "last", at: "14:09:00", kind: "message", from: "agent", text: "Newest of all" }])}>Add a row</button>
        <SessionThread {...args} rows={rows} />
      </>
    );
  },
  play: async ({ canvas }) => {
    const region = canvas.getByRole("region", { name: "Thread" });
    await expect(region.scrollTop + region.clientHeight).toBeGreaterThanOrEqual(region.scrollHeight - 2);
    await userEvent.click(canvas.getByRole("button", { name: "Add a row" }));
    await expect(canvas.getByText("Newest of all")).toBeVisible();
    region.scrollTop = 0;
    region.dispatchEvent(new Event("scroll"));
    await userEvent.click(canvas.getByRole("button", { name: "Add a row" }));
    await expect(region.scrollTop).toBe(0);
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

/** The agent asking the person: options and Other, a multi-select, and Answer held until each has something. */
export const Questions: Story = {
  args: {
    rows: ROWS,
    asked: {
      command: "Which toppings?",
      offers: [
        { id: "allow_once", label: "Allow once", means: "" },
        { id: "refuse", label: "Refuse", means: "" },
      ],
      questions: [
        {
          question: "Which toppings?",
          header: "Toppings",
          multi_select: true,
          options: [
            { label: "cheese", description: "Melted" },
            { label: "ham", description: "Cured" },
          ],
        },
        {
          question: "Which size?",
          header: "Size",
          multi_select: false,
          options: [
            { label: "S", description: "Small" },
            { label: "L", description: "Large" },
          ],
        },
      ],
    },
  },
  play: async ({ canvas, args }) => {
    const answer = canvas.getByRole("button", { name: "Answer" });
    await expect(answer).toBeDisabled();
    await userEvent.click(canvas.getByRole("checkbox", { name: "cheese" }));
    await userEvent.click(canvas.getByRole("checkbox", { name: "ham" }));
    await expect(answer).toBeDisabled();
    await userEvent.click(canvas.getByRole("radio", { name: "Other" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Other" }), "Medium, if you have it");
    await expect(answer).toBeEnabled();
    await userEvent.click(answer);
    await expect(args.onAnswer).toHaveBeenCalledWith("allow_once", [
      { question: "Which toppings?", chosen: ["cheese", "ham"] },
      { question: "Which size?", chosen: ["Medium, if you have it"] },
    ]);
  },
};

/** A question can be skipped, which the agent is told. */
export const QuestionsSkipped: Story = {
  args: {
    rows: ROWS,
    asked: {
      command: "Which size?",
      questions: [{ question: "Which size?", header: "", multi_select: false, options: [{ label: "S", description: "" }, { label: "L", description: "" }] }],
    },
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Skip" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("refuse");
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

/** A running turn marks the thread's end: on the last tool group's row when that is last, else a row of its own. Ended, no mark. */
export const WorkingMark: Story = {
  args: { rows: WORKED.slice(0, 4), working: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Working" })).toBeInTheDocument();
  },
};

export const WorkingMarkAfterAMessage: Story = {
  args: { rows: [WORKED[0]!, WORKED[4]!], working: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Working" })).toBeInTheDocument();
  },
};

export const NotWorkingNoMark: Story = {
  args: { rows: WORKED.slice(0, 4), working: false },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: "Working" })).toBeNull();
  },
};
