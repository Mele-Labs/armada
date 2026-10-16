import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { RunningChecks, ranFor, type RunningCheck } from "./RunningChecks";

const CHECKS: readonly RunningCheck[] = [
  { run: 32, name: "test", target: "session_checking", at: "14:20:00" },
  { run: 33, name: "app_smoke", at: "14:23:00" },
];

/** The Checks a Session has out, in its header: a mark with the count, and a press lists them. */
const meta: Meta<typeof RunningChecks> = {
  title: "Compositions/Running checks",
  component: RunningChecks,
  args: { checks: CHECKS, onOpenCheck: fn() },
};
export default meta;

type Story = StoryObj<typeof RunningChecks>;

/** Nothing is drawn at none. */
export const None: Story = {
  args: { checks: [] },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).not.toBeInTheDocument();
  },
};

/** The mark is named by its count, one and many. */
export const Counted: Story = {
  args: { checks: CHECKS.slice(0, 1) },
  render: (args) => (
    <>
      <RunningChecks {...args} />
      <RunningChecks checks={CHECKS} />
    </>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "1 check running" })).toHaveTextContent("1");
    await expect(canvas.getByRole("button", { name: "2 checks running" })).toHaveTextContent("2");
  },
};

/** A press lists each check by its key, target and state, and a press on one opens its run and closes the list. */
export const ListedAndOpened: Story = {
  play: async ({ canvas, userEvent, args }) => {
    const mark = canvas.getByRole("button", { name: "2 checks running" });
    await expect(mark).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByRole("menu")).not.toBeInTheDocument();

    await userEvent.click(mark);
    await expect(mark).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByRole("menu", { name: "2 checks running" })).toBeVisible();
    await expect(canvas.getAllByRole("menuitem")).toHaveLength(2);
    await expect(canvas.getByRole("menuitem", { name: "Check test session_checking, running" })).toBeVisible();
    await expect(canvas.getByRole("menuitem", { name: "Check app_smoke, running" })).toBeVisible();

    await userEvent.click(canvas.getByRole("menuitem", { name: "Check app_smoke, running" }));
    await expect(args.onOpenCheck).toHaveBeenCalledExactlyOnceWith(33);
    await expect(canvas.queryByRole("menu")).not.toBeInTheDocument();
  },
};

/** The first item takes focus; ↑ and ↓ walk the list and wrap at both ends. */
export const Walked: Story = {
  play: async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "2 checks running" }));
    const first = canvas.getByRole("menuitem", { name: "Check test session_checking, running" });
    const second = canvas.getByRole("menuitem", { name: "Check app_smoke, running" });
    await expect(first).toHaveFocus();

    await userEvent.keyboard("{ArrowDown}");
    await expect(second).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    await expect(first).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(second).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(first).toHaveFocus();

    await userEvent.keyboard("{ArrowDown}{Enter}");
    await expect(args.onOpenCheck).toHaveBeenCalledExactlyOnceWith(33);
  },
};

/** Esc closes the list and puts focus back on the mark. */
export const EscapeCloses: Story = {
  play: async ({ canvas, userEvent, args }) => {
    const mark = canvas.getByRole("button", { name: "2 checks running" });
    await userEvent.click(mark);
    await expect(canvas.getByRole("menu")).toBeVisible();

    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("menu")).not.toBeInTheDocument();
    await expect(mark).toHaveFocus();
    await expect(args.onOpenCheck).not.toHaveBeenCalled();
  },
};

/** The last check ending takes the mark and its open list with it. Focus moves to Finish by Tab, so only the check ending can close the list. */
function Ending() {
  const [checks, setChecks] = useState<readonly RunningCheck[]>(CHECKS.slice(0, 1));
  return (
    <>
      <RunningChecks checks={checks} />
      <button type="button" onClick={() => setChecks([])}>
        Finish
      </button>
    </>
  );
}

export const EndsWithTheLast: Story = {
  render: () => <Ending />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "1 check running" }));
    await expect(canvas.getByRole("menu")).toBeVisible();

    await userEvent.tab();
    await expect(canvas.getByRole("button", { name: "Finish" })).toHaveFocus();
    await expect(canvas.getByRole("menu")).toBeVisible();
    await userEvent.keyboard("{Enter}");
    await expect(canvas.queryByRole("menu")).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: /check.* running/ })).not.toBeInTheDocument();
  },
};

/** How long a check has run is read from the clock time it started. */
export const RanFor: Story = {
  render: () => <span />,
  play: async () => {
    const at = (h: number, m: number, s: number) => new Date(2026, 9, 16, h, m, s);
    await expect(ranFor("14:23:10", at(14, 23, 52))).toBe("42s");
    await expect(ranFor("14:20:40", at(14, 23, 52))).toBe("3m 12s");
    await expect(ranFor("13:18:00", at(14, 23, 52))).toBe("1h 5m");
    // "14:23" has no seconds.
    await expect(ranFor("14:23", at(14, 24, 5))).toBe("1m 5s");
    // A start after now is yesterday's.
    await expect(ranFor("23:59:00", at(0, 1, 0))).toBe("2m 0s");
    await expect(ranFor("later", at(14, 23, 52))).toBeUndefined();
  },
};
