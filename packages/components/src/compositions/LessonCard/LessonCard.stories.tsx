import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { LessonCard, type LessonAnswers } from "./LessonCard";

/**
 * One retro item, read top to bottom: one arrow from whose way it got in to
 * where the fix lands, a headline, what happened, what would change, and two
 * answers. The Retros page and a Job's retro sheet draw this card.
 */
const meta: Meta<typeof LessonCard> = {
  title: "Compositions/Lesson card",
  component: LessonCard,
  decorators: [
    (Story) => (
      <ul style={{ margin: 0, padding: "var(--space-6)", background: "var(--surface-canvas)", listStyle: "none" }}>
        <Story />
      </ul>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LessonCard>;

const JOB_TIP = "Turn this into a Job that applies the change. It waits for your approval on the Board.";

const answers = (agreeLabel: string, agreeTip: string): LessonAnswers => ({
  agreeLabel,
  agreeTip,
  disagreeTip: "Discards it.",
  onAgree: fn(),
  onDisagree: fn(),
});

const words = (root: HTMLElement) =>
  [...root.querySelectorAll(".armada-lesson__word")].map((one) => one.textContent);

/** **Armada.** The button makes a Job, so it says so. */
export const Armada: Story = {
  name: "An Armada item",
  args: {
    item: {
      who: "fleet",
      landsIn: "armada",
      statement: "The gate compared the step against a stale local main.",
      title: "The gate blamed the Drone for Fleet's own mistake",
      what: "It compared the step against a local main two commits behind origin.",
      fix: "Compare against origin/main, where the branch is cut from.",
    },
    answers: answers("Create Job", JOB_TIP),
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["Fleet", "Armada"]);
    await expect(canvas.getByRole("button", { name: "Create Job" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Reject change" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Accept" })).toBeNull();
  },
};

/** **Kit.** Nothing is dispatched, so the button accepts. */
export const Kit: Story = {
  name: "A Kit item",
  args: {
    item: {
      who: "drone",
      landsIn: "kit",
      statement: "A Drone waited on grep.",
      title: "A Drone had to wait for grep to be allowed",
      what: "It asked to run grep on a check log, and the step waited until you allowed it.",
      fix: "Add grep on .armada/checks to the allowlist.",
    },
    answers: answers("Accept", "Saves it under Accepted."),
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["Drone", "Kit"]);
    await expect(canvas.getByRole("button", { name: "Accept" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Create Job" })).toBeNull();
  },
};

/** **Every field**, with the cited rows behind Evidence. */
export const AllFields: Story = {
  name: "An item with a title, what happened, a fix and evidence",
  args: {
    item: {
      who: "fleet",
      landsIn: "manifest",
      statement: "A docs edit set off all 4211 Rust tests.",
      title: "A docs edit ran every Rust test",
      what: "One docs edit set off all 4211 Rust tests and a 7.5 minute compile.",
      fix: "Run only xtask's tests when only apps/ or packages/ change.",
      cites: [{ id: "check:2", name: "rust_test", mono: true, detail: "every Rust test ran", when: "Oct 2, 9:31 PM" }],
    },
    answers: answers("Create Job", JOB_TIP),
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["Fleet", "Manifest"]);
    await expect(canvas.getByText("What would change")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Create Job" })).toBeVisible();
    await expect(canvas.queryByText("rust_test")).toBeNull();
  },
};

/** **Written before the headline.** The statement is the body and no place is named, so no arrow and no buttons. */
export const Old: Story = {
  name: "An old item with only a statement",
  args: {
    item: { who: "owner", statement: "The Judge's question waited in the dock while the plan was read twice." },
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["You"]);
    await expect(canvasElement.querySelector(".armada-lesson__arrow")).toBeNull();
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};

/** **Answered.** Its last state and a link to the Job it proposed, and no buttons. */
export const Agreed: Story = {
  name: "An agreed item",
  args: {
    item: {
      who: "fleet",
      landsIn: "armada",
      statement: "The gate compared the step against a stale local main.",
      title: "The gate blamed the Drone for Fleet's own mistake",
      what: "It compared the step against a local main two commits behind origin.",
      fix: "Compare against origin/main, where the branch is cut from.",
    },
    settled: { said: "Agreed", job: { label: "Proposed Job", onOpen: fn() } },
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["Fleet", "Armada"]);
    await expect(canvas.getByRole("button", { name: "Proposed Job" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Create Job" })).toBeNull();
  },
};
