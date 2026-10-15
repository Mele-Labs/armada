import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";
import { ArrowUpToLine, Split } from "lucide-react";

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

/** A page the Session showed is a quiet row with its title, and pressing it opens the window again. */
export const ShownWindowRow: Story = {
  args: {
    rows: [{ id: "x1", at: "14:04:00", kind: "window", title: "Store clock findings", url: "https://example.com/findings" }],
    onOpenWindow: fn(),
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open window Store clock findings" }));
    await expect(args.onOpenWindow).toHaveBeenCalledWith("https://example.com/findings");
  },
};

/** The agent's messages are left aligned and sit on the panel with no fill of their own. */
export const AgentMessagesAreLeftAlignedWithNoFill: Story = {
  args: { rows: WORKED },
  play: async ({ canvas }) => {
    const message = canvas.getByText("The test reads the wall clock.").closest(".armada-helm-thread__message") as HTMLElement;
    const row = message.closest("li") as HTMLElement;
    await expect(getComputedStyle(row).textAlign).not.toBe("right");
    await expect(getComputedStyle(row).alignItems).not.toBe("flex-end");
    await expect(getComputedStyle(message).backgroundColor).toBe("rgba(0, 0, 0, 0)");
  },
};

const YOURS: SessionThreadRow[] = [
  { id: "y1", at: "09:55:40", kind: "message", from: "agent", text: "The clock is pinned in the store tests." },
  { id: "y2", at: "09:56:12", kind: "message", from: "you", text: "Keep the pin in the test module." },
  { id: "y3", at: "09:57:23", kind: "message", from: "you", text: "Approved: http://localhost:5188/?walk=session-reply" },
  { id: "y4", at: "09:57:51", kind: "message", from: "you", text: "Re: Which date should the clock pin to?\nPin it to the first of March" },
];

/**
 * What the person said and did are not the agent's row. A message is a card under a band with their
 * name; an Approve and an answer are one line each, the target as a chip, and the raw text is gone.
 */
export const YourWordsAndActs: Story = {
  args: { rows: YOURS },
  play: async ({ canvas }) => {
    const message = canvas.getByRole("region", { name: "Your message" });
    await expect(within(message).getByText("You")).toBeVisible();
    await expect(within(message).getByText("Keep the pin in the test module.")).toBeVisible();

    const approved = canvas.getByRole("region", { name: "You approved" });
    await expect(within(approved).getByText("Approved")).toBeVisible();
    await expect(within(approved).getByText("?walk=session-reply")).toBeVisible();
    // The chip shows the short form and names the whole address.
    await expect(within(approved).getByRole("img", { name: "Approved: http://localhost:5188/?walk=session-reply" })).toBeVisible();

    const answered = canvas.getByRole("region", { name: "You answered" });
    await expect(within(answered).getByText("Answered")).toBeVisible();
    await expect(within(answered).getByText("Which date should the clock pin to?")).toBeVisible();
    await expect(within(answered).getByText("Pin it to the first of March")).toBeVisible();

    await expect(canvas.queryByText(/^Approved: /)).toBeNull();
    await expect(canvas.queryByText(/^Re: /)).toBeNull();

    // The agent's row stays a plain row, named for the agent, in no region of its own.
    const agent = canvas.getByText("The clock is pinned in the store tests.").closest("li") as HTMLElement;
    await expect(agent).not.toHaveAttribute("role");
    await expect(within(agent).getByText("Agent")).toBeVisible();
  },
};

const LONG_ITEM = "Conflict detection: what happens on a real clash, and does Bridge show it";
const LONG_SAID = "Yes bridge shows it. Also we need to account for when agents sessions are done that the index gets cleaned up. I would assume main getting updated also impacts this";

/**
 * A long item and a long answer share a row. The item truncates to one line beside the eyebrow and
 * the time, and the answer wraps at full width on a line of its own under them.
 */
export const LongAnswer: Story = {
  args: { rows: [{ id: "l1", at: "10:02:11", kind: "message", from: "you", text: `Re: ${LONG_ITEM}\n${LONG_SAID}` }] },
  play: async ({ canvas }) => {
    const answered = canvas.getByRole("region", { name: "You answered" });
    const chip = within(answered).getByRole("img", { name: `Answered: ${LONG_ITEM}` });
    const said = within(answered).getByText(LONG_SAID);
    await expect(said).toBeVisible();
    await expect(said.getBoundingClientRect().top).toBeGreaterThanOrEqual(chip.getBoundingClientRect().bottom);
    // One line: the chip is no taller than twice its own line height.
    await expect(chip.getBoundingClientRect().height).toBeLessThan(parseFloat(getComputedStyle(chip).lineHeight) * 2);
    // The answer runs the width of the row, not a narrow column.
    await expect(said.getBoundingClientRect().width).toBeGreaterThan(answered.getBoundingClientRect().width / 2);
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
    // One at a time: a multi-select moves on with Next, the last one sends.
    const next = canvas.getByRole("button", { name: "Next" });
    await expect(next).toBeDisabled();
    await userEvent.click(canvas.getByRole("checkbox", { name: "cheese" }));
    await userEvent.click(canvas.getByRole("checkbox", { name: "ham" }));
    await userEvent.click(next);
    await expect(canvas.getByRole("button", { name: /Toppings: cheese, ham/ })).toBeVisible();
    const answer = canvas.getByRole("button", { name: "Answer" });
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

/** An option the agent attached a preview to shows it, line breaks kept, while it is in focus; one without shows nothing. */
export const QuestionPreview: Story = {
  args: {
    rows: ROWS,
    asked: {
      command: "Use these checks?",
      questions: [
        {
          question: "Use these checks?",
          header: "Checks",
          multi_select: false,
          options: [
            { label: "Yes", description: "", preview: "ops: cargo test\nconsumer: pnpm test" },
            { label: "No", description: "" },
          ],
        },
      ],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("region", { name: "Preview" })).toBeNull();
    await userEvent.hover(canvas.getByRole("radio", { name: "Yes" }));
    const preview = canvas.getByRole("region", { name: "Preview" });
    await expect(preview.textContent).toBe("ops: cargo test\nconsumer: pnpm test");
    await expect(getComputedStyle(preview).whiteSpace).toBe("pre");
    await userEvent.hover(canvas.getByRole("radio", { name: "No" }));
    await expect(canvas.queryByRole("region", { name: "Preview" })).toBeNull();
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

const CHECKED: SessionThreadRow[] = [
  { id: "k1", at: "14:21:40", kind: "check", name: "desktop:typecheck", run: 29, state: "passed" },
  { id: "k2", at: "14:22:20", kind: "check", name: "components_test", run: 30, state: "failed" },
  { id: "k3", at: "14:23:10", kind: "check", name: "app_smoke", run: 31, state: "running" },
];

/** Each `armada check` the agent ran is a row of its own, named by its key; a press opens that run. */
export const ChecksTheAgentRan: Story = {
  args: { rows: CHECKED, onOpenCheck: fn() },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("button", { name: "Check desktop:typecheck, passed" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Check components_test, failed" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Check app_smoke, running" }));
    await expect(args.onOpenCheck).toHaveBeenCalledWith(31);
  },
};

const COMMANDED: SessionThreadRow[] = [
  { id: "m1", at: "14:20:02", kind: "woke", why: "Check store failed" },
  { id: "m2", at: "14:20:05", kind: "tool", text: "Read crates/store/tests/flaky.rs" },
  { id: "m3", at: "14:20:12", kind: "act", icon: ArrowUpToLine, said: "Raised cap", text: "Turn cap 40 \u2192 60" },
  { id: "m4", at: "14:20:13", kind: "act", icon: Split, said: "Split task", text: "T4 split into T4a, T4b" },
  { id: "m5", at: "14:20:15", kind: "message", from: "agent", text: "The Drone ran out of turns." },
];

/** Fleet's wake is a quiet row naming why, and each thing the agent did on its own is a row marked by which act it was. */
export const WakeAndActRows: Story = {
  args: { rows: COMMANDED, agentName: "Commander" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Fleet woke it" })).toBeVisible();
    await expect(canvas.getByText("Check store failed")).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Raised cap" })).toBeVisible();
    await expect(canvas.getByText("Turn cap 40 \u2192 60")).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Split task" })).toBeVisible();
    // A wake and an act are not the person's or the agent's words: no message head is drawn for them.
    await expect(canvas.getAllByText("Commander")).toHaveLength(1);
    await expect(canvas.queryByText("You")).toBeNull();
  },
};

/** The agent's own rows carry the name the host gives, and read `Agent` where it gives none. */
export const AgentNamedByTheHost: Story = {
  args: { rows: [{ id: "n1", at: "14:20:15", kind: "message", from: "agent", text: "Done." }], agentName: "Commander" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Commander")).toBeVisible();
    await expect(canvas.queryByText("Agent")).toBeNull();
  },
};

/** A retro the person pressed for stays in the thread where it was pressed, and its link opens the kept one. */
export const RetroInTheThread: Story = {
  args: {
    onOpenRetro: fn(),
    rows: [
      ...WORKED.slice(0, 1),
      { id: "r1", at: "14:05:00", kind: "retro", stage: "written", characters: 0, retro: 2, items: [{ title: "A reset waited 40 minutes for an answer" }, { title: "Walks opened in your browser" }] },
      { id: "r2", at: "14:09:00", kind: "retro", stage: "writing", stageSince: Date.parse("2026-10-12T14:09:00Z"), characters: 240, items: [{ who: "agent", landsIn: "armada", title: "A restart of Fleet killed the subagents" }] },
    ],
  },
  play: async ({ canvas, args }) => {
    await expect(canvas.getAllByRole("region", { name: "Retro" })).toHaveLength(2);
    await userEvent.click(canvas.getByRole("button", { name: "Open retro" }));
    await expect(args.onOpenRetro).toHaveBeenCalledWith(2);
  },
};

const REPEATED: SessionThreadRow[] = [
  { id: "f1", at: "14:30:01", kind: "check", name: "test", run: 40, state: "failed", target: "crates/store" },
  { id: "f2", at: "14:30:44", kind: "check", name: "test", run: 41, state: "passed" },
  { id: "f3", at: "14:31:20", kind: "check", name: "test", run: 42, state: "passed", target: "crates/clock" },
  { id: "f4", at: "14:31:50", kind: "check", name: "lint", run: 43, state: "passed", target: "web" },
  { id: "f5", at: "14:32:10", kind: "check", name: "test", run: 44, state: "running", target: "crates/net" },
];

/**
 * Runs of one Check side by side are one closed row, named for the Check and marked with the latest
 * run, never counted. A different Check between them splits the fold; a lone run is not a fold and
 * shows its target. Opened, each run is a line with its target and time, and a press opens that run.
 */
export const RepeatedCheckRunsFold: Story = {
  args: { rows: REPEATED, onOpenCheck: fn() },
  play: async ({ canvas, args }) => {
    // Three adjacent `test` runs are one fold, labelled with the latest state (passed, after a failed first run).
    const fold = canvas.getByLabelText("Check test, latest passed");
    await expect(canvas.getAllByLabelText(/^Check test, latest /)).toHaveLength(1);
    const details = fold.closest("details") as HTMLElement;
    await expect(details).not.toHaveAttribute("open");
    await expect(canvas.getByRole("button", { name: "Check test crates/store, failed" })).not.toBeVisible();

    // `lint` between them splits the fold: the runs on either side are not one row.
    const lint = canvas.getByRole("button", { name: "Check lint web, passed" });
    await expect(lint.closest("details")).toBeNull();
    const lone = canvas.getByRole("button", { name: "Check test crates/net, running" });
    await expect(lone.closest("details")).toBeNull();
    await expect(within(lone).getByText("crates/net")).toBeVisible();

    // No count is drawn: the closed row is the mark's state word and the Check's name, and no digit.
    await expect(fold.textContent).not.toMatch(/\d/);
    await expect(canvas.queryByText(/×|\b3\b|\(3\)/)).toBeNull();

    // Opened, each run is a line with its target and time; a run with no target draws none.
    await userEvent.click(fold);
    await expect(details).toHaveAttribute("open");
    const first = within(details).getByRole("button", { name: "Check test crates/store, failed" });
    await expect(within(first).getByText("crates/store")).toBeVisible();
    await expect(within(first).getByText("14:30:01")).toBeVisible();
    const whole = within(details).getByRole("button", { name: "Check test, passed" });
    await expect(whole.textContent?.replaceAll("passed", "")).toBe("14:30:44");
    await expect(within(details).getByRole("button", { name: "Check test crates/clock, passed" })).toBeVisible();
    await expect(fold.textContent).not.toMatch(/\d/);

    // Pressing a run reports that run's number.
    await userEvent.click(first);
    await expect(args.onOpenCheck).toHaveBeenLastCalledWith(40);
    await userEvent.click(whole);
    await expect(args.onOpenCheck).toHaveBeenLastCalledWith(41);
    await userEvent.click(lone);
    await expect(args.onOpenCheck).toHaveBeenLastCalledWith(44);
  },
};

/** A Session this one's agent started, or forked itself into, is a press in the thread that opens it. */
export const StartedSessionOpens: Story = {
  args: {
    rows: [
      { id: "s1", at: "14:06:00", kind: "started", session: { id: "new-1", title: "Port the clock" }, fork: false },
      { id: "s2", at: "14:07:00", kind: "started", session: { id: "new-2", title: "Try the other store" }, fork: true },
    ],
  },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Port the clock" }));
    await expect(args.onOpenSession).toHaveBeenLastCalledWith("new-1");
    await userEvent.click(canvas.getByRole("button", { name: "Try the other store" }));
    await expect(args.onOpenSession).toHaveBeenLastCalledWith("new-2");
  },
};
