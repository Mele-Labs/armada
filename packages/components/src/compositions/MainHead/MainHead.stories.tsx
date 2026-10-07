import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { FixingMainMark, MainMark, MainRedBand, type MainRed, type RecentJob } from "./MainHead";

/**
 * Main's state at the head of the merge line. Green is a mark and its tooltip. Red is a frame with
 * the Check, the test and the merge that turned it red, each a link, then either the Job working on
 * it or the two ways to hand it to one. Drawn ahead of Fleet.
 */
const meta = {
  title: "Compositions/Main head",
  component: MainRedBand,
  parameters: { layout: "padded" },
} satisfies Meta<typeof MainRedBand>;

export default meta;
type Story = StoryObj<typeof meta>;

const PULL = "https://git.example/armada/pull/";

const JOB = { id: "job-1", title: "Cache the manifest read between dispatches" };

const MERGE = {
  number: 1812,
  url: `${PULL}1812`,
  branch: "nick/ports-cleanup",
  branchUrl: "https://git.example/armada/tree/nick/ports-cleanup",
};

const RED: MainRed = {
  check: "screens_test",
  test: "merge-line.test.ts > folds a line's failed Check onto the panel",
  testUrl: "https://git.example/armada/blob/main/packages/screens/src/merge-line.test.ts",
  merge: MERGE,
};

const RECENT: RecentJob[] = [
  { id: "job-3", title: "Debounce the Job Board's resize handler", branch: "armada/3-debounce" },
  { ...JOB, branch: "armada/1-cache" },
  { id: "job-2", title: "Fold the two notification routes into one", branch: "armada/2-notify" },
];

const args = {
  main: { state: "red", red: RED } as const,
  recent: RECENT,
  onOpenLink: fn(),
  onOpenCheck: fn(),
  onOpenJob: fn(),
  onFix: fn(),
};

/** Main is green: the mark, and its tooltip, are all that is drawn. */
export const Green: Story = {
  args,
  render: () => <MainMark main={{ state: "green" }} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Main is green" })).toBeVisible();
  },
};

/**
 * Main is red and nobody has it: the facts, each a link, and the two ways to hand it to a Job. The
 * Check opens its log; the others open their address.
 */
export const RedWithNobodyOnIt: Story = {
  args,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "screens_test" }));
    await expect(args.onOpenCheck).toHaveBeenCalledWith("screens_test");
    await userEvent.click(canvas.getByRole("link", { name: "#1812" }));
    await expect(args.onOpenLink).toHaveBeenCalledWith(`${PULL}1812`);
    await userEvent.click(canvas.getByRole("link", { name: "nick/ports-cleanup" }));
    await expect(args.onOpenLink).toHaveBeenCalledWith("https://git.example/armada/tree/nick/ports-cleanup");
    await expect(canvas.getByRole("button", { name: "Dispatch a new Job" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Send back to a Job" })).toBeVisible();
  },
};

/** A Job has it: the two ways are gone, and the Job is a link. Nobody is asked anything. */
export const RedWithAJobOnIt: Story = {
  args: { ...args, main: { state: "red", red: { ...RED, merge: { ...MERGE, job: JOB } }, taken: JOB } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: "Dispatch a new Job" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Send back to a Job" })).toBeNull();
    await expect(canvas.getByRole("img", { name: "Working on it" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: JOB.title }));
    await expect(args.onOpenJob).toHaveBeenCalledWith("job-1");
  },
};

/** A new Job, its brief filled in from what failed and left there to edit. */
export const DispatchANewJob: Story = {
  args,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a new Job" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Dispatch a Job to fix main" });
    const attached = within(dialog).getByRole("list", { name: "Attached" });
    await expect(within(attached).getAllByRole("listitem")).toHaveLength(4);
    await expect(within(dialog).getByRole("textbox", { name: "Brief" })).toHaveValue(
      "screens_test fails on main.\nTest: merge-line.test.ts > folds a line's failed Check onto the panel\nMerged in #1812 (nick/ports-cleanup).",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Dispatch" }));
    await expect(args.onFix).toHaveBeenCalledWith({ kind: "new", request: expect.stringContaining("screens_test fails on main.") });
    // Sent: the dialog is put away.
    await expect(within(document.body).queryByRole("dialog")).toBeNull();
  },
};

/**
 * The work goes back to an earlier Job. The culprit's own comes first wherever it is among the
 * recent, and nothing is chosen until the owner chooses.
 */
export const SendBackToAJob: Story = {
  args: { ...args, main: { state: "red", red: { ...RED, merge: { ...MERGE, job: JOB } } } },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Send back to a Job" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back to a Job" });
    const jobs = within(dialog).getAllByRole("radio");
    await expect(jobs[0]).toHaveAccessibleName(/^Cache the manifest read between dispatches/);
    await expect(within(dialog).getByRole("img", { name: "Merged #1812" })).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: "Send back" })).toBeDisabled();
    await userEvent.click(jobs[0]!);
    await userEvent.click(within(dialog).getByRole("button", { name: "Send back" }));
    await expect(args.onFix).toHaveBeenCalledWith({ kind: "back", job: "job-1" });
  },
};

/** A person's pull request has no Job: the recent Jobs come as they were, newest first. */
export const SendBackToAPersonsPullRequest: Story = {
  args,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Send back to a Job" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back to a Job" });
    const jobs = within(dialog).getAllByRole("radio");
    await expect(jobs[0]).toHaveAccessibleName(/^Debounce the Job Board's resize handler/);
    await expect(within(dialog).queryByRole("img", { name: /^Merged/ })).toBeNull();
  },
};

/**
 * The Job's mark beside its badge: a hammer while it works on main, a shield once main is green. An
 * icon and its tooltip, no phrase.
 */
export const JobMarks: Story = {
  args,
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-4)" }}>
      <FixingMainMark state="fixing" said="Fixing main: screens_test failed after #1812" />
      <FixingMainMark state="fixed" said="Fixed main in #1815" />
    </div>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: /^Fixing main/ }).querySelector("svg")).not.toBeNull();
    await expect(canvas.getByRole("img", { name: /^Fixed main/ }).querySelector("svg")).not.toBeNull();
  },
};

/**
 * CI that maps to no Manifest Check: the first row is the CI job as the forge names it, and with no
 * test read out of the log there is no Test row. The dialog's attached facts say the same.
 */
export const UnmappedCiJob: Story = {
  args: { ...args, main: { state: "red", red: { check: "test-all", unmapped: true, merge: RED.merge } } },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("CI job")).toBeVisible();
    await expect(canvas.queryByText("Check")).toBeNull();
    await expect(canvas.queryByText("Test")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a new Job" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Dispatch a Job to fix main" });
    const attached = within(dialog).getByRole("list", { name: "Attached" });
    await expect(within(attached).getAllByRole("listitem")).toHaveLength(3);
    await expect(within(attached).getByText("CI job")).toBeVisible();
    await expect(within(dialog).getByRole("textbox", { name: "Brief" })).toHaveValue(
      "test-all fails on main.\nMerged in #1812 (nick/ports-cleanup).",
    );
  },
};

/**
 * What the forge reports and nothing more: two CI jobs, neither a Check, one with a test read out of
 * its log, and no pull request to blame. Nothing is a link, and without a listener the two ways to
 * hand the red to a Job are not offered.
 */
export const RedFromCiAlone: Story = {
  args: {
    main: { state: "red", red: { check: "test-all", unmapped: true, also: [{ check: "lint-all", unmapped: true, test: "tests::one" }] } },
    recent: RECENT,
    onOpenLink: fn(),
    onOpenCheck: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByText("CI job")).toHaveLength(2);
    await expect(canvas.getByRole("button", { name: "test-all" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "lint-all" })).toBeVisible();
    await expect(canvas.getByText("tests::one")).toBeVisible();
    await expect(canvas.queryByRole("link")).toBeNull();
    await expect(canvas.queryByText("Broke in")).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Dispatch a new Job" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Send back to a Job" })).toBeNull();
  },
};

/**
 * Main is red and a newer commit's CI is still running, which may already have fixed it. The frame
 * is caution, one plain line names the pull request that is running, and the red's own rows stay
 * under it. Neither way to hand it to a Job is offered.
 */
export const HeldWhileChecksRun: Story = {
  args: { ...args, main: { state: "red", red: RED, checking: [{ commit: "d".repeat(40), number: 1852, url: `${PULL}1852` }] } },
  play: async ({ args, canvas, userEvent }) => {
    const band = canvas.getByRole("status", { name: "New checks are running on main" });
    await expect(within(band).getByText(/New checks are running on main:/)).toBeVisible();
    await userEvent.click(within(band).getByRole("link", { name: "#1852" }));
    await expect(args.onOpenLink).toHaveBeenCalledWith(`${PULL}1852`);
    await expect(canvas.queryByRole("status", { name: "Main is red" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "screens_test" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "#1812" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Dispatch a new Job" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Send back to a Job" })).toBeNull();
  },
};

/** Two newer runs, one a direct push: its commit stands in for a pull request nobody named. */
export const HeldWhileSeveralRun: Story = {
  args: {
    ...args,
    main: { state: "red", red: RED, checking: [{ commit: "d".repeat(40), number: 1853 }, { commit: "e".repeat(40) }] },
  },
  play: async ({ canvas }) => {
    const band = canvas.getByRole("status", { name: "New checks are running on main" });
    await expect(within(band).getByText("#1853")).toBeVisible();
    await expect(within(band).getByText("eeeeeeeeee")).toBeVisible();
  },
};

/** The mark beside the heading is caution while held, and says why. */
export const HeldMark: Story = {
  args,
  render: () => <MainMark main={{ state: "red", red: RED, checking: [{ commit: "d".repeat(40), number: 1852 }] }} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Main is red, new checks are running" })).toBeVisible();
  },
};
