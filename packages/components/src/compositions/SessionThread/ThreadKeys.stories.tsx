import type { Decorator, Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { HandoffBand } from "../NeedsYouPanel/HandoffBand";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import { ConcernCard, DoneBody, SessionThread, type SessionThreadProps, type SessionThreadRow } from "./SessionThread";
import { ThreadKeys } from "./thread-keys";

/**
 * The four keys on a Job's waiting cards: `⌥↵` yes, `⌥E` change, `⌥R` send back, `⌥X` no. The target is the card holding focus, else the
 * newest; `⌥↵` and `⌥X` always confirm in a dialog, `⌥E` and `⌥R` open the target's own box.
 */
type Job = SessionThreadProps & { composer?: { onSend: () => void } };

function JobThread({ composer, ...thread }: Job) {
  return (
    <>
      <SessionThread {...thread} />
      {composer === undefined ? null : <SessionComposer working={false} mode="auto" onMode={fn()} model={null} effort={null} models={[]} efforts={[]} onTune={fn()} commands={[]} taggable={[]} searchFiles={async () => []} tags={[]} onTags={fn()} drawn={[]} onDraw={fn()} onRemoveDrawn={fn()} onSend={composer.onSend} />}
    </>
  );
}

const meta: Meta<typeof JobThread> = {
  title: "Compositions/Thread keys",
  component: JobThread,
  args: { rows: [], onAnswer: fn(), onOpenSession: fn() },
  decorators: [
    (Story) => (
      <ThreadKeys>
        <Story />
      </ThreadKeys>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof JobThread>;

const framed: Decorator[] = [
  (Story) => (
    <div style={{ display: "flex", height: "calc(var(--space-12) * 12)", width: "calc(var(--space-12) * 10)", background: "var(--bg-raised)" }}>
      <Story />
    </div>
  ),
];
const flat: Decorator[] = [
  (Story) => (
    <div style={{ width: "calc(var(--space-12) * 10)", background: "var(--bg-raised)" }}>
      <Story />
    </div>
  ),
];

/** One chord, as a person holds it: Alt down, the key, Alt up. */
const alt = (key: string) => userEvent.keyboard(`{Alt>}${key}{/Alt}`);
const body = within(document.body);
const draftRow = (id: string, title: string, more: Partial<Extract<SessionThreadRow, { kind: "draft" }>> = {}): SessionThreadRow => ({ id, at: "14:06:00", kind: "draft", title, text: `Text of ${title}.`, ...more });
const DRAFTS: SessionThreadRow[] = [draftRow("d1", "First draft"), draftRow("d2", "Second draft"), draftRow("d3", "Third draft")];
const draftCard = (canvas: ReturnType<typeof within>, title: string) => canvas.getByRole("article", { name: `Draft: ${title}` });
const isTarget = (card: HTMLElement) => card.hasAttribute("data-thread-target");
const PLAN = {
  version: 1,
  approved: false,
  groups: [{ id: "g1", agent: "rust-engineer", title: "Keep the queue in the store", tasks: [{ id: "g1-0", text: "Add the queue table" }] }],
};
const plan = () => ({ ...PLAN, onApprove: fn(), onReview: fn(), onSetAside: fn() });

// Which card is the target

/** Three drafts wait: the newest is the target and wears the ring and the chords, and the other two draw neither. */
export const TargetIsTheNewestWaitingCard: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    const [first, second, third] = ["First draft", "Second draft", "Third draft"].map((title) => draftCard(canvas, title));
    await waitFor(() => expect(isTarget(third!)).toBe(true));
    await expect(isTarget(first!)).toBe(false);
    await expect(isTarget(second!)).toBe(false);
    await expect(within(third!).getByRole("button", { name: "Approve" }).textContent).toContain("⌥↵");
    await expect(within(third!).getByRole("button", { name: "Withdraw" }).textContent).toContain("⌥X");
    for (const card of [first!, second!]) await expect(within(card).getByRole("group", { name: "Answers" }).textContent).not.toMatch(/⌥/);
  },
};

/** Focus inside an older card makes it the target; the ring and the chords move with it. */
export const FocusMakesAnOlderCardTheTarget: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    const first = draftCard(canvas, "First draft");
    const third = draftCard(canvas, "Third draft");
    await waitFor(() => expect(isTarget(third)).toBe(true));
    await userEvent.click(within(first).getByText("First draft"));
    await waitFor(() => expect(isTarget(first)).toBe(true));
    await expect(isTarget(third)).toBe(false);
    await expect(within(first).getByRole("button", { name: "Approve" }).textContent).toContain("⌥↵");
    await expect(within(third).getByRole("group", { name: "Answers" }).textContent).not.toMatch(/⌥/);
  },
};

/** A settled draft and an approved plan wait on nothing: neither registers, so the target is the draft still open and the confirm lists it alone. */
export const SettledCardsDoNotRegister: Story = {
  args: { rows: [draftRow("d1", "First draft"), draftRow("d2", "Second draft", { settled: { how: "approved", at: "14:07:00" } })], approval: { ...plan(), approved: true }, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "First draft"))).toBe(true));
    await alt("{Enter}");
    const dialog = await body.findByRole("dialog", { name: "Approve which?" });
    await expect(within(dialog).getAllByRole("radio")).toHaveLength(1);
    await expect(within(dialog).getByRole("radio", { name: "Draft · First draft" })).toBeChecked();
  },
};

// The confirm on yes and no

/** One card waiting, and `⌥↵` still asks: nothing is answered until the dialog is confirmed with Enter. */
export const YesConfirmsEvenWithOneCard: Story = {
  args: { rows: [DRAFTS[0]!], onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "First draft"))).toBe(true));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Approve which?" });
    await expect(args.onDraft).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onDraft).toHaveBeenCalledWith("d1", { choice: 0 }));
    await expect(body.queryByRole("dialog")).toBeNull();
  },
};

/** `⌥X` asks too, and its confirm withdraws the draft. */
export const NoConfirmsEvenWithOneCard: Story = {
  args: { rows: [DRAFTS[0]!], onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "First draft"))).toBe(true));
    await alt("x");
    await body.findByRole("dialog", { name: "Withdraw which?" });
    await expect(args.onWithdrawDraft).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onWithdrawDraft).toHaveBeenCalledWith("d1", undefined));
  },
};

/** The confirm lists every waiting card with that act, numbered, the target picked; a digit picks another and Enter answers that one alone. */
export const ConfirmListsEveryCardWithTheActAndADigitPicks: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "Third draft"))).toBe(true));
    await alt("{Enter}");
    const dialog = await body.findByRole("dialog", { name: "Approve which?" });
    const radios = within(dialog).getAllByRole("radio");
    await expect(radios.map((radio) => radio.closest("label")?.textContent)).toEqual(["1Draft · First draft", "2Draft · Second draft", "3Draft · Third draft"]);
    await expect(radios.map((radio) => (radio as HTMLInputElement).checked)).toEqual([false, false, true]);
    await userEvent.keyboard("1");
    await waitFor(() => expect(within(dialog).getByRole("radio", { name: "Draft · First draft" })).toBeChecked());
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onDraft).toHaveBeenCalledTimes(1));
    await expect(args.onDraft).toHaveBeenCalledWith("d1", { choice: 0 });
  },
};

/** `⌥↵` inside the dialog confirms the picked card as Enter does. */
export const AltEnterConfirmsThePickedCard: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "Third draft"))).toBe(true));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Approve which?" });
    await userEvent.keyboard("2");
    await alt("{Enter}");
    await waitFor(() => expect(args.onDraft).toHaveBeenCalledWith("d2", { choice: 0 }));
    await expect(args.onDraft).toHaveBeenCalledTimes(1);
  },
};

/** Esc cancels: the dialog goes and no card's handler runs. */
export const EscCancelsTheConfirmAndAnswersNothing: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(draftCard(canvas, "Third draft"))).toBe(true));
    await alt("x");
    await body.findByRole("dialog", { name: "Withdraw which?" });
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(body.queryByRole("dialog")).toBeNull());
    await expect(args.onWithdrawDraft).not.toHaveBeenCalled();
    await expect(args.onDraft).not.toHaveBeenCalled();
  },
};

/** A card without the act is not listed: a plan has a yes and no no, so `⌥X` lists the two drafts and `⌥↵` lists all three. */
export const ConfirmListsOnlyTheCardsThatHaveTheAct: Story = {
  args: { rows: [DRAFTS[0]!, DRAFTS[1]!], approval: plan(), onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    await waitFor(() => expect(canvas.getByRole("article", { name: "Plan to approve" })).toBeVisible());
    await alt("x");
    const no = await body.findByRole("dialog", { name: /which\?$/ });
    await expect(within(no).getAllByRole("radio").map((radio) => radio.closest("label")?.textContent)).toEqual(["1Draft · First draft", "2Draft · Second draft"]);
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(body.queryByRole("dialog")).toBeNull());
    await alt("{Enter}");
    const yes = await body.findByRole("dialog", { name: /which\?$/ });
    await expect(within(yes).getAllByRole("radio")).toHaveLength(3);
  },
};

/** A card with no no sits above the drafts: with focus in the first draft, `⌥X` lists the two drafts and picks the one holding focus. */
export const ConfirmPicksTheTargetAmongTheCardsThatHaveTheAct: Story = {
  args: { rows: [DRAFTS[0]!, DRAFTS[1]!], onDraft: fn(), onWithdrawDraft: fn() },
  render: (args) => (
    <>
      <ConcernCard concern={{ id: "c1", text: "The queue has no ceiling." }} onSend={fn()} />
      <JobThread {...args} />
    </>
  ),
  decorators: framed,
  play: async ({ canvas }) => {
    const first = draftCard(canvas, "First draft");
    await userEvent.click(within(first).getByText("First draft"));
    await waitFor(() => expect(isTarget(first)).toBe(true));
    await alt("x");
    const dialog = await body.findByRole("dialog", { name: "Withdraw which?" });
    await expect(within(dialog).getAllByRole("radio")).toHaveLength(2);
    await expect(within(dialog).getByRole("radio", { name: "Draft · First draft" })).toBeChecked();
  },
};

// Change and send back open the target's own box

/** `⌥E` opens the target's edit box with no dialog and the caret in it; with focus in an older card it opens that one's. */
export const AltEOpensTheTargetsEditBoxWithTheCaretInIt: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    const third = draftCard(canvas, "Third draft");
    await waitFor(() => expect(isTarget(third)).toBe(true));
    await alt("e");
    const box = await within(third).findByRole("textbox", { name: "Draft text" });
    await waitFor(() => expect(box).toHaveFocus());
    await expect(body.queryByRole("dialog")).toBeNull();
    await userEvent.click(within(third).getByRole("button", { name: "Cancel" }));
    const first = draftCard(canvas, "First draft");
    await userEvent.click(within(first).getByText("First draft"));
    await waitFor(() => expect(isTarget(first)).toBe(true));
    await alt("e");
    const older = await within(first).findByRole("textbox", { name: "Draft text" });
    await waitFor(() => expect(older).toHaveFocus());
    await expect(within(third).queryByRole("textbox", { name: "Draft text" })).toBeNull();
  },
};

/** `⌥R` opens the target's note box with no dialog and the caret in it. */
export const AltROpensTheTargetsNoteBoxWithTheCaretInIt: Story = {
  args: { rows: DRAFTS, onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    const third = draftCard(canvas, "Third draft");
    await waitFor(() => expect(isTarget(third)).toBe(true));
    await alt("r");
    const box = await within(third).findByRole("textbox", { name: "Note" });
    await waitFor(() => expect(box).toHaveFocus());
    await expect(body.queryByRole("dialog")).toBeNull();
    await expect(within(draftCard(canvas, "Second draft")).queryByRole("textbox", { name: "Note" })).toBeNull();
  },
};

/** On a plan, `⌥E` opens the plan in full and `⌥R` sets it aside, neither through a dialog. */
export const AltEOnAPlanOpensItInFull: Story = {
  args: { rows: [], approval: plan() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(canvas.getByRole("article", { name: "Plan to approve" }))).toBe(true));
    await alt("e");
    await waitFor(() => expect(args.approval!.onReview).toHaveBeenCalledTimes(1));
    await alt("r");
    await waitFor(() => expect(args.approval!.onSetAside).toHaveBeenCalledTimes(1));
    await expect(body.queryByRole("dialog")).toBeNull();
  },
};

/** An Option-E press on a US Mac arrives as a dead key; it still opens the box, read by the key's place. */
export const DeadKeyAltEOpensTheEditBox: Story = {
  args: { rows: [DRAFTS[0]!], onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas }) => {
    const card = draftCard(canvas, "First draft");
    await waitFor(() => expect(isTarget(card)).toBe(true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Dead", code: "KeyE", altKey: true, bubbles: true, cancelable: true }));
    await within(card).findByRole("textbox", { name: "Draft text" });
  },
};

// The message box

const sendWhere = (waiting: boolean): Story => ({
  args: { rows: waiting ? [DRAFTS[0]!] : [], onDraft: fn(), onWithdrawDraft: fn() },
  render: (args) => <JobThread {...args} composer={{ onSend: sent }} />,
  decorators: framed,
});
const sent = fn();

/** With no card waiting a yes, `⌥↵` in the message box sends as it always did. */
export const MessageBoxSendsOnAltEnterWhenNoCardWaits: Story = {
  ...sendWhere(false),
  play: async ({ canvas }) => {
    sent.mockClear();
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "also check the tests");
    await alt("{Enter}");
    await expect(sent).toHaveBeenCalledTimes(1);
  },
};

/** While a card has a yes, the message box gives `⌥↵` up: it opens the confirm and sends nothing, while a plain Enter still sends. */
export const MessageBoxGivesUpAltEnterWhileACardHasAYes: Story = {
  ...sendWhere(true),
  play: async ({ canvas }) => {
    sent.mockClear();
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "also check the tests");
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Approve which?" });
    await expect(sent).not.toHaveBeenCalled();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(body.queryByRole("dialog")).toBeNull());
    await userEvent.click(field);
    await userEvent.keyboard("{Enter}");
    await expect(sent).toHaveBeenCalledTimes(1);
  },
};

// A proposed workflow

/** A workflow a Job proposed has no Edit and no Note, so `⌥E` and `⌥R` do nothing, and its `⌥X` is Dismiss. */
export const ProposedWorkflowHasNoEditOrNoteAndItsNoIsDismiss: Story = {
  args: { rows: [draftRow("w1", "Land on green", { workflow: { name: "land-on-green", scope: "repository" } })], onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    const card = canvas.getByRole("article", { name: "Workflow: Land on green" });
    await waitFor(() => expect(isTarget(card)).toBe(true));
    await alt("e");
    await alt("r");
    await expect(within(card).queryByRole("textbox")).toBeNull();
    await expect(body.queryByRole("dialog")).toBeNull();
    await expect(within(card).getByRole("button", { name: "Dismiss" }).textContent).toContain("⌥X");
    await alt("x");
    await body.findByRole("dialog", { name: "Dismiss which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onWithdrawDraft).toHaveBeenCalledWith("w1", undefined));
  },
};

// Each kind of card, its four keys

/** A draft: `⌥E` Edit, `⌥R` Note, `⌥↵` Approve and `⌥X` Withdraw each reach their own handler. */
export const DraftTakesAllFourKeys: Story = {
  args: { rows: [DRAFTS[0]!], onDraft: fn(), onWithdrawDraft: fn() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    const card = draftCard(canvas, "First draft");
    await waitFor(() => expect(isTarget(card)).toBe(true));
    await alt("e");
    await within(card).findByRole("textbox", { name: "Draft text" });
    await userEvent.click(within(card).getByRole("button", { name: "Cancel" }));
    await alt("r");
    await within(card).findByRole("textbox", { name: "Note" });
    await userEvent.click(within(card).getByRole("button", { name: "Cancel" }));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Approve which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onDraft).toHaveBeenCalledWith("d1", { choice: 0 }));
    await alt("x");
    await body.findByRole("dialog", { name: "Withdraw which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onWithdrawDraft).toHaveBeenCalledWith("d1", undefined));
  },
};

/** A plan: `⌥↵` Approve, `⌥E` the plan in full, `⌥R` Not yet; it has no no, so `⌥X` opens nothing. */
export const PlanTakesItsThreeKeys: Story = {
  args: { rows: [], approval: plan() },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(canvas.getByRole("article", { name: "Plan to approve" }))).toBe(true));
    await alt("e");
    await waitFor(() => expect(args.approval!.onReview).toHaveBeenCalledTimes(1));
    await alt("r");
    await waitFor(() => expect(args.approval!.onSetAside).toHaveBeenCalledTimes(1));
    await alt("x");
    await expect(body.queryByRole("dialog")).toBeNull();
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Approve which?" });
    await expect(args.approval!.onApprove).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.approval!.onApprove).toHaveBeenCalledTimes(1));
  },
};

/** The done band: `⌥↵` End job and `⌥X` Keep going; it has nothing to change or send back. */
export const DoneBandTakesYesAndNo: Story = {
  render: () => <DoneBody done={DONE} />,
  decorators: flat,
  play: async ({ canvas }) => {
    DONE.onEnd.mockClear();
    DONE.onKeep.mockClear();
    await waitFor(() => expect(isTarget(canvas.getByRole("button", { name: "End job" }).parentElement!)).toBe(true));
    await alt("e");
    await alt("r");
    await expect(body.queryByRole("dialog")).toBeNull();
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "End job which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(DONE.onEnd).toHaveBeenCalledTimes(1));
    await alt("x");
    await body.findByRole("dialog", { name: "Keep going which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(DONE.onKeep).toHaveBeenCalledTimes(1));
    await expect(DONE.onEnd).toHaveBeenCalledTimes(1);
  },
};
const DONE = { onEnd: fn(), onKeep: fn() };

/** A judge's concern: `⌥↵` Send to job and `⌥X` Dismiss. */
export const ConcernTakesYesAndNo: Story = {
  render: () => <ConcernCard concern={{ id: "c1", text: "The queue has no ceiling." }} onSend={concern.onSend} onDismiss={concern.onDismiss} />,
  decorators: flat,
  play: async ({ canvas }) => {
    concern.onSend.mockClear();
    concern.onDismiss.mockClear();
    await waitFor(() => expect(isTarget(canvas.getByRole("group", { name: "Judge: The queue has no ceiling." }))).toBe(true));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Send to job which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(concern.onSend).toHaveBeenCalledTimes(1));
    await alt("x");
    await body.findByRole("dialog", { name: "Dismiss which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(concern.onDismiss).toHaveBeenCalledTimes(1));
    await expect(concern.onSend).toHaveBeenCalledTimes(1);
  },
};
const concern = { onSend: fn(), onDismiss: fn() };

/** A permission the agent is held on: `⌥↵` allows once and `⌥X` takes the refuse the ask offers. */
export const PermissionTakesAllowOnceAndRefuse: Story = {
  args: {
    rows: [],
    asked: {
      command: "cargo test -p fleet queue",
      offers: [
        { id: "allow_once", label: "Allow once", means: "" },
        { id: "refuse", label: "Refuse", means: "" },
      ],
    },
  },
  decorators: framed,
  play: async ({ canvas, args }) => {
    await waitFor(() => expect(isTarget(canvas.getByRole("article", { name: "Waiting on you" }))).toBe(true));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Allow once which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onAnswer).toHaveBeenCalledWith("allow_once"));
    await alt("x");
    await body.findByRole("dialog", { name: "Refuse which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onAnswer).toHaveBeenCalledWith("refuse"));
  },
};

/** A question: `⌥E` takes Other with the caret in it, `⌥↵` answers only once the question has an answer, and `⌥X` skips. */
export const QuestionTakesOtherAnswerAndSkip: Story = {
  args: {
    rows: [],
    asked: {
      command: "Which size?",
      offers: [
        { id: "allow_once", label: "Allow once", means: "" },
        { id: "refuse", label: "Refuse", means: "" },
      ],
      questions: [{ question: "Which size?", header: "Size", multi_select: false, options: [{ label: "S", description: "Small" }, { label: "L", description: "Large" }] }],
    },
  },
  decorators: framed,
  play: async ({ canvas, canvasElement, args }) => {
    await waitFor(() => expect(canvasElement.querySelector("[data-thread-target]")).not.toBeNull());
    await alt("{Enter}");
    await expect(body.queryByRole("dialog")).toBeNull();
    await alt("e");
    const other = await canvas.findByRole("textbox", { name: "Other" });
    await waitFor(() => expect(other).toHaveFocus());
    await userEvent.type(other, "M");
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Answer which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onAnswer).toHaveBeenCalledWith("allow_once", [{ question: "Which size?", chosen: ["M"] }]));
    await alt("x");
    await body.findByRole("dialog", { name: "Skip which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(args.onAnswer).toHaveBeenCalledWith("refuse"));
  },
};

/** A failed hand off: `⌥↵` is Retry, and `R` and `D` in the band are Retry and Dispatch job to fix. */
export const HandoffBandTakesRetryAndDispatch: Story = {
  render: () => <HandoffBand reason="the close was refused" onRetry={handoff.onRetry} onDispatch={handoff.onDispatch} onDismiss={fn()} />,
  decorators: flat,
  play: async ({ canvas }) => {
    handoff.onRetry.mockClear();
    handoff.onDispatch.mockClear();
    const band = canvas.getByRole("region", { name: "Failed to hand off to new job" });
    await waitFor(() => expect(isTarget(band)).toBe(true));
    await alt("{Enter}");
    await body.findByRole("dialog", { name: "Retry which?" });
    await userEvent.keyboard("{Enter}");
    await waitFor(() => expect(handoff.onRetry).toHaveBeenCalledTimes(1));
    within(band).getByRole("button", { name: "Retry" }).focus();
    await userEvent.keyboard("d");
    await expect(handoff.onDispatch).toHaveBeenCalledTimes(1);
    await userEvent.keyboard("r");
    await expect(handoff.onRetry).toHaveBeenCalledTimes(2);
  },
};
const handoff = { onRetry: fn(), onDispatch: fn() };
