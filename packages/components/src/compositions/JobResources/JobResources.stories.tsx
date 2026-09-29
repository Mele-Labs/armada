import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor, within } from "storybook/test";
import type { JobExamined, Look } from "@armada/protocol";
import { GUIDE_LOOK, GUIDE_PULSE } from "../../guides";

import { JobResources, type PulseReading } from "./JobResources";

const meta: Meta<typeof JobResources> = {
  title: "Compositions/Job resources",
  component: JobResources,
};
export default meta;

type Story = StoryObj<typeof JobResources>;

/** The branch one Job's Drone is working on. Every row here belongs to it. */
const BRANCH = "armada/01JOBHOLDS001";

/**
 * What the Job is running and what it is spending. **The caller's rows**, so
 * this component holds no rule about which five they are.
 */
const FIGURES = [
  { label: "Drones running", value: "1" },
  { label: "Checks running", value: "2" },
  { label: "Judges running", value: "0" },
  { label: "Spend", value: "~$2.41", detail: "of $5.00", apart: true },
  { label: "Turns", value: "34", detail: "of 120" },
  { label: "Processes", value: "3" },
];

/** The board, as the caller derives it. One place, so a story cannot drift. */
function reading(over: Partial<PulseReading> = {}): PulseReading {
  return {
    held: "running",
    readAt: "2026-09-04T04:07:00.366Z",
    processes: [
      {
        pid: 41233,
        command: "node",
        owner: BRANCH,
        cpuPercent: 12.4,
        memoryBytes: 402_653_184,
        runningFor: "06:12",
        recorded: true,
      },
      {
        pid: 41287,
        command: "cargo",
        owner: BRANCH,
        cpuPercent: 98.7,
        memoryBytes: 838_860_800,
        runningFor: "00:41",
        recorded: false,
      },
    ],
    worktrees: [
      {
        path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
        branch: BRANCH,
        state: "on disk",
        bytes: 1_073_741_824,
      },
    ],
    logs: [{ kind: "job", owner: null, writing: true }],
    ...over,
  };
}

function look(over: Partial<Look> & Pick<Look, "asked" | "found">): Look {
  return { said: "", fields: [], ...over };
}

function examined(found: JobExamined["found"], looks: Look[]): JobExamined {
  return {
    job_id: "01JOBHOLDS001",
    looked_at: "2026-09-04T04:07:00.366Z",
    found,
    looks,
    // The wire's own reading, which the look carries back with it — not the
    // board above, which is what this component draws.
    resources: {
      job_id: "01JOBHOLDS001",
      read_at: "2026-09-04T04:07:00.366Z",
      held: "running",
      processes: [],
    },
  };
}

/**
 * **Nobody has pressed yet.** The figures are drawn and the question is not
 * answered, because looking walks a process table and a directory and an answer
 * that appeared unasked would be the automatic bound rather than the person's
 * half of it. **No line says nobody has asked** — beside `Updated 3s ago` it
 * read as a contradiction (owner, 29 Sep). That a look is free, and what its
 * three answers mean, is the `?` beside the act (#1602).
 */
export const NobodyHasAsked: Story = {
  args: {
    reading: reading(),
    figures: FIGURES,
    examined: null,
    age: "3s",
    onExamine: () => {},
  },
};

/**
 * **The reading a person came for.** A drone and the build it started, the one
 * fleet wrote down leading the list, and the disk the checkout has taken —
 * which is the figure with a second reason to exist, since seventy-four
 * worktrees once took 220 GB and nothing said so.
 */
export const WorkingAndSaidSo: Story = {
  args: {
    reading: reading(),
    age: "3s",
    examined: examined("working", [
      look({
        asked: "process",
        found: "working",
        said: "the process Fleet recorded is running",
        fields: [{ name: "pid", value: "41233" }],
      }),
      look({ asked: "worktree", found: "working", said: "the worktree is on disk" }),
      look({ asked: "span", found: "working", said: "waiting for the step to finish" }),
    ]),
    onExamine: () => {},
  },
  /**
   * **The head carries CPU and memory across every row, and not a count** —
   * the rows are under it (owner, 29 Sep). Two figures a bare `%` and `GiB`
   * tell apart, so each is named by its tooltip rather than a label.
   * The worktree row says its state in text, so the dot is hidden and takes no
   * stop, and a pointer on it still reads the state.
   */
  play: async ({ canvas, userEvent }) => {
    const processes = within(canvas.getByRole("region", { name: "Processes" }));
    await expect(processes.getByText("111.1%")).toHaveAccessibleDescription("CPU usage");
    await expect(processes.getByText("1.2 GiB")).toHaveAccessibleDescription("Memory usage");
    const row = within(within(canvas.getByRole("region", { name: "Worktrees" })).getByRole("listitem"));
    // The dot's tooltip is the `on disk` not yet drawn; the other is the row's.
    const bubble = row.getAllByText("on disk").find((one) => !one.checkVisibility())!;
    const dot = bubble.parentElement!.parentElement!;
    await expect(dot).toHaveAttribute("aria-hidden", "true");
    await expect(dot).not.toHaveAttribute("tabindex");
    await userEvent.hover(dot);
    await waitFor(() => expect(bubble).toBeVisible());
    await expect(row.getByText(/^1\.0 GiB/)).toHaveAccessibleDescription("Size on disk");
  },
};

/**
 * **The state that took a terminal to establish.** The job reads running and
 * fleet holds no process for it — which as an empty table under a heading is
 * exactly how it went unnoticed, so it is a sentence in the error treatment
 * instead. This is the 4 Sep 2026 job, drawn.
 *
 * **The look's own words say it, and no verdict over them** — the owner took
 * the sentence out of the head on 29 Sep. The Processes head has nothing to
 * total and nothing to kill, so it draws neither.
 */
export const NothingIsRunning: Story = {
  args: {
    reading: reading({ held: "none", processes: [] }),
    onKillAll: fn(),
    age: "1s",
    examined: examined("not_working", [
      look({
        asked: "process",
        found: "not_working",
        said: "this Job is running and Fleet recorded no process for it",
        fields: [{ name: "processes", value: "0" }],
      }),
      look({
        asked: "writing",
        found: "cannot_tell",
        said: "nothing has been written to this Job's log lately, which settles nothing on its own",
        fields: [{ name: "seconds_ago", value: "372" }],
      }),
    ]),
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/Fleet recorded no process for it/)).toBeVisible();
    await expect(canvas.queryByText(/doing what it should be/)).toBeNull();
    const processes = within(canvas.getByRole("region", { name: "Processes" }));
    await expect(processes.getByText(/Fleet holds no process for this job/)).toBeVisible();
    // No `0.0% · 0 B` over an empty list, and no kill with nothing to end.
    await expect(processes.queryByText(/%/)).toBeNull();
    await expect(processes.queryByRole("button", { name: /kill/i })).toBeNull();
  },
};

/**
 * **The answer that has to be said rather than implied.** Two of the five looks
 * can never report a fault — a job's log is quiet while a drone works steadily,
 * and a quiet drone is what a long command looks like — so an examination that
 * finds nothing wrong says which checks came back short rather than reporting
 * that everything looks fine.
 *
 * **Each look says it in its own words**, since the sentence over them went on
 * 29 Sep: the ones that came back short are listed by what they asked.
 */
export const SomeChecksCouldNotTell: Story = {
  args: {
    reading: reading(),
    age: "5s",
    examined: examined("cannot_tell", [
      look({
        asked: "process",
        found: "working",
        said: "the process Fleet recorded is running",
      }),
      look({
        asked: "writing",
        found: "cannot_tell",
        said: "nothing has been written to this Job's log lately, which settles nothing on its own",
      }),
      look({
        asked: "silence",
        found: "cannot_tell",
        said: "no Drone is in the slot",
      }),
    ]),
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The liveness watch")).toBeVisible();
    await expect(canvas.getByText("no Drone is in the slot")).toBeVisible();
    await expect(canvas.getByText("What was written")).toBeVisible();
    // Short is not fine, and nothing over the list rounds it into one.
    await expect(canvas.queryByText(/doing what it should be/)).toBeNull();
  },
};

/**
 * **A checkout too large to walk inside the bound.** Said rather than reported
 * as nothing: a size that did not arrive is a walk that ran long, which on a
 * worktree is itself worth knowing — and a zero here would be the one figure a
 * person acts on directly, wrong.
 */
export const TheWorktreeWasNotMeasuredInTime: Story = {
  args: {
    reading: reading({
      worktrees: [
        {
          path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
          branch: BRANCH,
          state: "on disk",
        },
      ],
    }),
    age: "2s",
    examined: null,
    onExamine: () => {},
  },
};

/** A look already out. A second press does not send a second act. */
export const LookingNow: Story = {
  args: { reading: reading(), age: "0s", examined: null, looking: true, onExamine: () => {} },
};

/**
 * **Nothing has been read.** Not the same as a job holding nothing, which is
 * the distinction the whole panel turns on — so the note says which rather than
 * drawing an empty table.
 *
 * Fleet is answering here and this one read did not come back, so the act
 * stays: another attempt is a reasonable move.
 */
export const NothingHasBeenRead: Story = {
  args: {
    reading: null,
    note: "Fleet did not answer, so what this job holds is unknown.",
    examined: null,
    onExamine: () => {},
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Refresh/ })).toBeVisible();
    // The owner's 28 Sep word: the button is `Refresh`, drawn as its glyph
    // since 29 Sep with the word as its name.
    await expect(canvas.queryByRole("button", { name: /Look now/ })).toBeNull();
    // No headline at all since 29 Sep: a look not run is not a failure, and
    // the owner took the sentence that said so out of the head.
    await expect(canvas.queryByText(/has not been looked at/)).toBeNull();
    await expect(canvas.queryByText(/Nobody has asked/)).toBeNull();
    // The reason there is no reading still reads, in the card.
    await expect(canvas.getByText(/Fleet did not answer/)).toBeVisible();
  },
};

/**
 * **Fleet is the thing that did not answer, so there is nothing to ask.** The
 * act is gone rather than greyed: it asks Fleet, and pressing it could only
 * ask the thing that is silent. A disabled button with no sentence beside it
 * is the same dead end drawn quieter, which is what this state was before.
 *
 * **Degraded and not a fault.** Amber, no red, because restarting Fleet is the
 * wrong move when the process is alive and only the connection stopped — and
 * the status bar is the one surface that says which of the two this is.
 *
 * **Not `lookFailed`.** That says one attempt did not come back, which invites
 * another. This says attempts are not the shape of the problem.
 */
export const FleetIsNotAnswering: Story = {
  args: { reading: null, examined: null, nothingToAsk: "no_answer", onExamine: () => {} },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet is not answering, so there is nothing to ask/),
    ).toBeVisible();
    await expect(canvas.getByText(/Nothing here is a reading of this job/)).toBeVisible();
    // The whole point: no control that asks Fleet, disabled or otherwise.
    await expect(canvas.queryByRole("button", { name: /Refresh/ })).toBeNull();
    // The look's `?` goes with the act. Nothing to ask means nothing to explain.
    await expect(canvas.queryByRole("button", { name: new RegExp(GUIDE_LOOK.title) })).toBeNull();
    // The board's own `?` stays. What Pulse is does not depend on Fleet
    // answering, and a person who arrived here confused is the likeliest
    // reader there is.
    await expect(canvas.getByRole("button", { name: new RegExp(GUIDE_PULSE.title) })).toBeVisible();
  },
};

/**
 * **Fleet answered, and Bridge could not read it.** The second reading, and it
 * is not the first said differently: Fleet is demonstrably up here, so "not
 * answering" would be false and pointing at the status bar would send a reader
 * to a line saying Fleet running.
 *
 * **Fleet being alive is not a reason to keep the act.** What stops the answer
 * is the two builds disagreeing about this route, so the same request meets
 * the same disagreement and a restart brings back the Fleet that caused it.
 * Attempts are not the shape of this problem either, which is why it withdraws
 * the control rather than reading as one failed look.
 *
 * **Still degraded and still not red**, for the reason an unreachable Fleet is
 * not: what has failed is this panel's ability to show a reading.
 */
export const BridgeCouldNotReadTheAnswer: Story = {
  args: { reading: null, examined: null, nothingToAsk: "unreadable", onExamine: () => {} },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText(/Fleet answered, and Bridge could not read the answer/),
    ).toBeVisible();
    await expect(canvas.getByText(/rebuilding both is what settles it/)).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Refresh/ })).toBeNull();
    // The two readings must not draw as one message. Neither the sentence that
    // says Fleet is silent nor the pointer at a bar reading "Fleet running".
    await expect(canvas.queryByText(/Fleet is not answering/)).toBeNull();
    await expect(canvas.queryByText(/The status bar names/)).toBeNull();
  },
};

/**
 * **A Job running several Drones.** One checkout per member, one process per
 * member, and the owner column is what joins them — *which of these is eating
 * the machine* cannot be answered by a table that names no owner.
 *
 * The first process belongs to no checkout. It is drawn as unplaced rather
 * than folded into the first row, which would name the wrong member.
 */
export const SeveralMembers: Story = {
  args: {
    reading: reading({
      processes: [
        {
          pid: 52_118,
          command: "node",
          owner: "armada/22-give-the-store-one-shape",
          cpuPercent: 12.4,
          memoryBytes: 486_539_264,
          runningFor: "06:12",
          recorded: true,
        },
        {
          pid: 52_640,
          command: "node",
          owner: "armada/24-drop-the-store-singleton",
          cpuPercent: 41.9,
          memoryBytes: 712_179_712,
          runningFor: "01:40",
          recorded: false,
        },
        {
          pid: 52_711,
          command: "cargo",
          owner: null,
          cpuPercent: 98.7,
          memoryBytes: 838_860_800,
          runningFor: "00:41",
          recorded: false,
        },
      ],
      worktrees: [
        {
          path: "/Users/user/armada/.armada/worktrees/22-give-the-store-one-shape",
          branch: "armada/22-give-the-store-one-shape",
          state: "merged",
          bytes: 1_020_054_016,
        },
        {
          path: "/Users/user/armada/.armada/worktrees/23-read-the-store-through-selectors",
          branch: "armada/23-read-the-store-through-selectors",
          state: "waiting on you",
          bytes: 980_321_280,
        },
        {
          path: "/Users/user/armada/.armada/worktrees/24-drop-the-store-singleton",
          branch: "armada/24-drop-the-store-singleton",
          state: "on disk",
        },
      ],
      logs: [
        { kind: "job", owner: null, bytes: 184_320, writing: false },
        { kind: "drone", owner: "armada/24-drop-the-store-singleton", bytes: 2_097_152, writing: true },
      ],
    }),
    figures: [
      { label: "Drones", value: "2 running" },
      { label: "Checks", value: "1 running" },
      { label: "Judges", value: "1 out" },
    ],
    age: "4s",
    examined: null,
    onExamine: () => {},
  },
  /**
   * **The owner is the reason this table has five columns.** A row that lost
   * it would render identically to the one-Drone case and read as correct, so
   * the assertion is that each branch is on screen and that the process
   * nothing placed says so instead of borrowing one.
   */
  play: async ({ canvas }) => {
    // Twice: once as a process's owner, once as a checkout. One occurrence
    // is the owner column gone, which renders as the one-Drone case and reads
    // as correct.
    expect(canvas.getAllByText("armada/22-give-the-store-one-shape")).toHaveLength(2);
    await expect(canvas.getByText("not placed")).toBeVisible();
    await expect(canvas.getByText("being written", { exact: true })).toBeVisible();
    // Whose logs, over every sub job by default and each one that owns a row.
    const logs = within(canvas.getByRole("region", { name: "Job logs" }));
    const whose = logs.getByRole("combobox", { name: "Which sub job" });
    await expect(whose).toHaveValue("All sub jobs");
    await expect(
      within(whose).getAllByRole("option").map((one) => one.textContent),
    ).toEqual(["All sub jobs", "armada/24-drop-the-store-singleton"]);
    // Both rows here carry `bytes`, and the list draws no size for either:
    // Fleet does not send it yet, so the column went (owner, 29 Sep).
    await expect(logs.queryAllByText(/\d (B|KiB|MiB|GiB)$/)).toHaveLength(0);
  },
};

/**
 * **A Job that has written nothing.** Its own sentence rather than an empty
 * list: a region that vanished when it held nothing would make "no logs yet"
 * and "this build does not draw logs" the same screen.
 */
export const NothingHasBeenWritten: Story = {
  args: {
    reading: reading({ logs: [] }),
    figures: FIGURES,
    age: "1s",
    examined: null,
    onExamine: () => {},
  },
};

/**
 * **Where the host can act on a row.** A held kill on each process and one for
 * all of them in the head, and `Open` on the checkout and the Job's own log.
 * Each is drawn only where its callback is, so every story above is the
 * surface that offers none.
 */
export const WithItsActs: Story = {
  args: {
    reading: reading({
      worktrees: [
        {
          path: "/Users/user/armada/.armada/worktrees/01JOBHOLDS001",
          branch: BRANCH,
          state: "1 drone working",
          working: true,
          bytes: 1_073_741_824,
          open: "worktree",
        },
      ],
      logs: [
        { kind: "job", owner: null, writing: false, open: "log" },
        {
          kind: "brief",
          owner: null,
          about: "implement · no_drift",
          writing: false,
          open: { kept: "steps/implement/briefs/no_drift-1.md", what: "brief" },
        },
      ],
    }),
    figures: FIGURES,
    age: "3s",
    examined: null,
    onExamine: () => {},
    onOpen: fn(),
    onKillProcess: fn(),
    onKillAll: fn(),
  },
};
