import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn } from "storybook/test";

import { ProposerWait } from "./ProposerWait";

/**
 * The wait a Job at `proposing` is in, drawn inside Overview's lead: how far the
 * model call has got, what is left of Fleet's budget, and the one act on it.
 *
 * Why a dispatched request is a Job with a row rather than a screen somebody
 * sits on is the owner's decision of 30 Sep 2026, *a dispatched request is a
 * job*, in the decisions register.
 */
const meta: Meta<typeof ProposerWait> = {
  title: "Compositions/Proposer wait",
  component: ProposerWait,
  args: { onStop: fn(), slowAfterMs: 120_000 },
};
export default meta;

type Story = StoryObj<typeof ProposerWait>;

/**
 * Fleet has said nothing about the call yet — an older Fleet, the moment before
 * the first `proposal.moved`, or a window reopened on somebody else's dispatch.
 *
 * **The stop is here anyway.** A Job somebody came back to carries the only act
 * on it from the first frame; the alternative is a wait with nothing to press.
 */
export const NothingSaid: Story = {
  args: {},
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText(/fills this Job in as it writes/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Stop the proposer" }));
    await expect(args.onStop).toHaveBeenCalled();
  },
};

/**
 * **The reach worth telling apart.** A call sitting at `starting` never reached
 * the vendor at all, which is a harness or a credential problem and will not
 * resolve by waiting — so the sentence names the proposer rather than the model.
 */
export const NeverReachedTheModel: Story = {
  args: {
    watch: { reached: "starting", elapsedMs: 8_000, budgetMs: 600_000, model: "sonnet" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Starting the proposer")).toBeVisible();
    await expect(canvas.getByText("8s")).toBeVisible();
    // No thinking figure: absent rather than zeroed, because a call that has
    // not started thinking and one thinking about nothing are different things.
    await expect(canvas.queryByText(/tokens of thinking/)).toBeNull();
  },
};

/**
 * The call has reached the vendor and is thinking. **What a wait is for**: the
 * reach, the elapsed figure against Fleet's ceiling, and how much thinking there
 * has been — none of which an elapsed count alone can say.
 *
 * Well inside `slowAfterMs`, so nothing says the wait is long.
 */
export const Thinking: Story = {
  args: {
    watch: {
      reached: "thinking",
      elapsedMs: 41_000,
      budgetMs: 600_000,
      model: "haiku",
      thinkingTokens: 763,
    },
  },
  /**
   * **The press is pressed once and goes dead**, which no rendering shows: a
   * stop sent twice would be two kills, and the second would arrive after the
   * call it names has gone.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("The model is thinking")).toBeVisible();
    await expect(canvas.getByText("41s")).toBeVisible();
    await expect(canvas.getByText("haiku · 9m 19s left")).toBeVisible();
    await expect(canvas.queryByText(/taking longer than expected/)).toBeNull();

    const stop = canvas.getByRole("button", { name: "Stop the proposer" });
    await userEvent.click(stop);
    await expect(args.onStop).toHaveBeenCalledOnce();

    // Pending, not disabled: it is the control being waited on, so it stays
    // focusable and sweeps a bar. #1117. Dispatched rather than clicked,
    // because the app's base styles take a dead control out of pointer reach.
    const stopping = canvas.getByRole("button", { name: "Stopping…" });
    await expect(stopping).toHaveAttribute("aria-busy", "true");
    fireEvent.click(stopping);
    await expect(args.onStop).toHaveBeenCalledOnce();
  },
};

/**
 * Past the mark, and the wait says so. **There is no `Keep waiting` control** —
 * waiting is what happens if nothing is pressed, and a button for it would
 * perform no act.
 */
export const NearlyOutOfBudget: Story = {
  args: {
    watch: {
      reached: "thinking",
      elapsedMs: 460_000,
      budgetMs: 600_000,
      model: "sonnet",
      thinkingTokens: 4_210,
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/taking longer than expected/)).toBeVisible();
    await expect(canvas.getByText("about 4,210 tokens of thinking")).toBeVisible();
    await expect(canvas.getByText("sonnet · 2m 20s left")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Keep waiting" })).toBeNull();
  },
};

/**
 * The answer is arriving, and the call is nearly over. **The budget is spent**,
 * which the line says rather than drawing a zero.
 */
export const Answering: Story = {
  args: {
    watch: {
      reached: "answering",
      elapsedMs: 601_000,
      budgetMs: 600_000,
      model: "sonnet",
      thinkingTokens: 9_140,
      answeredCharacters: 2_480,
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The answer is arriving")).toBeVisible();
    await expect(canvas.getByText("sonnet · out of time")).toBeVisible();
    await expect(canvas.getByText("2,480 characters of answer so far")).toBeVisible();
  },
};

/**
 * A caller with no way to reach the act draws no control. **A window that cannot
 * stop the call says nothing about stopping it**, rather than offering a press
 * that reaches nothing.
 */
export const NoWayToStop: Story = {
  args: {
    onStop: undefined,
    watch: { reached: "requesting", elapsedMs: 12_000, budgetMs: 600_000, model: "sonnet" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Asking the model")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Stop the proposer" })).toBeNull();
  },
};

/**
 * The four fields, as the third of them is arriving.
 *
 * **The owner asked for this on 30 Sep 2026** — *"Is there anyway for it to fill
 * in as it goes?"* — and named the four in this order, which is his reasoning
 * rather than a layout: the workflow decides the Job's shape, the title is what
 * makes the row recognisable, done-when is the goal, and the settings are the
 * part he can still change.
 *
 * **A row per criterion and never a count**: the lines arrive one at a time, and
 * a count beside the items it counts is refused (29 Sep 2026).
 */
export const FillingIn: Story = {
  args: {
    watch: {
      reached: "answering",
      elapsedMs: 21_000,
      budgetMs: 600_000,
      model: "sonnet",
      thinkingTokens: 1_840,
      answeredCharacters: 212,
      settled: [
        { label: "Workflow", said: "feature" },
        { label: "Title", said: "Say which of the two a clear gave back" },
        {
          label: "Done when",
          said: "The Cleared tab names the branch on every row whose worktree is gone",
        },
      ],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("feature")).toBeVisible();
    await expect(canvas.getByText("Say which of the two a clear gave back")).toBeVisible();
    // The fourth has not landed, and nothing stands in for it: an empty slot
    // stays empty (29 Sep 2026).
    await expect(canvas.queryByText("Urgency")).toBeNull();
  },
};

/**
 * Nothing has settled, so nothing is drawn for it. **The absence is the
 * design**: a label over a blank is the placeholder *an empty slot stays empty*
 * refuses, and a call that has not started writing has decided nothing.
 */
export const NothingSettledYet: Story = {
  args: {
    watch: { reached: "thinking", elapsedMs: 9_000, budgetMs: 600_000, model: "sonnet" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Workflow")).toBeNull();
    await expect(canvas.queryByText("Title")).toBeNull();
  },
};
