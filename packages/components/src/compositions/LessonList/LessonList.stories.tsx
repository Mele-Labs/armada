import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { LessonList, type LessonRow } from "./LessonList";

/**
 * The Retros page's list: what got in the way across Sessions, newest first, each
 * item a card the owner can agree or disagree with. The Session label opens its
 * retro.
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

const SESSION_3 = { sessionId: "s-01K6Q3", session: "Retire two guides · s-01K6Q3", sessionExact: "s-01K6Q3JOB3", when: "Oct 2, 9:41 PM" };

const answers = (agree?: { label: string; tip: string }) => ({
  ...(agree === undefined ? {} : { agree: { ...agree, onAgree: fn() } }),
  disagreeTip: "Discards it.",
  onDisagree: fn(),
  ...(agree === undefined ? { startSession: { tip: "Starts a Job on this repository and sends it this request now. It begins work at once.", onStart: fn() } } : {}),
});

const ROWS: LessonRow[] = [
  {
    id: "3-0",
    ...SESSION_3,
    who: "fleet",
    landsIn: "armada",
    statement: "The gate compared the step against a stale local main.",
    title: "The gate blamed the Drone for Fleet's own mistake",
    what: "It compared the step against a local main two commits behind origin, so two commits that edited armada.yml counted as the Drone's work.",
    fix: "Compare against origin/main, where the branch is cut from.",
    answers: answers(),
  },
  {
    id: "3-1",
    ...SESSION_3,
    who: "drone",
    landsIn: "kit",
    statement: "A Drone waited on grep.",
    title: "A Drone had to wait for grep to be allowed",
    what: "It asked to run grep on a check log, and the step waited until you allowed it.",
    fix: "Add grep on .armada/checks to the allowlist.",
    answers: answers({ label: "Accept", tip: "Saves it under Accepted." }),
  },
  {
    id: "3-2",
    ...SESSION_3,
    who: "fleet",
    landsIn: "manifest",
    statement: "A docs edit ran every Rust test.",
    title: "A docs edit ran every Rust test",
    what: "One docs edit set off all 4211 Rust tests and a 7.5 minute compile.",
    fix: "Run only xtask's tests when only apps/ or packages/ change.",
    answers: answers(),
  },
  {
    id: "2-0",
    sessionId: "s-01K6Q2",
    session: "Show what is running · s-01K6Q2",
    sessionExact: "s-01K6Q2JOB2",
    when: "Oct 1, 8:40 PM",
    who: "owner",
    statement: "The Judge's question waited in the dock while the plan was read twice.",
  },
];

/**
 * **Every kind of whose-way and place, each as a word with its mark beside it.**
 * The last item predates the headline and draws its statement alone.
 */
export const Listed: Story = {
  name: "Retros from every Session",
  args: { rows: ROWS },
  play: async ({ canvas, args }) => {
    const marks = canvas.getAllByRole("img").map((one) => one.getAttribute("aria-label"));
    await expect(marks).toEqual([
      "Fleet",
      "Lands in Armada",
      "Drone",
      "Lands in Kit",
      "Fleet",
      "Lands in the Manifest",
      "You",
    ]);
    await expect(canvas.getAllByRole("button", { name: "Start Job" })).toHaveLength(2);
    await userEvent.click(canvas.getByRole("button", { name: "Show what is running · s-01K6Q2" }));
    await expect(args.onOpen).toHaveBeenCalledWith("s-01K6Q2", "2-0");
  },
};

/** An answered item stays while it is on screen: its last state, and a link to the Session it started. */
export const Agreed: Story = {
  name: "An Armada item answered by a Session",
  args: {
    rows: [
      {
        id: "3-0",
        ...SESSION_3,
        who: "fleet",
        landsIn: "armada",
        statement: "The gate compared the step against a stale local main.",
        title: "The gate blamed the Drone for Fleet's own mistake",
        what: "It compared the step against a local main two commits behind origin.",
        fix: "Compare against origin/main, where the branch is cut from.",
        settled: { said: "Job started", session: { label: "Open Job", onOpen: fn() } },
      },
    ],
  },
};

/** Nothing written yet. An empty slot stays empty: no list and no sentence. */
export const NothingYet: Story = {
  name: "Retros before any is written",
  args: { rows: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("list")).toBeNull();
  },
};
