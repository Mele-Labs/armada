import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import { JobDrones, type JobDronesRow } from "./JobDrones";

/**
 * Every Drone a Job has used, with a filter menu that narrows them and one
 * Drone read whole in a sheet over the list.
 *
 * The rows, their words and the transcript are the caller's. This decides the
 * panel, the table and the sheet the open Drone sits in.
 */
const meta: Meta<typeof JobDrones> = {
  title: "Compositions/Job drones",
  component: JobDrones,
};
export default meta;

type Story = StoryObj<typeof JobDrones>;

const ROWS: JobDronesRow[] = [
  {
    id: "d5",
    drone: "Drone on T5",
    where: "Implement · T5",
    state: "running",
    stateSays: "Running",
    spent: "14 turns",
    ranFor: "18m 20s",
    sinceExact: "2026-09-22T10:15:00Z",
  },
  {
    id: "d1",
    drone: "Drone on T1",
    where: "Implement · T1",
    state: "done",
    stateSays: "Done",
    spent: "34 turns · $2.40",
    ranFor: "29m 00s",
    sinceExact: "2026-09-22T09:12:00Z",
  },
  {
    id: "d2a",
    drone: "Drone on T2",
    where: "Implement · T2",
    state: "killed",
    stateSays: "Killed",
    spent: "7 turns",
    ranFor: "8m 00s",
    sinceExact: "2026-09-22T09:12:00Z",
  },
  {
    id: "d6",
    drone: "Drone on T6",
    where: "Implement · T6",
    state: "failed",
    stateSays: "Failed",
    spent: "15 turns · $0.72",
    ranFor: "16m 00s",
    sinceExact: "2026-09-22T10:15:00Z",
  },
];

const FILTERS = [
  { id: "all", label: "All", count: 4 },
  { id: "running", label: "Running", count: 1 },
  { id: "done", label: "Done", count: 1 },
  { id: "failed", label: "Failed", count: 1 },
  { id: "killed", label: "Killed", count: 1 },
];

/**
 * The four states a Drone can be in, one row each. **The filter's trigger
 * carries no number**: the rows it would count are drawn under it (the owner,
 * 29 Sep 2026). The menu's entries keep theirs.
 */
export const EveryState: Story = {
  args: { rows: ROWS, filters: FILTERS, filter: "all", onFilter: fn(), onOpenRow: fn() },
  play: async ({ args, canvas, userEvent }) => {
    const panel = canvas.getByRole("region", { name: "Drones on this Job" });
    await expect(panel.querySelector("button")).toHaveTextContent(/^All$/);
    await userEvent.click(canvas.getByRole("button", { name: "Drone on T1" }));
    await expect(args.onOpenRow).toHaveBeenCalledWith("d1");
  },
};

/** A filter nothing answers to draws nothing under its menu: no table, no sentence. */
export const NothingUnderTheFilter: Story = {
  args: { rows: [], filters: FILTERS, filter: "killed", onFilter: fn() },
  play: async ({ canvas }) => {
    const panel = canvas.getByRole("region", { name: "Drones on this Job" });
    await expect(within(panel).queryByRole("table")).toBeNull();
    await expect(within(panel).queryByRole("note")).toBeNull();
  },
};

/** A running Drone open in the sheet, following its transcript's tail. */
export const OneOpen: Story = {
  args: {
    rows: ROWS,
    filters: FILTERS,
    filter: "all",
    onFilter: fn(),
    onOpenRow: fn(),
    openRow: "d5",
    reading: {
      title: "Drone on T5",
      subtitle: "Implement · T5 · running · 14 turns · 18m 20s",
      live: true,
      emptyNote: "This Drone has written nothing yet.",
      turns: [
        { id: "1", at: "10:15:20", kind: "said", who: "drone", said: "Starting on T5. Reading the files it touches first." },
        { id: "2", at: "10:15:40", kind: "called", who: "drone", subject: "Read", detail: "packages/screens/src/Running.tsx" },
        { id: "3", at: "10:16:24", kind: "called", who: "drone", subject: "Edit", detail: "packages/screens/src/Running.tsx" },
      ],
    },
  },
};
