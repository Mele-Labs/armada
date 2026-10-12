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

const SESSION_TIP = "Starts a Session on this repository and sends it this request now. It begins work at once.";

const answers = (agreeLabel: string, agreeTip: string, session = false): LessonAnswers => ({
  agreeLabel,
  agreeTip,
  disagreeTip: "Discards it.",
  onAgree: fn(),
  onDisagree: fn(),
  ...(session ? { startSession: { tip: SESSION_TIP, onStart: fn() } } : {}),
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
    answers: answers("Create Job", JOB_TIP, true),
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(words(canvasElement)).toEqual(["Fleet", "Armada"]);
    await expect(canvas.getByRole("button", { name: "Create Job" })).toBeVisible();
    // The Session is the other way to agree, and sits between the Job and the rejection.
    await expect(canvas.getByRole("button", { name: "Start Session" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Reject change" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Accept" })).toBeNull();
    const names = [...canvasElement.querySelectorAll("button")].map((one) => one.textContent?.trim());
    await expect(names.slice(0, 3)).toEqual(["Create Job", "Start Session", "Reject change"]);
  },
};

/** **A Session being started.** Its own button sweeps and the other two are held. */
export const StartingSession: Story = {
  name: "An item whose Session is being started",
  args: {
    item: {
      who: "fleet",
      landsIn: "manifest",
      statement: "A docs edit set off all 4211 Rust tests.",
      title: "A docs edit ran every Rust test",
      what: "One docs edit set off all 4211 Rust tests.",
      fix: "Run only xtask's tests when only apps/ or packages/ change.",
    },
    answers: { ...answers("Create Job", JOB_TIP, true), pressing: "session" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Create Job" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Reject change" })).toBeDisabled();
  },
};

/** **Refused.** Fleet's words, with all three buttons kept. */
export const SessionRefused: Story = {
  name: "An item whose Session Fleet refused",
  args: {
    item: {
      who: "fleet",
      landsIn: "armada",
      statement: "The gate compared the step against a stale local main.",
      title: "The gate blamed the Drone for Fleet's own mistake",
    },
    answers: { ...answers("Create Job", JOB_TIP, true), refusal: "Armada is not served by this Fleet." },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Armada is not served by this Fleet.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Start Session" })).toBeEnabled();
    await expect(canvas.getByRole("button", { name: "Create Job" })).toBeEnabled();
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
    // Kit is no repository, so there is no Session to start.
    await expect(canvas.queryByRole("button", { name: "Start Session" })).toBeNull();
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

/** **Answered by a Session.** `Session started`, and a link that opens it, as Agreed links to its Job. */
export const SessionStarted: Story = {
  name: "An item answered by a Session",
  args: {
    item: {
      who: "fleet",
      landsIn: "armada",
      statement: "The gate compared the step against a stale local main.",
      title: "The gate blamed the Drone for Fleet's own mistake",
      what: "It compared the step against a local main two commits behind origin.",
      fix: "Compare against origin/main, where the branch is cut from.",
    },
    settled: { said: "Session started", session: { label: "Open Session", onOpen: fn() } },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Session started", { exact: true })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Open Session" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Start Session" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Create Job" })).toBeNull();
  },
};

/** **Started, but the request did not go.** Fleet's words, and the link to the Session still there. */
export const SessionNotSent: Story = {
  name: "A Session started whose request was not sent",
  args: {
    item: { who: "fleet", landsIn: "armada", statement: "s", title: "The gate blamed the Drone for Fleet's own mistake" },
    settled: { said: "Session started", session: { label: "Open Session", onOpen: fn() }, note: "The agent could not be reached." },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The agent could not be reached.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Open Session" })).toBeVisible();
  },
};

/** **A Kit item whose command was added.** Updated Kit, and the command in monospace beside it. */
export const UpdatedKit: Story = {
  name: "A Kit item that updated Kit",
  args: {
    item: {
      who: "drone",
      landsIn: "kit",
      statement: "A Drone waited on grep.",
      title: "A Drone had to wait for grep to be allowed",
      what: "It asked to run grep on a check log, and the step waited until you allowed it.",
      fix: "Add grep on .armada/checks to the allowlist.",
    },
    settled: { said: "Updated Kit", command: "grep" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Updated Kit", { exact: true })).toBeVisible();
    await expect(canvas.getByText("grep", { exact: true }).classList.contains("mono")).toBe(true);
    await expect(canvas.queryByRole("button", { name: "Update Kit" })).toBeNull();
  },
};
