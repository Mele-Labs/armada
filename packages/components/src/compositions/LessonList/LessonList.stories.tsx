import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { LessonList, type LessonRow } from "./LessonList";

/**
 * The Lessons page's list: what got in the way across Jobs, newest first. A
 * row opens its Job's retro, and nothing else on it acts.
 */
const meta: Meta<typeof LessonList> = {
  title: "Compositions/Lesson list",
  component: LessonList,
  args: { onOpen: fn() },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LessonList>;

const JOB_3 = { jobId: "01K6Q3JOB3", job: "Job 3", jobExact: "3-retire-two-guides", when: "Oct 2, 9:41 PM" };

const ROWS: LessonRow[] = [
  {
    id: "3-0",
    ...JOB_3,
    who: "fleet",
    statement: "The gate failed out_of_bounds on armada.yml, a line that came in with an upstream commit.",
  },
  {
    id: "3-1",
    ...JOB_3,
    who: "drone",
    statement: "The Drone handed in without ever seeing screens_test pass.",
  },
  {
    id: "3-2",
    ...JOB_3,
    who: "owner",
    statement: "The step waited 19 minutes on a command you were asked to allow.",
  },
  {
    id: "2-0",
    jobId: "01K6Q2JOB2",
    job: "Job 2",
    jobExact: "2-show-whats-running",
    when: "Oct 1, 8:40 PM",
    who: "owner",
    statement: "You read the plan twice before the Judge's question reached the dock.",
  },
];

/**
 * **Every kind of whose-way, and a press opens the Job's retro.** The marks
 * are named by their tooltips and by nothing drawn: a row carries no word for
 * whose way it got in.
 */
export const Listed: Story = {
  name: "Lessons from every retro",
  args: { rows: ROWS },
  play: async ({ canvas, args }) => {
    // A bare mark, named as an image and on hover, and by no word in the row.
    const marks = canvas.getAllByRole("img").map((one) => one.getAttribute("aria-label"));
    await expect(marks).toEqual(["Fleet", "Drone", "You", "You"]);
    await userEvent.click(canvas.getByText(/handed in without ever seeing/));
    await expect(args.onOpen).toHaveBeenCalledWith("01K6Q3JOB3");
    // The keyboard reaches a row and opens it the same way.
    const last = canvas.getAllByRole("row").at(-1);
    last?.focus();
    await userEvent.keyboard("{Enter}");
    await expect(args.onOpen).toHaveBeenLastCalledWith("01K6Q2JOB2");
  },
};

/** One Job's retro open beside the list: its rows read as selected. */
export const OneOpen: Story = {
  name: "Lessons with one retro open",
  args: { rows: ROWS, openJob: "01K6Q3JOB3" },
};

/** Nothing written yet. An empty slot stays empty: no table and no sentence. */
export const NothingYet: Story = {
  name: "Lessons before any retro",
  args: { rows: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("table")).toBeNull();
  },
};
