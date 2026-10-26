import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, waitFor, within } from "storybook/test";

import { RunControl } from "./RunControl";
import type { SavedRun } from "./runs";

/**
 * The run control: a play glyph that runs the last run used, and a caret that lists the repository's saved runs and ends on Add a way to run.
 * The caller answers a run, and no run at all, which is where the repository has none saved.
 */
const meta: Meta<typeof RunControl> = {
  title: "Compositions/Run control",
  component: RunControl,
};
export default meta;

type Story = StoryObj<typeof RunControl>;

const RUNS: readonly SavedRun[] = [
  { id: "mock-bridge", name: "Mock Bridge", kind: "browser", processes: [{ id: "mock", name: "mock", folder: "apps/desktop", command: "npx vite --port ${port}" }] },
  { id: "storybook", name: "Storybook", kind: "browser", processes: [{ id: "storybook", name: "storybook", folder: "packages/components", command: "pnpm exec storybook dev -p ${port}" }] },
  { id: "bridge-dev", name: "Bridge dev", kind: "app", processes: [{ id: "bridge", name: "bridge", folder: "apps/desktop", command: "pnpm dev" }] },
];

/** The picker is a menu on the page, not inside the story's frame. */
const menu = () => within(screen.getByRole("menu"));

/**
 * **Play runs what was run last, and the caret is the way to another.** The face repeats `last`, so a press of play hands that run back
 * and opens nothing. The caret opens the picker: the run in use carries the check in the leading column and no other does, each run says
 * whether it opens in a browser or as an app, and the last item is Add a way to run. Esc closes the picker and hands focus back to the caret.
 */
export const Picker: Story = {
  args: { subject: "armada", on: "main", runs: RUNS, last: "storybook", onRun: fn(), onAdd: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run armada" }));
    await expect(args.onRun).toHaveBeenLastCalledWith(RUNS[1]);
    await expect(screen.queryByRole("menu")).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Runs for armada" }));
    await waitFor(() => expect(screen.getByRole("menu")).toBeVisible());
    const items = menu().getAllByRole("menuitem");
    await expect(items.map((one) => one.textContent)).toEqual([expect.stringContaining("Mock Bridge"), expect.stringContaining("Storybook"), expect.stringContaining("Bridge dev"), expect.stringContaining("Add a way to run")]);
    await expect(menu().getByRole("menuitem", { name: /Storybook/ })).toHaveAttribute("aria-current", "true");
    await expect(menu().getByRole("menuitem", { name: /Mock Bridge/ })).not.toHaveAttribute("aria-current");
    await expect(menu().getByRole("menuitem", { name: /Bridge dev/ })).not.toHaveAttribute("aria-current");
    await expect(menu().getByRole("menuitem", { name: /Mock Bridge/ })).toHaveTextContent("browser");
    await expect(menu().getByRole("menuitem", { name: /Bridge dev/ })).toHaveTextContent("app");
    await expect(items.at(-1)).toHaveTextContent("Add a way to run");

    await userEvent.keyboard("{Escape}");
    await expect(screen.queryByRole("menu")).toBeNull();
    await expect(canvas.getByRole("button", { name: "Runs for armada" })).toHaveFocus();
    await expect(args.onRun).toHaveBeenCalledTimes(1);
  },
};

/** **The arrow keys and Enter pick.** The caret opens the picker on the arrow key, the arrows step to a run, and Enter runs it; the last item answers `onAdd`, not a run. */
export const PickedByKeyboard: Story = {
  args: { subject: "armada", on: "main", runs: RUNS, last: "mock-bridge", onRun: fn(), onAdd: fn() },
  play: async ({ args, canvas, userEvent }) => {
    canvas.getByRole("button", { name: "Runs for armada" }).focus();
    await userEvent.keyboard("{ArrowDown}");
    await waitFor(() => expect(screen.getByRole("menu")).toBeVisible());
    await userEvent.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    await expect(args.onRun).toHaveBeenLastCalledWith(RUNS[2]);
    await expect(screen.queryByRole("menu")).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Runs for armada" }));
    await waitFor(() => expect(screen.getByRole("menu")).toBeVisible());
    await userEvent.keyboard("{ArrowUp}{Enter}");
    await expect(args.onAdd).toHaveBeenCalledTimes(1);
    await expect(args.onRun).toHaveBeenCalledTimes(1);
  },
};

/** **A repository with nothing saved.** Play hands back no run, so the caller can ask first, and the picker holds only Add a way to run. */
export const NothingSaved: Story = {
  args: { subject: "auto-ynab", on: "main", runs: [], onRun: fn(), onAdd: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run auto-ynab" }));
    await expect(args.onRun).toHaveBeenCalledWith(undefined);

    await userEvent.click(canvas.getByRole("button", { name: "Runs for auto-ynab" }));
    await waitFor(() => expect(screen.getByRole("menu")).toBeVisible());
    await expect(menu().getAllByRole("menuitem")).toHaveLength(1);
    await userEvent.click(menu().getByRole("menuitem", { name: "Add a way to run" }));
    await expect(args.onAdd).toHaveBeenCalledTimes(1);
  },
};
