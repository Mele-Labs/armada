import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, waitFor } from "storybook/test";

import type { LedgerEntry } from "../SessionLedger/SessionLedger";
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
function Frame({ question, judges, rows, startOpen = false, onOpen }: { question?: typeof QUESTION; judges?: typeof JUDGES; rows: LedgerEntry[]; startOpen?: boolean; onOpen?: (next: boolean) => void }) {
  const [open, setOpen] = useState(startOpen);
  const [ask, setAsk] = useState(question);
  const [items, setItems] = useState(rows);
  return (
    <div className="armada-session-frame__centre" style={{ display: "flex", flexDirection: "column", height: "calc(var(--space-12) * 8)", width: "calc(var(--space-12) * 10)" }}>
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
