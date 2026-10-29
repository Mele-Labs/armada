import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { ArrowUpToLine, Split } from "lucide-react";

import { ConcernCard, SessionThread, type SessionThreadRow, type ThreadConcern } from "./SessionThread";
import type { AdversarialReviewState, Finding } from "../AdversarialReview/AdversarialReview";
import { RequestOpen } from "./RequestedRow";

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
  { id: "1", at: "14:03:07", kind: "message", from: "you", text: "Fix the flaky store test.", sketches: [{ id: "k1", title: "Store clock" }], tags: [{ kind: "session", id: "s2", title: "Release notes script" }] },
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
    await expect(canvas.getByRole("region", { name: "Slot leased" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Open Job Release notes script" }));
    await expect(args.onOpenSession).toHaveBeenCalledWith("s2");
  },
};

/** The slot given back is a row of its own, in the style of the lease. */
export const SlotReleased: Story = {
  args: { rows: [{ id: "9", at: "14:41:02", kind: "release", slot: 3, branch: "fix/flaky-store" }] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Slot released" })).toBeInTheDocument();
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

/** Calls that ran one after another are one closed row reading as what they did, and pressing it shows each call. */
export const ToolCallsFoldIntoOneRow: Story = {
  args: { rows: WORKED },
  play: async ({ canvas }) => {
    const group = canvas.getByLabelText("Tool calls: Read, Bash").closest("details");
    await expect(group).not.toHaveAttribute("open");
    await expect(canvas.getByText("Read multiple files")).toBeVisible();
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

/** An agent message of several paragraphs keeps Prose's gap between them. */
export const AgentParagraphsAreSpaced: Story = {
  args: { rows: [{ id: "p1", at: "09:55:40", kind: "message", from: "agent", text: "The clock is pinned.\n\nThe store tests pass." }] },
  play: async ({ canvas }) => {
    const second = canvas.getByText("The store tests pass.");
    await expect(getComputedStyle(second).marginTop).not.toBe("0px");
  },
};

const YOURS: SessionThreadRow[] = [
  { id: "y1", at: "09:55:40", kind: "message", from: "agent", text: "The clock is pinned in the store tests." },
  { id: "y2", at: "09:56:12", kind: "message", from: "you", text: "Keep the pin in the test module." },
  { id: "y3", at: "09:57:23", kind: "message", from: "you", text: "Approved: http://localhost:5188/?walk=session-reply" },
  { id: "y4", at: "09:57:51", kind: "message", from: "you", text: "Re: Which date should the clock pin to?\nPin it to the first of March\nThis settles the item, so Fleet took it off your waiting list. Do not add it back." },
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
    // Fleet's line to the agent is not the person's words.
    await expect(within(answered).queryByText(/waiting list/)).toBeNull();

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

/**
 * A trailing entry is the thread's last item, after the newest row, and the thread follows it as it
 * grows for a person at the end. One who scrolled up to read is not moved.
 */
export const TrailingEntryGrowsAtTheEnd: Story = {
  args: { rows: LONG, sessionId: "s1" },
  render: (args) => {
    const [lines, setLines] = useState(1);
    return (
      <>
        <button onClick={() => setLines((now) => now + 6)}>Add words</button>
        <SessionThread
          {...args}
          trailing={
            <section aria-label="Draft turn">
              {Array.from({ length: lines }, (_, at) => (
                <p key={at}>Line {at + 1}</p>
              ))}
            </section>
          }
        />
      </>
    );
  },
  play: async ({ canvas }) => {
    const thread = canvas.getByRole("region", { name: "Thread" });
    const draft = canvas.getByRole("region", { name: "Draft turn" });
    await expect(thread.contains(draft)).toBe(true);
    await expect(canvas.getByText("Message 40").compareDocumentPosition(draft) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const atEnd = () => thread.scrollTop + thread.clientHeight >= thread.scrollHeight - 2;
    await expect(atEnd()).toBe(true);
    await userEvent.click(canvas.getByRole("button", { name: "Add words" }));
    await waitFor(() => expect(canvas.getByText("Line 7")).toBeInTheDocument());
    await waitFor(() => expect(atEnd()).toBe(true));
    thread.scrollTop = 0;
    thread.dispatchEvent(new Event("scroll"));
    await userEvent.click(canvas.getByRole("button", { name: "Add words" }));
    await waitFor(() => expect(canvas.getByText("Line 13")).toBeInTheDocument());
    await expect(thread.scrollTop).toBe(0);
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
    // Allow once is ▶ on the command's block; the rest are the answers.
    await expect(answers.getAllByRole("button").map((one) => one.textContent)).toEqual(["Allow and remember", "Refuse"]);
    await userEvent.click(answers.getByRole("button", { name: "Allow and remember" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("allow_and_remember");
    await userEvent.click(within(canvas.getByRole("group", { name: "git push origin fix/flaky-store" })).getByRole("button", { name: "Allow once" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("allow_once");
  },
};

const LIKE_OFFERS = [
  { id: "allow_once", label: "Allow once", means: "Runs it now. The next call outside its slots asks again." },
  { id: "refuse", label: "Reject once", means: "Not run. The job is told no, and the next call asks again." },
  { id: "always_allow", label: "Always allow", means: "Runs it now. Every job reads and writes in the folder chosen." },
  { id: "allow_for_session", label: "Always allow for this job", means: "Runs it now. This job reads and writes in the folder chosen." },
  { id: "allow_like_this_for_session", label: "Allow calls like this for this job", means: "Runs it, and every call of this kind until the job is closed." },
  { id: "always_block", label: "Always block", means: "Not run. Every job is refused the folder chosen." },
];

const LIKE_ASKED = {
  command: "node drive.mjs --out /Users/user/Desktop/shot.png",
  offers: LIKE_OFFERS,
  reaches: "/Users/user/Desktop/shot.png",
  reachesTo: "write" as const,
  kind: { program: "node", script: "drive.mjs" },
};

/** A Bash call reaching past its slots, with its kind: Calls like starts on the script, and its answer sends that kind. */
export const CallsLikeThisStartsOnTheScript: Story = {
  args: { rows: ROWS, asked: LIKE_ASKED },
  play: async ({ canvas, args }) => {
    const like = within(canvas.getByRole("group", { name: "Calls like" }));
    await expect(like.getAllByRole("button").map((one) => one.textContent)).toEqual(["node", "drive.mjs"]);
    await expect(like.getByRole("button", { name: "drive.mjs" })).toHaveAttribute("aria-pressed", "true");
    await expect(like.getByRole("button", { name: "node" })).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(canvas.getByRole("button", { name: "Allow calls like this for this job" }));
    await expect(args.onAnswer).toHaveBeenLastCalledWith("allow_like_this_for_session", undefined, undefined, { program: "node", script: "drive.mjs" });
  },
};

/** A press on the program widens the kind to every call of it, and Only in narrows it to the folder picked above. */
export const CallsLikeThisWidensAndNarrows: Story = {
  args: { rows: ROWS, asked: LIKE_ASKED },
  play: async ({ canvas, args }) => {
    const like = within(canvas.getByRole("group", { name: "Calls like" }));
    await userEvent.click(like.getByRole("button", { name: "node" }));
    await expect(like.getByRole("button", { name: "node" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(canvas.getByRole("button", { name: "Allow calls like this for this job" }));
    await expect(args.onAnswer).toHaveBeenLastCalledWith("allow_like_this_for_session", undefined, undefined, { program: "node" });
    await userEvent.click(within(canvas.getByRole("group", { name: "Folder for always" })).getByRole("button", { name: "Desktop" }));
    const only = canvas.getByRole("switch", { name: "Only in Desktop" });
    await expect(only).not.toBeChecked();
    await userEvent.click(only);
    await userEvent.click(canvas.getByRole("button", { name: "Allow calls like this for this job" }));
    await expect(args.onAnswer).toHaveBeenLastCalledWith("allow_like_this_for_session", undefined, "/Users/user/Desktop", { program: "node" });
  },
};

/** A kind with no script draws the program alone, already chosen. */
export const CallsLikeThisProgramAlone: Story = {
  args: { rows: ROWS, asked: { ...LIKE_ASKED, kind: { program: "rm" }, command: "rm /Users/user/Desktop/shot.png" } },
  play: async ({ canvas, args }) => {
    const like = within(canvas.getByRole("group", { name: "Calls like" }));
    await expect(like.getAllByRole("button").map((one) => one.textContent)).toEqual(["rm"]);
    await expect(like.getByRole("button", { name: "rm" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(canvas.getByRole("button", { name: "Allow calls like this for this job" }));
    await expect(args.onAnswer).toHaveBeenLastCalledWith("allow_like_this_for_session", undefined, undefined, { program: "rm" });
  },
};

/** A reach card with no kind: the folder answers, and no Calls like row. */
export const CallsLikeThisAbsentWithNoKind: Story = {
  args: { rows: ROWS, asked: (({ kind: _, ...bare }) => bare)(LIKE_ASKED) },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Folder for always" })).toBeInTheDocument();
    await expect(canvas.queryByRole("group", { name: "Calls like" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Allow calls like this for this job" })).toBeNull();
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

/** An option the agent attached a preview to shows it as a card, line breaks kept, while it is pointed at or in focus; the card goes with the pointer, and an option without one shows none. */
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
    // The card is drawn on the page, out of the panel that would clip it.
    const preview = await within(document.body).findByRole("region", { name: "Preview" });
    await expect(preview.textContent).toBe("ops: cargo test\nconsumer: pnpm test");
    await expect(getComputedStyle(preview).whiteSpace).toBe("pre");
    await userEvent.unhover(canvas.getByRole("radio", { name: "Yes" }));
    await waitFor(() => expect(within(document.body).queryByRole("region", { name: "Preview" })).toBeNull());
    await userEvent.hover(canvas.getByRole("radio", { name: "No" }));
    await expect(within(document.body).queryByRole("region", { name: "Preview" })).toBeNull();
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

/** A Job this one's agent started, or forked itself into, is a press in the thread that opens it. */
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

const NOTICE = "Subagents burned 5.0 units in your last hour, 87% of it, on opus. Pass model sonnet on reading and searching agents.";
const FLEET = { id: "fleet", title: "Fleet" };

/** **Fleet's card carries the Disputed mark once a later row disputes it**, and the row's press opens the review Job. */
export const FleetNoticeDisputed: Story = {
  name: "A Fleet notice the agent disputed",
  args: {
    rows: [
      { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: NOTICE },
      { id: "n2", at: "14:10:40", kind: "message", from: "agent", text: "My reading agents already run on sonnet. The opus spend came from building agents." },
      { id: "n3", at: "14:10:41", kind: "disputed", notice: "n1", why: "Reading agents already run on sonnet. The opus spend came from building agents.", spotted: "called", review: "review-1" },
    ],
  },
  play: async ({ canvas, args }) => {
    await expect(within(canvas.getByRole("region", { name: "Message from Fleet" })).getByRole("img", { name: "Disputed" })).toBeVisible();
    await expect(canvas.getByText("Said by the agent")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Open the review job" }));
    await expect(args.onOpenSession).toHaveBeenLastCalledWith("review-1");
  },
};

/** Read from the agent's replies, and no review started: the line draws no press and says nothing more. */
export const FleetNoticeDisputedNoReview: Story = {
  name: "A Fleet notice disputed with no review",
  args: {
    rows: [
      { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: NOTICE },
      { id: "n2", at: "14:20:41", kind: "disputed", notice: "n1", why: "Reading agents already run on sonnet.", spotted: "read" },
      { id: "n3", at: "14:21:00", kind: "message", from: "session", sender: FLEET, text: "Context is 180k tokens." },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText(/does not serve/)).toBeNull();
    await expect(canvas.getByText("Read by Fleet from the agent's replies")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Open the review job" })).toBeNull();
    // Only the notice the row names is marked.
    const disputed = canvas.getByRole("region", { name: "Message from Fleet" });
    await expect(within(disputed).getByRole("img", { name: "Disputed" })).toBeVisible();
    await expect(within(canvas.getByRole("region", { name: "Fleet turn" })).queryByRole("img", { name: "Disputed" })).toBeNull();
  },
};

/** **A right press on a notice offers three Disable choices**; a folded Fleet message that is no notice has no menu. */
export const FleetNoticeMuted: Story = {
  name: "A Fleet notice a person can turn off",
  args: {
    rows: [
      { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: "slot-4 is 23 commits behind master. Merge master into your branch before your next push.", notice: "behind_base" },
      { id: "n2", at: "14:11:00", kind: "message", from: "session", sender: FLEET, text: "Context is 180k tokens." },
    ],
    onMuteNotice: fn(),
  },
  play: async ({ canvas, canvasElement, args }) => {
    const menu = within(canvasElement.ownerDocument.body);
    const [notice, plain] = canvas.getAllByRole("region", { name: "Fleet turn" });
    await userEvent.pointer({ keys: "[MouseRight]", target: notice! });
    await expect(menu.getAllByRole("menuitem").map((one) => one.textContent)).toEqual(["For job", "For repo", "For 1 hour"]);
    await userEvent.click(menu.getByRole("menuitem", { name: "For repo" }));
    await expect(args.onMuteNotice).toHaveBeenLastCalledWith("behind_base", "repo");
    await userEvent.pointer({ keys: "[MouseRight]", target: plain! });
    await expect(menu.queryByRole("menuitem")).toBeNull();
  },
};

const FLEET_ROWS = [
  { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: "Context is 180k tokens." },
  { id: "a1", at: "14:10:30", kind: "message", from: "agent", text: "Noted, continuing." },
  { id: "n2", at: "14:20:00", kind: "message", from: "session", sender: FLEET, text: "main moved under your branch." },
] as const;

/** **A turn Fleet started folds into one row**; one still waiting on the person stays open, and folds once it is answered. */
export const FleetTurnFolds: Story = {
  name: "Fleet turns fold, the one waiting stays open",
  args: { rows: [...FLEET_ROWS] as never, waitingOnYou: true },
  play: async ({ canvas }) => {
    const [folded] = canvas.getAllByRole("region", { name: "Fleet turn" });
    await expect(canvas.getAllByRole("region", { name: "Fleet turn" })).toHaveLength(1);
    await expect(canvas.getAllByRole("region", { name: "Message from Fleet" })).toHaveLength(1);
    await expect(within(folded!).getByRole("img", { name: "Handled" })).toBeVisible();
    await userEvent.click(within(folded!).getByRole("button"));
    await expect(within(folded!).queryByText("Noted, continuing.")).toBeNull();
    await expect(canvas.getByText("Noted, continuing.")).toBeVisible();
  },
};

export const FleetTurnFoldsAfterAnswer: Story = {
  name: "Fleet turns all fold once answered",
  args: { rows: [...FLEET_ROWS] as never },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("region", { name: "Fleet turn" })).toHaveLength(2);
    await expect(canvas.queryByRole("region", { name: "Message from Fleet" })).toBeNull();
  },
};

/** **The person's message ends a Fleet turn's fold**: it and the agent's answer to it draw in the thread, not under the notice. */
export const FleetTurnEndsAtYourMessage: Story = {
  name: "Fleet turn: the person's reply is not folded",
  args: {
    rows: [
      { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: "#2734 failed its checks at 4f2a91c." },
      { id: "a1", at: "14:10:30", kind: "message", from: "agent", text: "Checking whether main is red too:" },
      { id: "a3", at: "14:11:00", kind: "message", from: "agent", text: "The failure is on main too." },
      { id: "y1", at: "15:00:00", kind: "message", from: "you", text: "I reran the build." },
      { id: "a2", at: "15:00:10", kind: "message", from: "agent", text: "I'll read the run when it ends." },
    ] as never,
  },
  play: async ({ canvas }) => {
    const folded = canvas.getByRole("region", { name: "Fleet turn" });
    await userEvent.click(within(folded).getByRole("button"));
    await expect(within(folded).getByText("Checking whether main is red too:")).toBeVisible();
    await expect(within(folded).queryByText("I reran the build.")).toBeNull();
    await expect(within(folded).queryByText("I'll read the run when it ends.")).toBeNull();
    await expect(canvas.getByText("I reran the build.")).toBeVisible();
    await expect(canvas.getByText("I'll read the run when it ends.")).toBeVisible();
  },
};

/** **The agent's last words in a Fleet turn draw in the thread**: they are meant for the person. Its words on the way stay folded with the tool runs. */
export const FleetTurnLastWordsUnfolded: Story = {
  name: "Fleet turn: the agent's last message is not folded",
  args: {
    rows: [
      { id: "n1", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: "#2734 failed its checks at 4f2a91c." },
      { id: "t1", at: "14:10:10", kind: "tool", text: "Bash gh run view" },
      { id: "a1", at: "14:10:20", kind: "message", from: "agent", text: "Checking whether main is red too:" },
      { id: "t2", at: "14:10:30", kind: "tool", text: "Bash gh run list" },
      { id: "a2", at: "14:10:40", kind: "message", from: "agent", text: "The failure is on main too, so nothing is pushed." },
    ] as never,
  },
  play: async ({ canvas }) => {
    const folded = canvas.getByRole("region", { name: "Fleet turn" });
    await expect(within(folded).getByRole("img", { name: "Handled" })).toBeVisible();
    await userEvent.click(within(folded).getByRole("button"));
    await expect(within(folded).getByText("Checking whether main is red too:")).toBeVisible();
    await expect(within(folded).queryByText("The failure is on main too, so nothing is pushed.")).toBeNull();
    await expect(canvas.getByText("The failure is on main too, so nothing is pushed.")).toBeVisible();
  },
};

/** **A folded row is named by the notice's first sentence**, marked handled once the agent replied; a turn that changed what is owed shows its last agent message outside the fold. */
export const FleetTurnSummaryAndOwed: Story = {
  name: "Fleet turn: summary, handled, owner-facing prose",
  args: {
    rows: [
      { id: "a", at: "14:00:00", kind: "message", from: "session", sender: FLEET, text: "#2734 merged into main. finish_up releases your slots." },
      { id: "b", at: "14:00:10", kind: "message", from: "agent", text: "Releasing now." },
      { id: "c", at: "14:00:20", kind: "tool", text: "armada waiting_for" },
      { id: "d", at: "14:00:30", kind: "message", from: "agent", text: "The only open item is in Needs you." },
      { id: "e", at: "14:10:00", kind: "message", from: "session", sender: FLEET, text: "slot-5's branch merged. Lease it again." },
    ] as never,
  },
  play: async ({ canvas }) => {
    const [first, second] = canvas.getAllByRole("region", { name: "Fleet turn" });
    await expect(within(first!).getByRole("button", { name: /#2734 merged into main/ })).toBeVisible();
    await expect(within(first!).getByRole("img", { name: "Handled" })).toBeVisible();
    await expect(within(first!).queryByText("The only open item is in Needs you.")).toBeNull();
    await expect(canvas.getByText("The only open item is in Needs you.")).toBeVisible();
    await expect(within(second!).queryByRole("img", { name: "Handled" })).toBeNull();
  },
};

/** The page's clipboard, replaced by one that keeps what it was given, and the way back. */
function stubClipboard() {
  const written: string[] = [];
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
  });
  return { written, restore: () => void Reflect.deleteProperty(navigator, "clipboard") };
}

/** A pointer selection as the page hears it: a press on `at`, the words selected, the button let go. */
function selectWith(words: Node, at: Element) {
  press(at);
  window.getSelection()?.selectAllChildren(words);
  release(at);
}

/** Raw mouse events, since a pointer helper moves the selection on its own at both ends. */
const press = (at: Element) => void at.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 }));
const release = (at: Element) => void at.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, button: 0 }));

/** **Words selected in the thread go to the clipboard when the selection ends**, with or without a Fork bar, and the selection stays. */
export const SelectedWordsAreCopied: Story = {
  args: { rows: ROWS },
  play: async ({ canvas }) => {
    const { written, restore } = stubClipboard();
    try {
      const words = canvas.getByText("The test reads the wall clock.");
      selectWith(words, words);
      await expect(written).toEqual(["The test reads the wall clock."]);
      await expect(window.getSelection()?.toString()).toBe("The test reads the wall clock.");
    } finally {
      restore();
    }
  },
};

export const SelectedWordsAreCopiedBesideTheForkBar: Story = {
  args: { rows: ROWS, onForkAt: fn() },
  play: async ({ canvas }) => {
    const { written, restore } = stubClipboard();
    try {
      const words = canvas.getByText("The test reads the wall clock.");
      selectWith(words, words);
      await expect(written).toEqual(["The test reads the wall clock."]);
      await waitFor(() => expect(within(document.body).getByRole("toolbar", { name: "Selection" })).toBeVisible());
      await expect(window.getSelection()?.toString()).toBe("The test reads the wall clock.");
    } finally {
      restore();
    }
  },
};

/** What does not happen: no copy for a press with no selection, a selection outside the thread, or words in a field the person types in. */
export const NothingElseIsCopied: Story = {
  args: { rows: ROWS },
  play: async ({ canvas, canvasElement }) => {
    const { written, restore } = stubClipboard();
    const thread = canvasElement.querySelector(".armada-session-thread")!;
    const field = document.createElement("textarea");
    field.value = "typed words";
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    editable.textContent = "edited words";
    const outside = document.createElement("p");
    outside.textContent = "words outside";
    thread.append(field, editable);
    document.body.append(outside);
    try {
      const message = canvas.getByText("The test reads the wall clock.");
      // A press that selects nothing.
      window.getSelection()?.removeAllRanges();
      press(message);
      release(message);
      // Words outside the thread.
      selectWith(outside, message);
      // Words in a textarea, which holds the focus after they are selected.
      press(message);
      field.focus();
      field.select();
      release(message);
      // Words in a contenteditable.
      selectWith(editable, message);
      await expect(written).toEqual([]);
    } finally {
      field.remove();
      editable.remove();
      outside.remove();
      restore();
    }
  },
};

const LIVE_PLAN = {
  version: 2,
  approved: false,
  groups: [
    { id: "q1", agent: "rust-engineer", title: "Keep the queue in the store", tasks: ["Add the queue table", "Read it back when Fleet starts", "Take a Session off it when a bay is free"] },
    { id: "q2", agent: "protocol-engineer", title: "Queue operation on the wire", tasks: ["Add queue_session", "Mirror it in the TypeScript wire"] },
    { id: "q3", agent: "bridge-engineer", title: "Queue from the dispatch panel", tasks: ["Offer Queue beside Start", "Show a queued Session in the list", "Let him take one off the queue"] },
    { id: "q4", agent: "general-purpose", title: "Document the queue", tasks: ["Write the queue's place in the Session concept", "Say what a restart keeps"] },
  ].map((group) => ({ ...group, tasks: group.tasks.map((text, at) => ({ id: `${group.id}-${at}`, text })) })),
  onApprove: fn(),
  onReview: fn(),
};
const LIVE_ROWS: SessionThreadRow[] = [
  { id: "l1", at: "14:03:00", kind: "message", from: "you", text: "Queue a Session for later, so it starts when a bay is free." },
  { id: "l2", at: "14:05:10", kind: "message", from: "agent", text: "The plan is recorded: ten tasks in four groups. Approve it, or tell me what to change." },
  { id: "l3", at: "14:08:40", kind: "message", from: "you", text: "Update the plan: the queue should survive a restart." },
];

/** A plan with four groups and ten tasks pending, beside a permission: they are the thread's last card, after its last message, and the thread scrolls to them. */
export const WhatWaitsIsTheThreadsLastCard: Story = {
  args: {
    rows: LIVE_ROWS,
    approval: LIVE_PLAN,
    asked: { command: "cargo test -p fleet queue" },
  },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 7)", width: "calc(var(--space-12) * 10)", background: "var(--bg-raised)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas }) => {
    const thread = canvas.getByRole("region", { name: "Thread" });
    const last = canvas.getByText("Update the plan: the queue should survive a restart.");
    const tail = within(thread).getByRole("listitem", { name: "Waiting on you" });
    await expect(thread.contains(tail)).toBe(true);
    await expect(last.compareDocumentPosition(tail) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await expect(canvas.queryByRole("region", { name: "Pinned calls" })).toBeNull();
    await expect(within(tail).getByRole("article", { name: "Plan to approve" })).toBeInTheDocument();
  },
};

/** A turn running behind a waiting plan marks the thread's end below the card, beside the composer, not on a calls row above it. */
export const WorkingMarkBelowTheWaitingCard: Story = {
  args: {
    rows: [...LIVE_ROWS, { id: "l4", at: "14:09:00", kind: "tool", text: "Read crates/fleet/src/queue.rs" }, { id: "l5", at: "14:09:02", kind: "tool", text: "Bash cargo check -p fleet" }],
    approval: LIVE_PLAN,
    working: true,
  },
  play: async ({ canvas }) => {
    const tail = canvas.getByRole("listitem", { name: "Waiting on you" });
    const mark = canvas.getByRole("img", { name: "Working" });
    await expect(tail.compareDocumentPosition(mark) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  },
};

const PAIRING_PR = { number: 2814, title: "Pairing route in Fleet", state: "open" as const };

/** A Job that keeps its work local opened no pull request, so its plan draws no chip. */
export const KeepLocalPlanDrawsNoChip: Story = {
  args: { rows: LIVE_ROWS, approval: LIVE_PLAN },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Keep the queue in the store")).toBeVisible();
    await expect(canvas.queryByRole("group", { name: /not opened yet/ })).toBeNull();
    await expect(canvas.queryByRole("region", { name: /^Lands in/ })).toBeNull();
    await expect(canvas.queryByRole("button", { name: /^Pull request #/ })).toBeNull();
  },
};

/** An Auto land Job with a one-PR plan draws one placeholder, PR 1, over every group. */
export const AutoLandPlanDrawsOnePlaceholder: Story = {
  args: { rows: LIVE_ROWS, approval: { ...LIVE_PLAN, opensPullRequests: true } },
  play: async ({ canvas }) => {
    const chips = canvas.getAllByRole("group", { name: "PR 1, not opened yet" });
    await expect(chips).toHaveLength(1);
    const landing = canvas.getByRole("region", { name: "Lands in PR 1" });
    await expect(within(landing).getByText("Keep the queue in the store")).toBeVisible();
    await expect(within(landing).getByText("Document the queue")).toBeVisible();
  },
};

/** Groups sharing a label stand under one chip: the placeholder takes the real pull request once one of them has it, and a group with no landsIn is PR 1. */
export const GroupsSharingALabelStandUnderOneChip: Story = {
  args: {
    rows: LIVE_ROWS,
    approval: {
      ...LIVE_PLAN,
      opensPullRequests: true,
      onOpenPullRequest: fn(),
      groups: [
        { ...LIVE_PLAN.groups[0]!, landsIn: { label: "PR 1" } },
        { ...LIVE_PLAN.groups[1]!, landsIn: { label: "PR 1", pullRequest: PAIRING_PR } },
        LIVE_PLAN.groups[2]!,
        { ...LIVE_PLAN.groups[3]!, landsIn: { label: "PR 2" } },
      ],
    },
  },
  play: async ({ canvas, args }) => {
    const first = canvas.getByRole("region", { name: "Lands in #2814" });
    for (const title of ["Keep the queue in the store", "Queue operation on the wire", "Queue from the dispatch panel"]) await expect(within(first).getByText(title)).toBeVisible();
    await expect(within(first).queryByText("Document the queue")).toBeNull();
    await expect(canvas.queryByRole("group", { name: "PR 1, not opened yet" })).toBeNull();
    const second = canvas.getByRole("region", { name: "Lands in PR 2" });
    await expect(within(second).getByText("Document the queue")).toBeVisible();
    await expect(within(second).getByRole("group", { name: "PR 2, not opened yet" })).toBeVisible();
    await userEvent.click(within(first).getByRole("button", { name: "Pull request #2814 Pairing route in Fleet" }));
    await expect(args.approval!.onOpenPullRequest).toHaveBeenCalledWith(2814);
  },
};

/** The plan card folds on its band to one line: Plan, its version and its state, with Approve still on it. */
export const PlanFoldsToOneLine: Story = {
  args: { rows: LIVE_ROWS, approval: LIVE_PLAN },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByText("Keep the queue in the store")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Plan" }));
    await expect(canvas.queryByText("Keep the queue in the store")).toBeNull();
    await expect(canvas.getByText("v2")).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Plan waiting for approval" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Approve" }));
    await expect(args.approval!.onApprove).toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Plan" }));
    await expect(canvas.getByText("Keep the queue in the store")).toBeVisible();
  },
};

const PLAIN_PLAN = {
  ...LIVE_PLAN,
  summary: "He can queue a Session for later, and it starts when a bay is free.",
  groups: LIVE_PLAN.groups.map((group, at) => ({ ...group, gives: ["A queue that survives a restart", "Fleet takes a Session off the queue", "A Queue press in the dispatch panel", "The queue written down"][at]! })),
  onSetAside: fn(),
};

/** A plan with a summary reads first: the summary, a line of what each group gives, and the groups under Details. */
export const PlanReadsSummaryThenGivesThenDetails: Story = {
  args: { rows: LIVE_ROWS, approval: PLAIN_PLAN },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("He can queue a Session for later, and it starts when a bay is free.")).toBeVisible();
    const gives = canvas.getByRole("list", { name: "What each group gives" });
    await expect(within(gives).getAllByRole("listitem").map((one) => one.textContent)).toEqual(["A queue that survives a restart", "Fleet takes a Session off the queue", "A Queue press in the dispatch panel", "The queue written down"]);
    await expect(canvas.queryByText("Keep the queue in the store")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Details" }));
    await expect(canvas.getByText("Keep the queue in the store")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Show less" }));
    await expect(canvas.queryByText("Keep the queue in the store")).toBeNull();
  },
};

/** Not yet tells the agent to keep talking; the plan then draws folded to a `Plan set aside` chip with nothing to press but its own band. */
export const NotYetSetsThePlanAside: Story = {
  args: { rows: LIVE_ROWS, approval: PLAIN_PLAN },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Not yet" }));
    await expect(args.approval!.onSetAside).toHaveBeenCalled();
  },
};

/** A plan he set aside arrives folded to a chip: no Approve, no Not yet, no groups, until a press on the chip unfolds it. */
export const SetAsidePlanDrawsFolded: Story = {
  args: { rows: LIVE_ROWS, approval: { ...PLAIN_PLAN, setAside: true } },
  play: async ({ canvas }) => {
    const chip = canvas.getByRole("button", { name: "Plan set aside" });
    await expect(chip).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.getByRole("article", { name: "Plan set aside" })).toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Approve" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Not yet" })).toBeNull();
    await expect(canvas.queryByText("He can queue a Session for later, and it starts when a bay is free.")).toBeNull();
    await userEvent.click(chip);
    await expect(canvas.getByText("He can queue a Session for later, and it starts when a bay is free.")).toBeVisible();
  },
};

/** An adversarial review's findings, each about a part of `PLAIN_PLAN`: a task, a group, the whole plan, and a task the plan no longer holds. */
const FINDINGS: readonly Finding[] = [
  { id: "a1", about: "q1-1", consequence: "A restart drops the Sessions queued while Fleet was down", expected: "The queue is read back whole", found: "It reads the table before the store has opened it" },
  { id: "a2", about: "q3", consequence: "A queued Session cannot be told apart from a running one", expected: "A queued row reads as waiting", found: "Both draw the same mark" },
  { id: "a3", consequence: "Nothing proves the queue survives a restart", expected: "A test restarts Fleet with a Session queued", found: "No task adds one" },
  { id: "a4", about: "gone-task", consequence: "Two Fleets can take the same Session off the queue", expected: "One takes it, the other sees it gone", found: "Nothing holds the row while it is taken" },
];

/** The plan card with a review's findings pinned to it, settled as Fleet would settle them: a press folds a finding to a line. */
function Reviewed({ review: first, onRun, onSend, onDismiss }: { review: AdversarialReviewState; onRun?: () => void; onSend?: (id: string) => void; onDismiss?: (id: string) => void }) {
  const [review, setReview] = useState(first);
  const settle = (id: string, settled: "sent" | "dismissed") =>
    setReview((was) => (was.state === "done" ? { ...was, findings: was.findings.map((one) => (one.id === id ? { ...one, settled } : one)) } : was));
  return (
    <SessionThread
      rows={LIVE_ROWS}
      onAnswer={() => undefined}
      onOpenSession={() => undefined}
      approval={{
        ...PLAIN_PLAN,
        adversarial: {
          review,
          onRun: onRun ?? (() => undefined),
          onSend: (id) => (onSend?.(id), settle(id, "sent")),
          onDismiss: (id) => (onDismiss?.(id), settle(id, "dismissed")),
          shortcut: "r",
        },
      }}
    />
  );
}

/** A finding sits under the task it is about, a group holding one carries its mark, and one about the whole plan, or a part it no longer holds, sits under the summary. */
export const AdversarialFindingsPinnedToThePlan: Story = {
  render: () => <Reviewed review={{ state: "done", findings: FINDINGS }} />,
  play: async ({ canvas }) => {
    const task = canvas.getByRole("list", { name: "Findings on Read it back when Fleet starts" });
    await expect(within(task).getByRole("group", { name: "Finding: A restart drops the Sessions queued while Fleet was down" })).toBeVisible();
    const group = canvas.getByRole("list", { name: "Findings on Queue from the dispatch panel" });
    await expect(within(group).getByRole("group", { name: "Finding: A queued Session cannot be told apart from a running one" })).toBeVisible();
    // The mark at the end of a group's line: on the two groups that hold findings, and on no other.
    await expect(canvas.getByRole("button", { name: "Findings on A queue that survives a restart" })).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByRole("button", { name: "Findings on A Queue press in the dispatch panel" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Findings on The queue written down" })).toBeNull();
    const whole = canvas.getByRole("list", { name: "Findings on the plan" });
    await expect(within(whole).getAllByRole("group", { name: /^Finding: / }).map((one) => one.getAttribute("aria-label"))).toEqual([
      "Finding: Nothing proves the queue survives a restart",
      "Finding: Two Fleets can take the same Session off the queue",
    ]);
  },
};

/** Send to agent and Dismiss fold a finding to one muted line in place, by press and by the registry's keys, and focus passes to the next. */
export const SendAndDismissFoldAFinding: Story = {
  render: (args) => <Reviewed review={{ state: "done", findings: FINDINGS }} onSend={args.approval?.adversarial?.onSend} onDismiss={args.approval?.adversarial?.onDismiss} />,
  args: { approval: { ...PLAIN_PLAN, adversarial: { review: { state: "idle" }, onRun: fn(), onSend: fn(), onDismiss: fn() } } },
  play: async ({ canvas, args }) => {
    const lost = canvas.getByRole("group", { name: "Finding: A restart drops the Sessions queued while Fleet was down" });
    await userEvent.click(within(lost).getByRole("button", { name: "Send to agent" }));
    await expect(args.approval!.adversarial!.onSend).toHaveBeenCalledWith("a1");
    await expect(canvas.getByRole("group", { name: "Finding sent: A restart drops the Sessions queued while Fleet was down" })).toBeVisible();
    await expect(canvas.queryByRole("group", { name: "Finding: A restart drops the Sessions queued while Fleet was down" })).toBeNull();
    const next = canvas.getByRole("group", { name: "Finding: Nothing proves the queue survives a restart" });
    next.focus();
    await userEvent.keyboard("d");
    await expect(args.approval!.adversarial!.onDismiss).toHaveBeenCalledWith("a3");
    await expect(canvas.getByRole("group", { name: "Finding dismissed: Nothing proves the queue survives a restart" })).toBeVisible();
    await waitFor(() => expect(canvas.getByRole("group", { name: "Finding: Two Fleets can take the same Session off the queue" })).toHaveFocus());
  },
};

/** A review of an older version keeps its findings on the plan, says which version it read, and offers Run again, by press and by its key. */
export const StaleReviewOffersRunAgain: Story = {
  render: (args) => <Reviewed review={{ state: "stale", read: 1, findings: FINDINGS }} onRun={args.approval?.adversarial?.onRun} />,
  args: { approval: { ...PLAIN_PLAN, adversarial: { review: { state: "idle" }, onRun: fn(), onSend: fn(), onDismiss: fn() } } },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("img", { name: "Read v1" })).toBeInTheDocument();
    await expect(canvas.getByRole("group", { name: "Finding: Nothing proves the queue survives a restart" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Run again" }));
    await expect(args.approval!.adversarial!.onRun).toHaveBeenCalledTimes(1);
    canvas.getByRole("article", { name: "Plan to approve" }).focus();
    await userEvent.keyboard("r");
    await expect(args.approval!.adversarial!.onRun).toHaveBeenCalledTimes(2);
  },
};

/** The cards a judge's concerns draw, as the Needs you panel holds them: ConcernCard alone, since the thread no longer draws them. */
function Concerns({ concerns }: { concerns: readonly ThreadConcern[] }) {
  return (
    <div style={{ width: "calc(var(--space-12) * 10)", background: "var(--bg-raised)" }}>
      {concerns.map((one) => (
        <ConcernCard key={one.id} concern={one} onSend={fn()} onDismiss={fn()} />
      ))}
    </div>
  );
}

/** A judge's concern says what it is: the glyph and Judge in its band, where it came from, the concern, its cites as chips, and Send says whose message it goes as. */
export const JudgeConcernSaysWhatItIs: Story = {
  args: { rows: LIVE_ROWS },
  render: () => <Concerns concerns={[
    { id: "c1", step: "Build", text: "The queue has no ceiling.", cites: ["crates/fleet/src/queue.rs:12", "crates/fleet/src/pool.rs"] },
    { id: "c2", text: "A queued Session that is taken off is not told." },
  ]} />,
  play: async ({ canvas }) => {
    const card = canvas.getByRole("group", { name: "Judge: The queue has no ceiling." });
    await expect(within(card).getByRole("img", { name: "Judge" })).toBeInTheDocument();
    await expect(card.querySelector(".armada-session-thread__eyebrow")?.textContent).toBe("Judge");
    await expect(within(card).getByText("Build")).toBeInTheDocument();
    await expect(within(card).getByText("crates/fleet/src/queue.rs:12")).toBeInTheDocument();
    await expect(within(canvas.getByRole("group", { name: "Judge: A queued Session that is taken off is not told." })).getByText("Judge tile")).toBeInTheDocument();
    await userEvent.hover(within(card).getByRole("button", { name: "Send to job" }));
    await expect((await canvas.findAllByText("Goes to the agent as your message")).length).toBeGreaterThan(0);
  },
};

/**
 * A concern with its parts is a claim: the consequence as the headline, then Expected and Found, each clamped to two lines with More
 * for the rest, so a row that fits has no More. A concern kept as text alone is its lines, in the same prose.
 */
export const JudgeConcernIsAClaim: Story = {
  args: { rows: LIVE_ROWS },
  render: () => (
    <Concerns
      concerns={[
        {
          id: "c1",
          text: "Expected: x\nProduced: y\nConsequence: z",
          consequence: "Nothing in the dispatch panel reflects the feature, so a person starting a Session from it cannot tell it exists.",
          expected: "A diff touching dispatch-panel that adds the queue act and the keys that start it, with a test beside each change, the registry rows those keys read from, and the generated vocabulary rebuilt from them, so the panel offers Queue wherever it offers Start.",
          produced: "No file has changed yet.",
          cites: ["apps/desktop/src/renderer/src/dispatch-panel/dispatching.tsx"],
        },
      ]}
    />
  ),
  play: async ({ canvas }) => {
    const card = canvas.getByRole("group", { name: /^Judge: / });
    await expect(within(card).getByText(/^Nothing in the dispatch panel reflects the feature/)).toBeVisible();
    await expect(within(card).getByText("Expected")).toBeVisible();
    await expect(within(card).getByText("Found")).toBeVisible();
    await expect(within(card).getByText("No file has changed yet.")).toBeVisible();
    // Only the long row is clamped, and More opens it whole.
    await expect(within(card).getAllByRole("button", { name: /^More of / })).toHaveLength(1);
    const long = within(card).getByText(/^A diff touching dispatch-panel/);
    const clamped = long.getBoundingClientRect().height;
    await userEvent.click(within(card).getByRole("button", { name: "More of expected" }));
    await expect(long.getBoundingClientRect().height).toBeGreaterThan(clamped);
    await userEvent.click(within(card).getByRole("button", { name: "Less of expected" }));
    await expect(long.getBoundingClientRect().height).toBe(clamped);
  },
};

/** A concern kept before its parts were is its text, on its lines, in prose. */
export const JudgeConcernWithoutPartsIsItsLines: Story = {
  args: { rows: LIVE_ROWS },
  render: () => <Concerns concerns={[{ id: "c1", text: "Expected: a test beside the change\nProduced: none\nConsequence: the change is unchecked" }]} />,
  play: async ({ canvas }) => {
    const card = canvas.getByRole("group", { name: /^Judge: / });
    await expect(within(card).getByText("Consequence: the change is unchecked")).toBeVisible();
    await expect(within(card).queryByRole("button", { name: /^More of / })).toBeNull();
  },
};

/** A re-recorded plan's rename stacks: the old text struck through, and the new text on the line below it. */
export const RenameStacksOldOverNew: Story = {
  args: {
    rows: LIVE_ROWS,
    approval: { ...LIVE_PLAN, changes: [{ kind: "renamed", from: "Add the queue table", text: "Add the queue table and its index" }] },
  },
  play: async ({ canvas }) => {
    const old = canvas.getByText("Add the queue table");
    const now = canvas.getByText("Add the queue table and its index");
    await expect(old.tagName).toBe("S");
    await expect(now.getBoundingClientRect().top).toBeGreaterThanOrEqual(old.getBoundingClientRect().bottom - 1);
  },
};

const task = (id: string, text: string, state: "open" | "working" | "done", since?: { kind: "added" } | { kind: "renamed"; from: string }) => ({ id, text, state, ...(since === undefined ? {} : { since }) });

/** Plan v5 over a lot of finished work: two groups done, one in progress with a task added, one renamed and one taken out. */
const REVISED_PLAN = {
  version: 5,
  approved: false,
  groups: [
    { id: "w1", agent: "rust-engineer", title: "Warm a bay before it is leased", tasks: [{ id: "W1", text: "Find the next free bay" }, { id: "W2", text: "Warm it in the background" }] },
    { id: "w2", agent: "protocol-engineer", title: "Warm state on the wire", tasks: [{ id: "W4", text: "Add warming to the bay reply" }] },
    { id: "w3", agent: "bridge-engineer", title: "Show warming on the bay tile", tasks: [{ id: "W6", text: "Draw a warming bay" }, { id: "W7", text: "Say how long the warm took" }, { id: "W8", text: "Show warming in the bay pane" }] },
  ],
  revision: {
    groups: [
      { id: "w1", agent: "rust-engineer", title: "Warm a bay before it is leased", gone: [], tasks: [task("W1", "Find the next free bay", "done"), task("W2", "Warm it in the background", "done")] },
      { id: "w2", agent: "protocol-engineer", title: "Warm state on the wire", gone: [], tasks: [task("W4", "Add warming to the bay reply", "done")] },
      {
        id: "w3",
        agent: "bridge-engineer",
        title: "Show warming on the bay tile",
        gone: ["Add a warm button to the bay tile"],
        tasks: [task("W6", "Draw a warming bay", "done"), task("W7", "Say how long the warm took", "working", { kind: "renamed", from: "Show how long warming took" }), task("W8", "Show warming in the bay pane", "open", { kind: "added" })],
      },
    ],
  },
  onApprove: fn(),
  onReview: fn(),
};

/** A later version reads as its diff: done groups nothing touched are one closed line each, and the changed group is open with its added, renamed and removed tasks marked where they stand. */
export const RevisionClosesWhatDidNotChange: Story = {
  args: { rows: LIVE_ROWS, approval: REVISED_PLAN },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Plan to approve" });
    for (const title of ["Warm a bay before it is leased", "Warm state on the wire"]) {
      await expect(within(card).getByRole("button", { name: title })).toHaveAttribute("aria-expanded", "false");
    }
    await expect(within(card).queryByText("Find the next free bay")).toBeNull();
    await expect(within(card).queryByText("Add warming to the bay reply")).toBeNull();
    await expect(within(card).getByText("Show warming on the bay tile")).toBeVisible();
    await expect(within(card).getByText("Draw a warming bay")).toBeVisible();
    await expect(within(card).getByRole("img", { name: "Added" })).toBeVisible();
    await expect(within(card).getByRole("img", { name: "Renamed" })).toBeVisible();
    await expect(within(card).getByRole("img", { name: "Removed" })).toBeVisible();
    await expect(within(card).getByText("Show how long warming took").tagName).toBe("S");
    await expect(within(card).getByText("Say how long the warm took")).toBeVisible();
    await expect(within(card).getByText("Add a warm button to the bay tile").tagName).toBe("S");
    await expect(within(card).getByText("Show warming in the bay pane")).toBeVisible();
    // The old card's changes list is not drawn beside it.
    await expect(within(card).queryByRole("list", { name: "Changed since the last plan" })).toBeNull();
  },
};

/** U opens every closed group and closes them all again, from the card or from a control on it. */
export const UOpensAndClosesTheClosedGroups: Story = {
  args: { rows: LIVE_ROWS, approval: REVISED_PLAN },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Plan to approve" });
    card.focus();
    await userEvent.keyboard("u");
    await expect(within(card).getByText("Find the next free bay")).toBeVisible();
    await expect(within(card).getByText("Add warming to the bay reply")).toBeVisible();
    await expect(within(card).getByRole("button", { name: "Warm state on the wire" })).toHaveAttribute("aria-expanded", "true");
    await userEvent.keyboard("u");
    await expect(within(card).queryByText("Find the next free bay")).toBeNull();
    await expect(within(card).queryByText("Add warming to the bay reply")).toBeNull();
    // With one open already, U opens the other rather than closing the one.
    within(card).getByRole("button", { name: "Warm state on the wire" }).focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard("u");
    await expect(within(card).getByText("Find the next free bay")).toBeVisible();
    await userEvent.keyboard("u");
    await expect(within(card).queryByText("Add warming to the bay reply")).toBeNull();
  },
};

/** Enter on a closed group opens just that group, and Enter again closes it. */
export const EnterOpensOneClosedGroup: Story = {
  args: { rows: LIVE_ROWS, approval: REVISED_PLAN },
  play: async ({ canvas, args }) => {
    const card = canvas.getByRole("article", { name: "Plan to approve" });
    const group = within(card).getByRole("button", { name: "Warm a bay before it is leased" });
    group.focus();
    await userEvent.keyboard("{Enter}");
    await expect(group).toHaveAttribute("aria-expanded", "true");
    await expect(within(card).getByText("Find the next free bay")).toBeVisible();
    await expect(within(card).queryByText("Add warming to the bay reply")).toBeNull();
    // A press on a group's line is not a press on the card: the plan stays where it is.
    await expect(args.approval!.onReview).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await expect(group).toHaveAttribute("aria-expanded", "false");
    await expect(within(card).queryByText("Find the next free bay")).toBeNull();
  },
};

/** A first plan, or a later one with nothing changed, draws as it always did: every group a row with a mark for each task, nothing to open, and U does nothing. */
export const PlanWithNoChangesDrawsWhole: Story = {
  args: { rows: LIVE_ROWS, approval: LIVE_PLAN },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Plan to approve" });
    await expect(within(card).getByText("Keep the queue in the store")).toBeVisible();
    await expect(within(card).getByText("Document the queue")).toBeVisible();
    await expect(within(card).queryByRole("button", { name: "Keep the queue in the store" })).toBeNull();
    await expect(within(card).queryByRole("list", { name: "Plan" })).toBeNull();
    card.focus();
    await userEvent.keyboard("u");
    await expect(within(card).getByText("Keep the queue in the store")).toBeVisible();
    await expect(within(card).queryByText("Add the queue table")).toBeNull();
  },
};

const REQUESTED: SessionThreadRow[] = [
  { id: "q1", at: "15:02:10", kind: "requested", by: "agent", method: "POST", url: "https://api.example.test/graphql", environment: "staging", headers: [{ name: "Content-Type", value: "application/json" }], body: '{"query":"{ me { id } }"}', status: 401, ms: 184, response: '{"errors":[{"message":"Not signed in"}]}', truncated: false },
  { id: "q2", at: "15:04:30", kind: "requested", by: "owner", method: "POST", url: "https://api.example.test/graphql", environment: "staging", headers: [{ name: "Content-Type", value: "application/json" }], body: '{"query":"{ me { id } }"}', status: 200, ms: 212, response: '{"data":{"me":{"id":"u_7"}}}', truncated: false },
  { id: "q3", at: "15:05:02", kind: "requested", by: "agent", method: "GET", url: "http://127.0.0.1:9/health", headers: [], status: null, ms: 3, error: "connection refused", response: "", truncated: false },
];

const opened = fn();

/** A request the agent or the person sent: who, how it was answered, the method, the address and the time; the answer's body under the fold, and a press back into Requests. */
export const RequestsSent: Story = {
  args: { rows: REQUESTED, agentName: "Agent" },
  decorators: [
    (Story) => (
      <RequestOpen.Provider value={opened}>
        <Story />
      </RequestOpen.Provider>
    ),
  ],
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "No answer: connection refused" })).toBeInTheDocument();
    const head = canvas.getByLabelText("Agent: 401 POST https://api.example.test/graphql");
    const row = within(head.closest("li")!);
    await userEvent.click(head);
    await expect(row.getByRole("log", { name: "Response" })).toBeVisible();
    await expect(row.getByRole("log", { name: "Response" })).toHaveTextContent('{"errors":[{"message":"Not signed in"}]}');
    await userEvent.click(row.getByRole("button", { name: "Open in Requests" }));
    await expect(opened).toHaveBeenCalledWith(expect.objectContaining({ id: "q1", body: '{"query":"{ me { id } }"}' }));
  },
};

const WITH_DEBUG: SessionThreadRow[] = [
  { id: "d1", at: "14:03:07", kind: "message", from: "you", text: "The title is cut off in the row.\n\n## Debug info\nJob: Fix the flaky store test (01SESSION)\nState: working" },
  { id: "d2", at: "14:03:14", kind: "message", from: "you", text: "Nothing is under this one." },
];

/** A message holding a Job's facts shows the words and folds the facts under Debug info; one without is as it was. */
export const DebugInfoFolded: Story = {
  args: { rows: WITH_DEBUG },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The title is cut off in the row.")).toBeVisible();
    await expect(canvas.getByText(/State: working/)).not.toBeVisible();
    await expect(canvas.queryByText("## Debug info")).toBeNull();
    await expect(canvas.getAllByText("Debug info")).toHaveLength(1);
    await userEvent.click(canvas.getByText("Debug info"));
    await expect(canvas.getByText(/State: working/)).toBeVisible();
  },
};

const PIXEL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/** A sent picture is a thumbnail that opens larger; a file with no picture stays a chip. */
export const SentPicture: Story = {
  args: {
    rows: [
      {
        id: "p1",
        at: "14:03:07",
        kind: "message",
        from: "you",
        text: "See these.",
        files: [
          { id: "f1", name: "login-error.png", src: PIXEL },
          { id: "f2", name: "notes.txt" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "login-error.png" }));
    await expect(await within(document.body).findByRole("dialog", { name: "login-error.png" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "notes.txt" })).toBeNull();
  },
};

const DRAFT_TEXT = "## Queue a Job for later\n\nA Job waits for a free slot and starts when one opens.\n\n- The queue survives a restart\n- Queue sits beside Start";
const DRAFT: SessionThreadRow = {
  id: "d1",
  at: "14:06:00",
  kind: "draft",
  title: "Description for the queue pull request",
  text: DRAFT_TEXT,
  before: "Adds a queue.",
};
const DRAFT_ROWS: SessionThreadRow[] = [{ id: "d0", at: "14:05:00", kind: "message", from: "agent", text: "The description is rewritten." }, DRAFT];
const drafted = (row: Partial<Extract<SessionThreadRow, { kind: "draft" }>>): SessionThreadRow[] => [DRAFT_ROWS[0]!, { ...(DRAFT as Extract<SessionThreadRow, { kind: "draft" }>), ...row }];

/** A draft the agent put to him: the eyebrow, its title, the text as prose, and Approve, Edit, Note and Withdraw on the card. */
export const DraftOpen: Story = {
  args: { rows: DRAFT_ROWS, onDraft: fn(), onWithdrawDraft: fn() },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Draft: Description for the queue pull request" });
    await expect(within(card).getByText("Draft")).toBeVisible();
    await expect(within(card).getByText("Description for the queue pull request")).toBeVisible();
    await expect(within(card).getByText("Queue a Job for later")).toBeVisible();
    await expect(within(card).getByText("The queue survives a restart")).toBeVisible();
    const answers = within(within(card).getByRole("group", { name: "Answers" })).getAllByRole("button");
    await expect(answers.map((one) => one.textContent)).toEqual(["Approve", "Edit", "Note", "Withdraw"]);
  },
};

/** Withdraw ends the draft with no reason given. */
export const DraftWithdraw: Story = {
  args: { rows: drafted({ id: "d9" }), onDraft: fn(), onWithdrawDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Withdraw" }));
    await expect(args.onWithdrawDraft).toHaveBeenCalledWith("d9", undefined);
    await expect(args.onDraft).not.toHaveBeenCalled();
  },
};

/** In the note, Withdraw is last as it was in read mode and takes his note as the reason; Send back is as it was. */
export const DraftNoteWithdraws: Story = {
  args: { rows: drafted({ id: "d10" }), onDraft: fn(), onWithdrawDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Note" }));
    const answers = within(canvas.getByRole("group", { name: "Answers" })).getAllByRole("button");
    await expect(answers.map((one) => one.textContent)).toEqual(["Send back", "Cancel", "Withdraw"]);
    await userEvent.type(canvas.getByRole("textbox", { name: "Note" }), "  The pull request is closed.  ");
    await userEvent.click(canvas.getByRole("button", { name: "Withdraw" }));
    await expect(args.onWithdrawDraft).toHaveBeenCalledWith("d10", "The pull request is closed.");
    await expect(args.onDraft).not.toHaveBeenCalled();
  },
};

/** The text a draft replaces is folded away, and a press shows it. */
export const DraftShowsWhatItReplaces: Story = {
  args: { rows: DRAFT_ROWS, onDraft: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Adds a queue.")).toBeNull();
    const press = canvas.getByRole("button", { name: "Text it replaces" });
    await expect(press).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(press);
    await expect(press).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText("Adds a queue.")).toBeVisible();
  },
};

/** Approve takes the draft as written. */
export const DraftApprove: Story = {
  args: { rows: DRAFT_ROWS, onDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Approve" }));
    await expect(args.onDraft).toHaveBeenCalledWith("d1", { choice: 0 });
  },
};

/** Edit puts the text in a box; Approve with fixes sends his whole text back as the fixed draft. */
export const DraftEditThenApproveWithFixes: Story = {
  args: { rows: drafted({ id: "d2" }), onDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Edit" }));
    const box = canvas.getByRole("textbox", { name: "Draft text" });
    await expect(box).toHaveValue(DRAFT_TEXT);
    await expect(canvas.queryByText("Queue a Job for later")).toBeNull();
    const answers = within(canvas.getByRole("group", { name: "Answers" })).getAllByRole("button");
    await expect(answers.map((one) => one.textContent)).toEqual(["Approve with fixes", "Cancel"]);
    await userEvent.clear(box);
    await userEvent.type(box, "A Job waits for a free slot.");
    await userEvent.click(canvas.getByRole("button", { name: "Approve with fixes" }));
    await expect(args.onDraft).toHaveBeenCalledWith("d2", { fixed: "A Job waits for a free slot." });
  },
};

/** Edited back to what it was, the draft is approved as written. Cancel puts the card back as it came. */
export const DraftEditUnchangedApprovesPlainly: Story = {
  args: { rows: drafted({ id: "d3" }), onDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Edit" }));
    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    await expect(canvas.queryByRole("textbox", { name: "Draft text" })).toBeNull();
    await expect(canvas.getByText("Queue a Job for later")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Edit" }));
    await userEvent.click(canvas.getByRole("button", { name: "Approve with fixes" }));
    await expect(args.onDraft).toHaveBeenCalledWith("d3", { choice: 0 });
  },
};

/** Note opens a box under the draft, and Send back sends his note for a rewrite. */
export const DraftNoteSendsBack: Story = {
  args: { rows: drafted({ id: "d4" }), onDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Note" }));
    await expect(canvas.getByText("Queue a Job for later")).toBeVisible();
    const answers = within(canvas.getByRole("group", { name: "Answers" })).getAllByRole("button");
    await expect(answers.map((one) => one.textContent)).toEqual(["Send back", "Cancel"]);
    await userEvent.type(canvas.getByRole("textbox", { name: "Note" }), "Say what a restart keeps.");
    await userEvent.click(canvas.getByRole("button", { name: "Send back" }));
    await expect(args.onDraft).toHaveBeenCalledWith("d4", { text: "Say what a restart keeps." });
  },
};

/** Send back waits for words: blank or spaces, it cannot be pressed. */
export const DraftSendBackWaitsForWords: Story = {
  args: { rows: drafted({ id: "d5" }), onDraft: fn() },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Note" }));
    const send = canvas.getByRole("button", { name: "Send back" });
    await expect(send).toBeDisabled();
    const note = canvas.getByRole("textbox", { name: "Note" });
    await userEvent.type(note, "   ");
    await expect(send).toBeDisabled();
    await userEvent.type(note, "Shorter");
    await expect(send).toBeEnabled();
    await userEvent.clear(note);
    await expect(send).toBeDisabled();
    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    await expect(args.onDraft).not.toHaveBeenCalled();
  },
};

/** Approved as written: the state says so, the text stays, and nothing is left to press. */
export const DraftApproved: Story = {
  args: { rows: drafted({ id: "d6", settled: { how: "approved", at: "2026-10-26T14:07:00Z" } }), onDraft: fn() },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Draft: Description for the queue pull request" });
    await expect(within(card).getByRole("img", { name: "Approved" })).toBeInTheDocument();
    await expect(within(card).getAllByText("Approved").some((one) => one.checkVisibility())).toBe(true);
    await expect(within(card).getByText("Queue a Job for later")).toBeVisible();
    await expect(within(card).queryByRole("group", { name: "Answers" })).toBeNull();
    await expect(within(card).queryByRole("button", { name: /^(Approve|Edit|Note)$/ })).toBeNull();
  },
};

/** Approved with his fixes: the card reads his text, not the agent's, and nothing is left to press. */
export const DraftApprovedWithFixes: Story = {
  args: { rows: drafted({ id: "d7", settled: { how: "fixed", words: "A Job waits for a free slot.", at: "2026-10-26T14:07:00Z" } }), onDraft: fn() },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Draft: Description for the queue pull request" });
    await expect(within(card).getByRole("img", { name: "Approved with your fixes" })).toBeInTheDocument();
    await expect(within(card).getAllByText("Approved with your fixes").some((one) => one.checkVisibility())).toBe(true);
    await expect(within(card).getByText("A Job waits for a free slot.")).toBeVisible();
    await expect(within(card).queryByText("Queue a Job for later")).toBeNull();
    await expect(within(card).queryByRole("group", { name: "Answers" })).toBeNull();
  },
};

/** Withdrawn: the state, his reason quoted under the draft, and nothing left to press. */
export const DraftWithdrawn: Story = {
  args: { rows: drafted({ id: "d11", settled: { how: "withdrawn", words: "The pull request is closed.", at: "2026-10-26T14:07:00Z" } }), onDraft: fn(), onWithdrawDraft: fn() },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Draft: Description for the queue pull request" });
    await expect(within(card).getByRole("img", { name: "Withdrawn" })).toBeInTheDocument();
    await expect(within(card).getAllByText("Withdrawn").some((one) => one.checkVisibility())).toBe(true);
    await expect(within(card).getByText("The pull request is closed.").closest("blockquote")).not.toBeNull();
    await expect(within(card).queryByRole("group", { name: "Answers" })).toBeNull();
    await expect(within(card).queryByRole("button", { name: /^(Approve|Edit|Note|Withdraw)$/ })).toBeNull();
  },
};

/** Sent back: the state, and his note quoted under the draft. Nothing is left to press. */
export const DraftSentBack: Story = {
  args: { rows: drafted({ id: "d8", settled: { how: "sent_back", words: "Say what a restart keeps.", at: "2026-10-26T14:07:00Z" } }), onDraft: fn() },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("article", { name: "Draft: Description for the queue pull request" });
    await expect(within(card).getByRole("img", { name: "Sent back" })).toBeInTheDocument();
    await expect(within(card).getAllByText("Sent back").some((one) => one.checkVisibility())).toBe(true);
    await expect(within(card).getByText("Say what a restart keeps.").closest("blockquote")).not.toBeNull();
    await expect(within(card).queryByRole("group", { name: "Answers" })).toBeNull();
    await expect(within(card).queryByRole("button", { name: /^(Approve|Edit|Note)$/ })).toBeNull();
  },
};

const NOT_STARTED: SessionThreadRow = { id: "n1", at: "14:09:00", kind: "not_started", why: "Did not start: the agent did not come up within 10 seconds", log: ["error: unknown option '--resume'", "usage: agent [flags]"] };

/** A Job whose agent never came up: the cause is the line, the error lines stay folded until pressed open, and Start again and Report call their handlers. */
export const StartFailed: Story = {
  args: { rows: [NOT_STARTED], onStartAgain: fn(), onReport: fn() },
  play: async ({ canvas, args }) => {
    const line = within(canvas.getByRole("region", { name: "Failed to start" }));
    await expect(line.getByText("Did not start: the agent did not come up within 10 seconds")).toBeVisible();
    await expect(line.queryByText(/unknown option/)).toBeNull();
    await userEvent.click(line.getByRole("button", { name: "Error lines" }));
    await expect(line.getByText(/unknown option '--resume'/)).toBeVisible();
    await expect(line.getByText(/usage: agent \[flags\]/)).toBeVisible();
    await userEvent.click(line.getByRole("button", { name: "Error lines" }));
    await expect(line.queryByText(/unknown option/)).toBeNull();
    await userEvent.click(line.getByRole("button", { name: "Start again" }));
    await expect(args.onStartAgain).toHaveBeenCalledTimes(1);
    await userEvent.click(line.getByRole("button", { name: "Report" }));
    await expect(args.onReport).toHaveBeenCalledTimes(1);
  },
};

/** Where the host serves neither act, and the agent left no error lines, the line is the cause alone: no press is drawn. */
export const StartFailedWithNothingToPress: Story = {
  args: { rows: [{ ...NOT_STARTED, log: [] } as SessionThreadRow] },
  play: async ({ canvas }) => {
    const line = within(canvas.getByRole("region", { name: "Failed to start" }));
    await expect(line.getByText("Did not start: the agent did not come up within 10 seconds")).toBeVisible();
    await expect(line.queryByRole("button")).toBeNull();
  },
};

/** Each press is absent when its handler is: with only Report served, Start again is not drawn. */
export const StartFailedPressesFollowTheirHandlers: Story = {
  args: { rows: [NOT_STARTED], onReport: fn() },
  play: async ({ canvas }) => {
    const line = within(canvas.getByRole("region", { name: "Failed to start" }));
    await expect(line.getByRole("button", { name: "Report" })).toBeInTheDocument();
    await expect(line.queryByRole("button", { name: "Start again" })).toBeNull();
  },
};

/** And with only Start again served, Report is not drawn. */
export const StartFailedWithoutReport: Story = {
  args: { rows: [NOT_STARTED], onStartAgain: fn() },
  play: async ({ canvas }) => {
    const line = within(canvas.getByRole("region", { name: "Failed to start" }));
    await expect(line.getByRole("button", { name: "Start again" })).toBeInTheDocument();
    await expect(line.queryByRole("button", { name: "Report" })).toBeNull();
  },
};
