import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import type { LedgerEntry } from "../SessionLedger/SessionLedger";
import { AskCards } from "../SessionThread/SessionThread";
import type { AskedQuestion } from "../SessionThread/SessionThread";
import { NeedsYouPanel } from "./NeedsYouPanel";

/**
 * Everything the agent needs from the person, in one panel joined to the composer. It opens as a list, the question first, then a judge's
 * concerns, then the rest, a row each and no act on a row; a press on a row opens that one, and Back returns to the list.
 */
const meta: Meta<typeof NeedsYouPanel> = {
  title: "Compositions/Needs you panel",
  component: NeedsYouPanel,
};
export default meta;

type Story = StoryObj<typeof NeedsYouPanel>;

const walk = fn();
const removed = fn();
const ROWS: LedgerEntry[] = [
  { key: "w1", kind: "waiting", act: "walk", name: "Look at the findings page", text: "Look at the findings page", onOpen: walk, onRemove: removed },
  { key: "w2", kind: "waiting", act: "approve_pr", name: "Approve #1843", text: "Approve #1843", onOpen: fn() },
];
const QUESTION = { id: "q1", label: "Question", text: "Which clock?", children: <p>Fake or frozen?</p> };
const JUDGES = [{ id: "j1", text: "The queue has no ceiling.", children: <p>A concern with no ceiling.</p> }];

/** The thread's stage is the dimmed part; the box beneath it is what the panel is attached to. Both carry the classes the Session view gives them. */
function Frame({ question, judges, rows, startOpen = false, onOpen, threadH }: { question?: typeof QUESTION; judges?: typeof JUDGES; rows: LedgerEntry[]; startOpen?: boolean; onOpen?: (next: boolean) => void; /** The thread's height in px, which the Session view sets as `--thread-h` for the panel to bound itself by. */ threadH?: number }) {
  const [open, setOpen] = useState(startOpen);
  const [ask, setAsk] = useState(question);
  const [items, setItems] = useState(rows);
  return (
    <div className="armada-session-frame__centre" style={{ display: "flex", flexDirection: "column", height: "calc(var(--space-12) * 8)", width: "calc(var(--space-12) * 10)", ...(threadH === undefined ? {} : { height: threadH, ["--thread-h" as string]: `${threadH}px` }) }}>
      <div className="armada-session-thread__stage" data-testid="stage" style={{ position: "relative", flex: 1 }}>
        <p>The thread</p>
      </div>
      <div style={{ position: "relative", marginBlockStart: "var(--h-badge)" }}>
        <NeedsYouPanel
          rows={items}
          open={open}
          onOpen={(next) => {
            setOpen(next);
            onOpen?.(next);
          }}
          {...(ask === undefined ? {} : { question: ask })}
          {...(judges === undefined ? {} : { judges })}
        />
        <textarea aria-label="Message" />
      </div>
      <button type="button" onClick={() => setAsk({ ...QUESTION })}>
        Ask again
      </button>
      <button type="button" onClick={() => setAsk(undefined)}>
        Answered
      </button>
      <button type="button" onClick={() => setItems([])}>
        Clear the rest
      </button>
    </div>
  );
}

type Canvas = Parameters<NonNullable<Story["play"]>>[0]["canvas"];
const rowNames = (canvas: Canvas) => canvas.getAllByRole("button", { name: /^Open / }).map((one) => one.getAttribute("aria-label"));

/** Opened, it is a list: the question first, then the judge, then the rest, and not one of them carries an act of its own. */
export const OpensToAListQuestionFirst: Story = {
  render: () => <Frame question={QUESTION} judges={JUDGES} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    await expect(rowNames(canvas)).toEqual(["Open Which clock?", "Open The queue has no ceiling.", "Open Look at the findings page", "Open Approve #1843"]);
    await expect(canvas.queryByRole("article", { name: "Waiting on you" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Back" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Needs you" })).toHaveAttribute("aria-expanded", "true");
  },
};

/** A question asked in the walk window is a question: its act is the walk, but its row and its glyph on the band are marked Question. */
export const AQuestionAskedInTheWindowIsMarkedQuestion: Story = {
  render: () => (
    <Frame
      rows={[
        { key: "wa", kind: "waiting", act: "walk", name: "Beside the title or under it?", text: "Beside the title or under it?", options: [{ label: "Beside", onPick: fn() }, { label: "Under", onPick: fn() }], onOpen: walk },
        ...ROWS,
      ]}
      startOpen
    />
  ),
  play: async ({ canvas }) => {
    const row = canvas.getByRole("button", { name: "Open Beside the title or under it?" });
    await expect(row.querySelector("[role='img']")).toHaveAccessibleName("Question");
    await expect(canvas.getAllByRole("img", { name: "Opens the walk" })).toHaveLength(1);
  },
};

/** The band that sits on the composer is named for what it is, and folded it holds none of its rows. */
export const TheTriggerIsNamedNeedsYou: Story = {
  render: () => <Frame rows={ROWS} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Needs you" })).toBeInTheDocument();
    const trigger = canvas.getByRole("button", { name: "Needs you" });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByRole("button", { name: /^Open / })).toBeNull();
  },
};

/** A press on a row opens that one in the panel's place, with Back, and the list is not drawn under it. */
export const ARowOpensItsDetail: Story = {
  render: () => <Frame question={QUESTION} judges={JUDGES} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Which clock?" }));
    await expect(canvas.getByText("Fake or frozen?")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /^Open / })).toBeNull();
  },
};

/** Back, Esc and Backspace each return to the list, and focus is on the row that was left. */
export const BackEscAndBackspaceReturnToTheRowLeft: Story = {
  render: () => <Frame question={QUESTION} judges={JUDGES} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    const leave = async (go: () => Promise<unknown>) => {
      await userEvent.click(canvas.getByRole("button", { name: "Open The queue has no ceiling." }));
      await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
      await go();
      await waitFor(() => expect(canvas.getByRole("button", { name: "Open The queue has no ceiling." })).toHaveFocus());
    };
    await leave(() => userEvent.click(canvas.getByRole("button", { name: "Back" })));
    await leave(() => userEvent.keyboard("{Escape}"));
    // The detail's own focus is in place before Backspace is pressed, and nothing there takes typing.
    await leave(() => userEvent.keyboard("{Backspace}"));
  },
};

/** Esc on the list folds the panel to its trigger, and focus goes to the trigger. */
export const EscOnTheListFolds: Story = {
  render: () => <Frame question={QUESTION} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Which clock?" }));
    // The detail takes focus on the next frame; Esc before that has no panel focus to go back from.
    await waitFor(() => expect(canvas.getByRole("region", { name: "Needs you" }).contains(document.activeElement)).toBe(true));
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(canvas.getByRole("button", { name: "Open Which clock?" })).toHaveFocus());
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(canvas.getByRole("button", { name: "Needs you" })).toHaveAttribute("aria-expanded", "false"));
    await expect(canvas.getByRole("button", { name: "Needs you" })).toHaveFocus();
  },
};

/** The arrows move between rows, and Enter opens the one under focus. */
export const ArrowsAndEnter: Story = {
  render: () => <Frame question={QUESTION} judges={JUDGES} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    // Arriving, the question puts focus on the first row itself.
    await waitFor(() => expect(canvas.getByRole("button", { name: "Open Which clock?" })).toHaveFocus());
    await userEvent.keyboard("{ArrowDown}");
    await expect(canvas.getByRole("button", { name: "Open The queue has no ceiling." })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}");
    await expect(canvas.getByRole("button", { name: "Open Approve #1843" })).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(canvas.getByRole("button", { name: "Open Look at the findings page" })).toHaveFocus();
    walk.mockClear();
    await userEvent.keyboard("{Enter}");
    await expect(walk).toHaveBeenCalledTimes(1);
    await userEvent.keyboard("{ArrowUp}{ArrowUp}{Enter}");
    await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
  },
};

const raised = fn();
/** A question that arrives opens the panel once, to the list; folded, it stays folded when the same question is drawn again. */
export const AQuestionOpensItOnce: Story = {
  render: () => <Frame rows={ROWS} onOpen={raised} />,
  play: async ({ canvas }) => {
    const trigger = canvas.getByRole("button", { name: "Needs you" });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(canvas.getByRole("button", { name: "Ask again" }));
    await waitFor(() => expect(trigger).toHaveAttribute("aria-expanded", "true"));
    await expect(rowNames(canvas)[0]).toBe("Open Which clock?");
    await expect(canvas.queryByRole("button", { name: "Back" })).toBeNull();
    canvas.getByRole("button", { name: "Open Which clock?" }).focus();
    await userEvent.keyboard("{Escape}");
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(canvas.getByRole("button", { name: "Ask again" }));
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  },
};

/** A list with nothing left in it folds the panel and takes it away. */
export const AnEmptyListFoldsAndGoes: Story = {
  render: () => <Frame rows={ROWS} startOpen onOpen={raised} />,
  play: async ({ canvas }) => {
    raised.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Clear the rest" }));
    await waitFor(() => expect(canvas.queryByRole("region", { name: "Needs you" })).toBeNull());
    await expect(raised).toHaveBeenCalledWith(false);
  },
};

/** The thread behind the open panel is dimmed, and a press on it folds the panel. */
export const APressOnTheThreadFolds: Story = {
  render: () => <Frame question={QUESTION} rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    const trigger = canvas.getByRole("button", { name: "Needs you" });
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(canvas.getByTestId("stage"));
    await waitFor(() => expect(trigger).toHaveAttribute("aria-expanded", "false"));
  },
};

/** With the panel open and focus outside it, Esc folds it; focus is left where it was, and the next Esc is no longer the panel's. */
export const EscFoldsWithFocusOutside: Story = {
  render: () => <Frame rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    const trigger = canvas.getByRole("button", { name: "Needs you" });
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    canvas.getByRole("button", { name: "Ask again" }).focus();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveAttribute("aria-expanded", "false"));
    await expect(canvas.getByRole("button", { name: "Ask again" })).toHaveFocus();
    let seen = false;
    const note = (event: KeyboardEvent) => void (seen = !event.defaultPrevented);
    window.addEventListener("keydown", note);
    await userEvent.keyboard("{Escape}");
    window.removeEventListener("keydown", note);
    await expect(seen).toBe(true);
  },
};

/** An item whose whole act is to open something does it from its row: no detail, one press. Remove stays at the row's end. */
export const AnOpeningItemActsFromTheList: Story = {
  render: () => <Frame rows={ROWS} startOpen />,
  play: async ({ canvas }) => {
    walk.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Open Look at the findings page" }));
    await expect(walk).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("button", { name: "Back" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Remove Look at the findings page" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Reason for removing Look at the findings page" }), "Already done{Enter}");
    await expect(removed).toHaveBeenCalledWith("Already done");
  },
};

/** An item with a form of its own, here a reply, still opens a detail from its row when something else waits beside it. */
export const AReplyKeepsListThenDetail: Story = {
  render: () => <Frame rows={[{ key: "r1", kind: "waiting", name: "Say which branch", text: "Say which branch", onReply: fn(), onOpen: fn() }, ROWS[0]!]} startOpen />,
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open Say which branch" }));
    await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
    await expect(canvas.getByRole("textbox", { name: "Reply to Say which branch" })).toBeVisible();
  },
};

const misread = fn();
const confirmed = fn();
const READ: LedgerEntry = { key: "m1", kind: "waiting", name: "What you get", text: "What you get", fromReply: true, onReply: fn(), onMisread: misread, onConfirmed: confirmed, onOpen: fn() };

/** A line Fleet read from a reply keeps its two thumbs when it is opened, so he can say it was no question without taking it off as an item the agent stated. */
export const AnOpenedLineReadFromAReplyKeepsItsThumbs: Story = {
  render: () => <Frame rows={[READ, ROWS[0]!]} startOpen />,
  play: async ({ canvas }) => {
    misread.mockClear();
    confirmed.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Open What you get" }));
    await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "A question" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Not a question" }));
    await expect(misread).toHaveBeenCalledTimes(1);
    await expect(confirmed).not.toHaveBeenCalled();
  },
};

/** Once Fleet holds the line as a question, opened or listed it draws no thumbs; and a line the agent stated never had any. */
export const AnOpenedConfirmedOrStatedLineHasNoThumbs: Story = {
  render: () => <Frame rows={[{ ...READ, confirmed: true }, { key: "m2", kind: "waiting", name: "Say which branch", text: "Say which branch", onReply: fn(), onOpen: fn() }]} startOpen />,
  play: async ({ canvas }) => {
    for (const name of ["Open What you get", "Open Say which branch"]) {
      await userEvent.click(canvas.getByRole("button", { name }));
      await expect(canvas.getByRole("button", { name: "Back" })).toBeVisible();
      await expect(canvas.queryByRole("button", { name: "Not a question" })).toBeNull();
      await expect(canvas.queryByRole("button", { name: "A question" })).toBeNull();
      await userEvent.click(canvas.getByRole("button", { name: "Back" }));
    }
  },
};

const CLOCK: AskedQuestion = {
  question: "Which clock should the queue tests use?",
  header: "Clock",
  multi_select: false,
  options: [
    { label: "Fake clock", description: "Tests advance time by hand with a fake clock. A retry that waits thirty seconds runs in no time, and a test can stop the clock between two enqueues. Timers created outside the fake are never seen by it." },
    { label: "Frozen clock", description: "Every call to now returns one fixed instant. Ordering by enqueue time needs a second source of ticks, and a lease that expires never does. The simplest option, and the one the older tests already use." },
    { label: "Real clock with short waits", description: "Tests sleep for real, with lease and retry waits cut to a few milliseconds. Nothing is faked, so timer bugs show up, and a slow CI machine can make a test flaky when a wait is shorter than a scheduler tick." },
    { label: "Injected clock per test", description: "Each test passes its own clock into the queue, fake or real as it needs. The queue's constructor gains a parameter, and the sixty callers in the daemon each pass the system clock." },
    { label: "Clock trait in the crate", description: "A Clock trait with a system and a test implementation lives in the queue crate. The other crates that read time can adopt it later, but until then two clocks sit side by side in the daemon." },
  ],
};
const CLOCK_ASK = { id: "q2", label: "Question", text: CLOCK.question, children: <AskCards flat asked={{ command: "AskUserQuestion", questions: [CLOCK] }} onAnswer={fn()} /> };

/** An open question keeps Back, the question and the Answer row in view while only its options scroll, however long their descriptions run. */
export const AnOpenQuestionScrollsOnlyItsOptions: Story = {
  render: () => <Frame question={CLOCK_ASK} rows={[]} startOpen threadH={400} />,
  play: async ({ canvas }) => {
    const options = canvas.getByRole("radiogroup", { name: "Options" });
    // The body is the one box with a bound; it has no role to find it by.
    const body = options.closest(".armada-needs-panel__body")!;
    const inside = (name: string | HTMLElement) => {
      const one = (typeof name === "string" ? canvas.getByRole("button", { name }) : name).getBoundingClientRect();
      const box = body.getBoundingClientRect();
      return one.top >= box.top && one.bottom <= box.bottom && one.left >= box.left && one.right <= box.right;
    };
    await expect(inside("Answer")).toBe(true);
    await expect(inside("Back")).toBe(true);
    await expect(inside(canvas.getByText(CLOCK.question))).toBe(true);
    await waitFor(() => expect(options.scrollHeight).toBeGreaterThan(options.clientHeight));
    const answer = canvas.getByRole("button", { name: "Answer" });
    const before = answer.getBoundingClientRect();
    options.scrollTop = options.scrollHeight;
    await expect(options.scrollTop).toBeGreaterThan(0);
    const after = answer.getBoundingClientRect();
    await expect([after.top, after.left]).toEqual([before.top, before.left]);
    await expect(inside("Answer")).toBe(true);
  },
};

const LANDS: AskedQuestion = {
  question:
    "Where should you choose how a Job lands? Right now you type a request into Dispatch and send it. The Job starts, a few seconds later Armada picks its kind, and only then does the delivery question land in Needs you, next to whatever else Fleet and the agent are already asking. Where should it go instead?",
  header: "Lands",
  multi_select: false,
  options: [
    { label: "In the Dispatch box", description: "A row beside Send.", preview: "┌ New Job ─────────┐\n│ Lands  ● Auto land │\n│        ○ PR only   │\n│        ○ Keep local│\n│ [ Start ↵ ]        │\n│                    │\n│                    │\n│                    │\n│                    │\n│                    │\n└────────────────────┘" },
    { label: "On the Job's first card", description: "Asked once the kind is known." },
    { label: "As a default per repository", description: "Asked once, then remembered." },
  ],
};
const LANDS_ASK = { id: "q3", label: "Question", text: LANDS.question, children: <AskCards flat asked={{ command: "AskUserQuestion", questions: [LANDS] }} onAnswer={fn()} /> };

/** A preview is a small card beside the option pointed at, not a box under the options, so a long question and a tall preview leave the options their rows. */
export const APreviewIsACardOnHover: Story = {
  render: () => <Frame question={LANDS_ASK} rows={[]} startOpen threadH={400} />,
  play: async ({ canvas }) => {
    const options = canvas.getByRole("radiogroup", { name: "Options" });
    const row = canvas.getByRole("radio", { name: "In the Dispatch box" });
    await expect(canvas.queryByRole("region", { name: "Preview" })).toBeNull();
    await waitFor(() => expect(options.clientHeight).toBeGreaterThanOrEqual(row.getBoundingClientRect().height * 3));
    const before = options.getBoundingClientRect().height;
    await userEvent.hover(row);
    const card = await within(document.body).findByRole("region", { name: "Preview" });
    await expect(card.closest(".armada-needs-panel__body")).toBeNull();
    await expect(options.getBoundingClientRect().height).toBe(before);
    await userEvent.unhover(row);
    await waitFor(() => expect(within(document.body).queryByRole("region", { name: "Preview" })).toBeNull());
  },
};
