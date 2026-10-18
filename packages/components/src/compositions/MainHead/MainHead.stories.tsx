import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { FixingMainMark, MainMark, MainRedBand, type MainRed, type RecentSession } from "./MainHead";

/**
 * Main's state at the head of the merge line. Green is a mark and its tooltip. Red is a frame with
 * the Check, the test and the merge that turned it red, each a link, then either the Session working on
 * it or the ways to hand it to one. Drawn ahead of Fleet.
 */
const meta = {
  title: "Compositions/Main head",
  component: MainRedBand,
  parameters: { layout: "padded" },
} satisfies Meta<typeof MainRedBand>;

export default meta;
type Story = StoryObj<typeof meta>;

const PULL = "https://git.example/armada/pull/";

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

const SESSION = { id: "session-1", title: "Show who merged on the head" };

const RECENT_SESSIONS: RecentSession[] = [
  { id: "session-2", title: "Rework the Cockpit deck cards", branch: "armada/session-2" },
  { ...SESSION, branch: "armada/session-1" },
];

const args = {
  main: { state: "red", red: RED } as const,
  onOpenLink: fn(),
  onOpenCheck: fn(),
  onOpenSession: fn(),
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
 * Main is red and nobody has it: the facts, each a link, and the ways to hand it to a Session. The
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
    await expect(canvas.getByRole("button", { name: /^Send back to/ })).toBeVisible();
  },
};

/** A Session has it: the ways to hand it on are gone, and the Session is a link. Nobody is asked anything. */
export const RedWithASessionOnIt: Story = {
  args: { ...args, main: { state: "red", red: { ...RED, merge: { ...MERGE, session: SESSION } }, takenBySession: SESSION } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: "Dispatch a new Job" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: /^Send back/ })).toBeNull();
    await expect(canvas.getByRole("img", { name: "Working on it" })).toBeVisible();
    // The Session is named twice: as the merge's owner, and as the one fixing.
    const named = canvas.getAllByRole("button", { name: SESSION.title });
    await expect(named).toHaveLength(2);
    await userEvent.click(named[1]!);
    await expect(args.onOpenSession).toHaveBeenCalledWith("session-1");
  },
};

/** A new Session: the brief is filled in from what failed and handed on, for the Dispatch panel to open on. */
export const DispatchANewSession: Story = {
  args,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a new Job" }));
    await expect(args.onFix).toHaveBeenCalledWith({
      kind: "newSession",
      request: "screens_test fails on main.\nTest: merge-line.test.ts > folds a line's failed Check onto the panel\nMerged in #1812 (nick/ports-cleanup).",
    });
  },
};

/**
 * A Session's pull request merged: the row names it by its icon, the Send back beside it goes back
 * to that Session, the culprit first in its group and chosen, and the brief is the message sent.
 */
export const SendBackToASession: Story = {
  args: { ...args, recentSessions: RECENT_SESSIONS, main: { state: "red", red: { ...RED, merge: { ...MERGE, session: SESSION } } } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("img", { name: "Job" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: SESSION.title }));
    await expect(args.onOpenSession).toHaveBeenCalledWith("session-1");
    await userEvent.click(canvas.getByRole("button", { name: "Send back" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back" });
    const sessions = within(within(dialog).getByRole("radiogroup", { name: "Jobs" })).getAllByRole("radio");
    await expect(sessions[0]).toHaveAccessibleName(/^Show who merged on the head/);
    await expect(sessions[0]).toBeChecked();
    await expect(within(dialog).getByRole("textbox", { name: "Message" })).toHaveValue(
      "screens_test fails on main.\nTest: merge-line.test.ts > folds a line's failed Check onto the panel\nMerged in #1812 (nick/ports-cleanup).",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Send back" }));
    await expect(args.onFix).toHaveBeenCalledWith({ kind: "session", session: "session-1", message: expect.stringContaining("screens_test fails on main.") });
  },
};

/** A person's pull request has no Session: the recent Sessions come as they were, newest first, with none chosen. */
export const SendBackToAPersonsPullRequest: Story = {
  args: { ...args, recentSessions: RECENT_SESSIONS },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /^Send back to/ }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back" });
    const sessions = within(dialog).getAllByRole("radio");
    await expect(sessions[0]).toHaveAccessibleName(/^Rework the Cockpit deck cards/);
    await expect(within(dialog).queryByRole("img", { name: /^Merged/ })).toBeNull();
    await expect(within(dialog).getByRole("button", { name: "Send back" })).toBeDisabled();
    await expect(sessions.filter((one) => (one as HTMLInputElement).checked)).toHaveLength(0);
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
  args: { ...args, recentSessions: RECENT_SESSIONS, main: { state: "red", red: { check: "test-all", unmapped: true, merge: RED.merge } } },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("CI job")).toBeVisible();
    await expect(canvas.queryByText("Check")).toBeNull();
    await expect(canvas.queryByText("Test")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: /^Send back/ }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back" });
    const attached = within(dialog).getByRole("list", { name: "Attached" });
    await expect(within(attached).getAllByRole("listitem")).toHaveLength(3);
    await expect(within(attached).getByText("CI job")).toBeVisible();
  },
};

/**
 * What the forge reports and nothing more: two CI jobs, neither a Check, one with a test read out of
 * its log, and no pull request to blame. Nothing is a link, and without a listener the ways to
 * hand the red to a Session are not offered.
 */
export const RedFromCiAlone: Story = {
  args: {
    main: { state: "red", red: { check: "test-all", unmapped: true, also: [{ check: "lint-all", unmapped: true, test: "tests::one" }] } },
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
    await expect(canvas.queryByRole("button", { name: /^Send back/ })).toBeNull();
  },
};

/**
 * A direct push: no pull request, so no row of its own, and the head offers Send back to… beside
 * Dispatch, with nothing chosen.
 */
export const DirectPush: Story = {
  args: { ...args, main: { state: "red", red: { check: "screens_test" } } },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByText("Broke in")).toBeNull();
    await expect(canvas.getByRole("button", { name: "Dispatch a new Job" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Send back to…" }));
    const dialog = within(document.body).getByRole("dialog", { name: "Send the work back" });
    await expect(within(dialog).getByRole("button", { name: "Send back" })).toBeDisabled();
  },
};

/**
 * Main is red and a newer commit's CI is still running, which may already have fixed it. The frame
 * is caution, one plain line names the pull request that is running, and the red's own rows stay
 * under it. Neither way to hand it to a Session is offered.
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
    await expect(canvas.queryByRole("button", { name: /^Send back/ })).toBeNull();
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

/** Main is green and newer commits are running: the mark says so, and no band is drawn. */
export const GreenWhileChecksRun: Story = {
  args,
  render: () => <MainMark main={{ state: "green", checking: [{ commit: "d".repeat(40), number: 1852 }] }} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Main is green, new checks are running" })).toBeVisible();
  },
};

const NOW = Date.parse("2026-10-14T21:30:00Z");
const MINUTES = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString();
const ELSEWHERE = { machine: "work-mac-mini", holder: { kind: "session", id: "7be40d15" }, since: MINUTES(12) } as const;

/**
 * Another machine holds the red: its chip, with `monitor` naming the machine, and the claim's age.
 * Dispatch and Send back are gone; Take over is the one act.
 */
export const HeldElsewhere: Story = {
  args: { ...args, now: NOW, main: { state: "red", red: RED, elsewhere: ELSEWHERE } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Job 7be40d15, work-mac-mini" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "work-mac-mini" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Claimed 12m ago" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Take over" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Dispatch a new Job" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: /^Send back/ })).toBeNull();
  },
};

/** A claim an hour old or more reads as stale by its clock and its age, with no sentence. */
export const StaleClaim: Story = {
  args: { ...args, now: NOW, main: { state: "red", red: RED, elsewhere: { ...ELSEWHERE, holder: { kind: "session", id: "9c74d82e" }, since: MINUTES(3 * 60) } } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Job 9c74d82e, work-mac-mini" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Claimed 3h ago, stale" })).toBeVisible();
    await expect(canvas.queryByRole("img", { name: "Claimed 12m ago" })).toBeNull();
  },
};

/** Take over asks once, with the claim's age in the ask, and only then replaces the claim. */
export const TakeOverAsksOnce: Story = {
  args: { ...args, now: NOW, main: { state: "red", red: RED, elsewhere: { ...ELSEWHERE, since: MINUTES(3 * 60) } } },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Take over" }));
    await expect(args.onFix).not.toHaveBeenCalled();
    const ask = within(document.body).getByRole("dialog", { name: "Take over from work-mac-mini" });
    await expect(within(ask).getByRole("img", { name: "Claimed 3h ago, stale" })).toBeVisible();
    await userEvent.click(within(ask).getByRole("button", { name: "Take over" }));
    await expect(args.onFix).toHaveBeenCalledWith({ kind: "takeOver", request: expect.stringContaining("screens_test fails on main.") });
    await expect(within(document.body).queryByRole("dialog")).toBeNull();
  },
};

/** Cancelling the ask replaces nothing. */
export const TakeOverCancelled: Story = {
  args: { ...args, now: NOW, main: { state: "red", red: RED, elsewhere: ELSEWHERE } },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Take over" }));
    await userEvent.click(within(within(document.body).getByRole("dialog")).getByRole("button", { name: "Cancel" }));
    await expect(args.onFix).not.toHaveBeenCalled();
  },
};

/**
 * The store could not be reached when Dispatch was pressed: a plain row says so, beside the acts that
 * remain, and Try again sends the same press.
 */
export const ClaimNotMade: Story = {
  args: { ...args, onFix: fn(async () => "claim_not_made" as const) },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a new Job" }));
    await expect(await canvas.findByText("Not made, store unreachable")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Try again" }));
    await expect(args.onFix).toHaveBeenCalledTimes(2);
    await expect(args.onFix).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "newSession" }));
  },
};

/** Another machine's claim was not in the hub when Dispatch was pressed: a plain row, and Try again. */
export const ClaimTakenElsewhere: Story = {
  args: { ...args, onFix: fn(async () => "taken_elsewhere" as const) },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Dispatch a new Job" }));
    await expect(await canvas.findByText("Held on another machine")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Try again" })).toBeVisible();
  },
};
