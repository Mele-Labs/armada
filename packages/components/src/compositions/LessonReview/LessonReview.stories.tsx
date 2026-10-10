import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { LessonCard } from "../LessonCard/LessonCard";
import {
  LessonReviewEnd,
  LessonReviewStep,
  LessonReviewWait,
  ReviewSetAside,
  type LessonReviewStepProps,
} from "./LessonReview";

/**
 * The guided review of the open Retros: one card at a time, the model's line on
 * why it is here, the duplicates it stands for, and a thread to ask about it.
 * The card is the Lessons list's own; the Retros page passes it in.
 */
const meta: Meta<typeof LessonReviewStep> = {
  title: "Compositions/Lesson review",
  component: LessonReviewStep,
  decorators: [
    (Story) => (
      <div style={{ margin: 0, padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "48rem" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LessonReviewStep>;

const card = (
  <ul style={{ margin: 0, padding: 0, listStyle: "none" }}>
    <LessonCard
      item={{
        who: "fleet",
        landsIn: "armada",
        statement: "The gate compared the step against a stale local main.",
        title: "The gate blamed the Drone for Fleet's own mistake",
        what: "It compared the step against a local main two commits behind origin.",
        fix: "Compare against origin/main, where the branch is cut from.",
      }}
      answers={{
        agreeLabel: "Create Job",
        agreeTip: "Turn this into a Job that applies the change. It waits for your approval on the Board.",
        disagreeTip: "Discards it.",
        onAgree: fn(),
        onDisagree: fn(),
      }}
    />
  </ul>
);

const base: LessonReviewStepProps = {
  at: 3,
  of: 12,
  reason: "Armada's own fault cost two Jobs a rerun.",
  onSkip: fn(),
  onBack: fn(),
  children: card,
  thread: { turns: [], draft: "", onDraft: fn(), onSend: fn() },
};

export const First: Story = {
  name: "The first card",
  args: { ...base, at: 1, onBack: undefined },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("1 of 12")).toBeVisible();
    await expect(canvas.getByText("Armada's own fault cost two Jobs a rerun.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Back" })).toBeDisabled();
  },
};

export const Merged: Story = {
  name: "Standing for duplicates",
  args: {
    ...base,
    merged: [
      { id: "a", label: "Job 2", title: "The gate measured from local main" },
      { id: "b", label: "Job 4", title: "A stale main blamed the Drone" },
    ],
    duplicates: { checked: false, onChange: fn() },
  },
  play: async ({ canvas, args }) => {
    const seen = canvas.getByRole("button", { name: "Also seen in 2 other Jobs" });
    await expect(seen).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(seen);
    await expect(canvas.getByText("The gate measured from local main")).toBeVisible();
    const box = canvas.getByRole("checkbox", { name: "Also reject the 2 duplicates" });
    await expect(box).not.toBeChecked();
    await userEvent.click(box);
    await expect(args.duplicates?.onChange).toHaveBeenCalledWith(true);
  },
};

export const Thread: Story = {
  name: "A thread under the card",
  args: {
    ...base,
    thread: {
      turns: [
        { role: "person", text: "Which Jobs did this cost?" },
        { role: "fleet", text: "Job 3 and Job 2, by the gate's two runs in the record." },
      ],
      draft: "",
      onDraft: fn(),
      onSend: fn(),
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Which Jobs did this cost?")).toBeVisible();
    await expect(canvas.getByText(/Job 3 and Job 2/)).toBeVisible();
  },
};

export const Asking: Story = {
  name: "A question out",
  args: { ...base, thread: { turns: [{ role: "person", text: "Why two commits?" }], draft: "", onDraft: fn(), onSend: fn(), pending: true } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("textbox", { name: "Ask about this item" })).toBeDisabled();
  },
};

export const Refused: Story = {
  name: "A question refused",
  args: {
    ...base,
    thread: { turns: [], draft: "Why?", onDraft: fn(), onSend: fn(), failure: "That question is over 2000 characters. Nothing was asked." },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/over 2000 characters/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};

export const Waiting: StoryObj<typeof LessonReviewWait> = {
  name: "Fleet is reading",
  render: () => <LessonReviewWait />,
};

export const Failed: StoryObj<typeof LessonReviewWait> = {
  name: "The review failed",
  render: () => <LessonReviewWait failure="Fleet could not make the review. Nothing was changed." onRetry={fn()} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/could not make the review/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Retry" })).toBeVisible();
  },
};

export const Done: StoryObj<typeof LessonReviewEnd> = {
  name: "Nothing left to review",
  render: () => <LessonReviewEnd title="Nothing left to review" onList={fn()} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Nothing left to review")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Back to the list" })).toBeVisible();
  },
};

export const SetAside: StoryObj<typeof ReviewSetAside> = {
  name: "Set aside, folded",
  render: () => (
    <ReviewSetAside
      items={[
        { id: "x", title: "A Drone waited on grep", why: "Already allowed by a later Update Kit.", onPutBack: fn() },
        { id: "y", title: "A docs edit ran every Rust test", why: "Two weeks old and fixed in the Manifest.", onPutBack: fn() },
      ]}
    />
  ),
  play: async ({ canvas }) => {
    const fold = canvas.getByRole("button", { name: "Set aside (2)" });
    await expect(canvas.queryByText(/Already allowed/)).toBeNull();
    await userEvent.click(fold);
    await expect(canvas.getByText(/Already allowed/)).toBeVisible();
    await expect(canvas.getAllByRole("button", { name: "Put back in the queue" })).toHaveLength(2);
  },
};
