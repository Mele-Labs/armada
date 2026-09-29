import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { FileCheck, FileDiff } from "lucide-react";
import { useState } from "react";
import { JobLedger, type JobLedgerRow } from "./JobLedger";

/**
 * One table of everything that happened to a Job, newest first, with a filter
 * menu that narrows it and the open row in a sheet over it.
 *
 * The rows, their words and which filter each answers to are the caller's.
 * This decides the chrome, where the panel goes at a width, and how much of an
 * endless list is drawn.
 */
const meta: Meta<typeof JobLedger> = {
  title: "Compositions/Job ledger",
  component: JobLedger,
};
export default meta;

type Story = StoryObj<typeof JobLedger>;

/* 12px at strokeWidth 2, `docs/contracts/iconography.md`'s only badge size. */
const MARK = { size: 12, strokeWidth: 2, "aria-hidden": true } as const;

const ROWS: JobLedgerRow[] = [
  {
    id: "r1",
    when: "10:44",
    whenExact: "2026-09-22T10:44:00Z",
    where: "Implement · group 3",
    who: "check",
    whoSays: "Check",
    what: "screens_test",
    outcome: "failed — 1 of 1384 failed: the Drones row opened the Board",
    tone: "failed",
  },
  {
    id: "r2",
    when: "10:14",
    whenExact: "2026-09-22T10:14:00Z",
    where: "Implement · group 2",
    who: "check",
    whoSays: "Check",
    what: "typecheck",
    outcome: "passed",
    tone: "passed",
  },
  {
    id: "r3",
    when: "09:41",
    whenExact: "2026-09-22T09:41:00Z",
    where: "Implement · group 1 · T1",
    who: "drone",
    whoSays: "Drone",
    what: "T1 — Serve one read of everything running",
    outcome: "The read answers Drones, Checks and Judge calls in one call",
    tone: "passed",
  },
  {
    id: "r4",
    when: "09:21",
    whenExact: "2026-09-22T09:21:00Z",
    where: "Plan the change",
    who: "judge",
    whoSays: "Judge",
    what: "The rail's Drones stat reads one running beside the machine's most",
    outcome: "met",
  },
  {
    id: "r5",
    when: "09:21",
    whenExact: "2026-09-22T09:21:00Z",
    where: "Plan the change",
    who: "fleet",
    whoSays: "Fleet",
    what: "the plan was recorded",
    outcome: "8 tasks",
  },
  {
    id: "r6",
    when: "09:14",
    whenExact: "2026-09-22T09:14:00Z",
    where: "The Job itself",
    who: "you",
    whoSays: "You",
    what: "approved the dispatch",
    // No mark: the icon registry assigns the Job's own moves no glyph, and a
    // borrowed one would mean something else. `[record-kind-marks]`.
    outcome: "the workflow and the gates are frozen",
  },
  {
    id: "r7",
    when: "09:38",
    whenExact: "2026-09-22T09:38:00Z",
    where: "Implement · T1",
    who: "drone",
    whoSays: "Drone",
    what: "running-rows.tsx",
    outcome: "modified, which the step never said it would change",
    mark: { glyph: <FileDiff {...MARK} />, says: "file_written" },
  },
  {
    id: "r8",
    when: "09:36",
    whenExact: "2026-09-22T09:36:00Z",
    where: "Plan the change",
    who: "drone",
    whoSays: "Drone",
    what: "The plan reads back as eight tasks in three groups",
    outcome: "plan.md",
    mark: { glyph: <FileCheck {...MARK} />, says: "evidence_submitted" },
  },
];

/**
 * All is the total; the eight are families of it. **Job is one of them** (the
 * owner, 28 September 2026): the Job's own machine moving answered to no filter
 * before it, which is what the `note` had to say out loud on every Record.
 */
const FILTERS = [
  { id: "all", label: "All", count: ROWS.length },
  { id: "job", label: "Job", count: 1 },
  { id: "evidence", label: "Evidence", count: 1 },
  { id: "files", label: "Files", count: 1 },
  { id: "checks", label: "Checks", count: 2 },
  { id: "judges", label: "Judges", count: 1 },
  { id: "drones", label: "Drones", count: 0 },
  { id: "tasks", label: "Tasks", count: 2 },
  { id: "tests", label: "Tests", count: 0 },
];

/** The ledger with nothing open — nine filters, and the table under them. */
export const OneLedger: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
  },
  /**
   * **No row is counted twice, and Who is one word.** The defect the counts
   * were built against is a board whose All read 34 while its filters summed to
   * 35, so the families may never exceed All.
   */
  play: async ({ args, canvas }) => {
    const all = args.filters.find((one) => one.id === "all")!.count;
    const families = args.filters
      .filter((one) => one.id !== "all")
      .reduce((total, one) => total + one.count, 0);

    await expect(families).toBeLessThanOrEqual(all);
    await expect(canvas.getByRole("columnheader", { name: "Who" })).toBeVisible();
    await expect(canvas.queryByRole("columnheader", { name: "Who ran it" })).toBeNull();
  },
};

/**
 * A row no filter names, counted under All and said out loud.
 *
 * **Job closed most of this gap and cannot close all of it**: `kind` is an
 * opaque string, so a kind this Bridge has never heard of still answers to
 * nothing, and the line is what a reader is owed instead of the subtraction.
 */
export const ARowNoFilterNames: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS.map((one) => (one.id === "job" ? { ...one, count: 0 } : one)),
    filter: "all",
    onFilter: () => undefined,
    note: "One more row is under All alone: a kind no filter names.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("note")).toHaveTextContent(/under All alone/);
  },
};

/** A row open, read in the sheet over the table. */
export const ARowOpen: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    openRow: "r1",
    inspector: <p className="armada-ledger__note">What this Check printed goes here.</p>,
  },
};

/**
 * All, with each row led by its kind's mark.
 *
 * **Two families have a glyph and the rest have none.** `file-check` is a
 * submission that landed and `file-diff` is reading what one file changed, both
 * by their own reservations; nothing in `packages/icons/icons.toml` means *a
 * Check ran* or *a Drone arrived*, and a borrowed glyph would say something
 * else. `[record-kind-marks]` is where that gap is filed.
 */
export const KindMarkedOnAll: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    kindMarks: true,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("columnheader", { name: "Kind" })).toBeInTheDocument();
    await expect(canvas.getByText("file_written")).toBeInTheDocument();
    await expect(canvas.getByText("evidence_submitted")).toBeInTheDocument();
  },
};

/**
 * One family chosen, and the mark column gone with it.
 *
 * **A filter has already said the kind.** Repeating it down the rows of Files
 * is one glyph over and over, which trains the eye to stop reading the channel.
 */
export const OneFamilyNoMarks: Story = {
  args: {
    rows: ROWS.filter((row) => row.mark?.says === "file_written"),
    filters: FILTERS,
    filter: "files",
    onFilter: () => undefined,
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("columnheader", { name: "Kind" })).toBeNull();
    await expect(canvas.queryByText("file_written")).toBeNull();
  },
};

/** A filter that holds nothing. Never a bare menu over an empty frame. */
export const NothingUnderThisFilter: Story = {
  args: {
    rows: [],
    filters: FILTERS,
    filter: "evidence",
    onFilter: () => undefined,
    emptyNote: "No Drone has submitted evidence on this Job.",
  },
};

/** More rows than the bound draws. What was left out is counted, never dropped. */
export const BoundedAndSaidSo: Story = {
  args: {
    rows: [...ROWS, ...ROWS.map((row) => ({ ...row, id: `${row.id}-b` }))],
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    bound: 4,
  },
  /**
   * **A bounded list says by how much.** A list that simply stopped would be
   * indistinguishable from a Job that did nothing else, which is the quiet
   * truncation the v1 failure log is full of.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/12 older rows are not drawn/)).toBeVisible();
  },
};

/** The press reaches the surface from the row and from the keyboard alike. */
export const OpeningARow: Story = {
  render: function Opening({ onOpenRow, ...args }) {
    const [open, setOpen] = useState<string | null>(null);
    return (
      <JobLedger
        {...args}
        openRow={open}
        onOpenRow={(id) => {
          setOpen(id);
          onOpenRow?.(id);
        }}
        inspector={open === null ? undefined : <p className="armada-ledger__note">Row {open}</p>}
      />
    );
  },
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: () => undefined,
    onOpenRow: fn(),
  },
  /**
   * **One press is one report.** The button inside the cell sits inside the
   * row that handles the same act, so its click bubbles into it: on screen the
   * two are indistinguishable, because both name the same row — and a surface
   * counting presses hears the act twice. The spy is the only thing that can
   * see it, which is what earns one here.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /screens_test/ }));

    await expect(canvas.getByText("Row r1")).toBeVisible();
    await expect(args.onOpenRow).toHaveBeenCalledTimes(1);
  },
};
