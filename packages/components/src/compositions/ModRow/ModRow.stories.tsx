import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { Card, CardContent } from "../../primitives/Card/Card";
import { ModRow } from "./ModRow";

const meta: Meta<typeof ModRow> = {
  title: "Compositions/Mod row",
  component: ModRow,
  args: { title: "Dusk", enabled: true, onEnabled: fn(), onPromote: fn() },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Card>
          <CardContent>
            <Story />
          </CardContent>
        </Card>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ModRow>;

/** On and valid, which is where a new mod starts. */
export const OnAndValid: Story = {
  play: async ({ canvasElement }) => {
    const row = within(canvasElement).getByRole("group", { name: "Dusk" });
    await expect(within(row).getByRole("img", { name: "Theme" })).toBeVisible();
    await expect(within(row).getByRole("switch", { name: "Dusk" })).toBeChecked();
  },
};

export const SwitchedOff: Story = { args: { enabled: false } };

/** Fleet's reason, read exactly. Promote is off because a mod that fails its checks has nothing to promote. */
export const Invalid: Story = {
  args: { detail: "line 3: `--color-x` is not a token", promoteDisabled: true },
};

/** The branch it was put on, and nothing else. Promote is spent. */
export const OnABranch: Story = {
  args: { detail: "armada/mod-dusk-1791500000000", promoteDisabled: true },
};

/** The two presses reach their callbacks: the switch with the new value, Promote once. */
export const Presses: Story = {
  play: async ({ canvasElement, args }) => {
    const row = within(canvasElement).getByRole("group", { name: "Dusk" });
    await userEvent.click(within(row).getByRole("switch", { name: "Dusk" }));
    await expect(args.onEnabled).toHaveBeenCalledWith(false);
    await userEvent.click(within(row).getByRole("button", { name: "Promote Dusk" }));
    await expect(args.onPromote).toHaveBeenCalledTimes(1);
  },
};

const rowOf = (canvasElement: HTMLElement, name: string) => within(canvasElement).getByRole("group", { name });

/** A tracker mod used by a Manifest: tried, valid and on. */
export const TrackerUsed: Story = {
  args: {
    title: "linear",
    kind: "tracker",
    standing: "used",
    tracker: { tried: { state: "passed", words: "Tried 14 Oct on armada" }, valid: true },
    onTry: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const row = rowOf(canvasElement, "linear");
    await expect(row).toHaveAttribute("data-standing", "used");
    await expect(within(row).getByRole("img", { name: "Tracker" })).toBeVisible();
    await expect(within(row).getByRole("img", { name: "Tried 14 Oct on armada" })).toHaveAttribute("data-tone", "passed");
    await expect(within(row).getByRole("img", { name: "Valid" })).toHaveAttribute("data-tone", "valid");
    await userEvent.click(within(row).getByRole("button", { name: "Try linear" }));
    await expect(args.onTry).toHaveBeenCalledTimes(1);
  },
};

/** Edited since its try, so a Manifest does not use it until it is tried again. */
export const TrackerStale: Story = {
  args: { title: "linear", kind: "tracker", standing: "waiting", tracker: { tried: { state: "stale", words: "Stale" }, valid: true }, onTry: fn() },
  play: async ({ canvasElement, args }) => {
    const row = rowOf(canvasElement, "linear");
    await expect(row).toHaveAttribute("data-standing", "waiting");
    await expect(within(row).getByRole("img", { name: "Stale" })).toHaveAttribute("data-tone", "stale");
    await userEvent.click(within(row).getByRole("button", { name: "Try linear" }));
    await expect(args.onTry).toHaveBeenCalledTimes(1);
  },
};

/** Never tried: the same waiting frame as a stale one, with its own words. */
export const TrackerNeverTried: Story = {
  args: { title: "linear", kind: "tracker", standing: "waiting", tracker: { tried: { state: "never", words: "Not tried" }, valid: true }, onTry: fn() },
  play: async ({ canvasElement }) => {
    const row = rowOf(canvasElement, "linear");
    await expect(row).toHaveAttribute("data-standing", "waiting");
    await expect(within(row).getByRole("img", { name: "Not tried" })).toHaveAttribute("data-tone", "never");
    await expect(within(row).queryByRole("img", { name: "Stale" })).toBeNull();
  },
};

/** A try that failed: Fleet's reason on the mark, the refused frame, and Try still on, since the mod itself is valid. */
export const TrackerFailed: Story = {
  args: {
    title: "linear",
    kind: "tracker",
    standing: "refused",
    tracker: { tried: { state: "failed", words: "Failed 14 Oct: `tracker-cli` is not installed" }, valid: true },
    onTry: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const row = rowOf(canvasElement, "linear");
    await expect(row).toHaveAttribute("data-standing", "refused");
    await expect(within(row).getByRole("img", { name: "Failed 14 Oct: `tracker-cli` is not installed" })).toHaveAttribute("data-tone", "failed");
    await userEvent.click(within(row).getByRole("button", { name: "Try linear" }));
    await expect(args.onTry).toHaveBeenCalledTimes(1);
  },
};

/** Refused: Fleet's reason is the line, and Try is off. */
export const TrackerRefused: Story = {
  args: {
    title: "shortcut",
    kind: "tracker",
    standing: "refused",
    detail: "[search] has no argv",
    promoteDisabled: true,
    tracker: { tried: { state: "never", words: "Not tried" }, valid: false },
    onTry: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const row = rowOf(canvasElement, "shortcut");
    await expect(row).toHaveAttribute("data-standing", "refused");
    await expect(within(row).getByRole("img", { name: "Invalid" })).toHaveAttribute("data-tone", "invalid");
    await expect(within(row).getByText("[search] has no argv")).toBeVisible();
    const tryButton = within(row).getByRole("button", { name: "Try shortcut" });
    await expect(tryButton).toBeDisabled();
    await userEvent.click(tryButton, { pointerEventsCheck: 0 });
    await expect(args.onTry).not.toHaveBeenCalled();
  },
};

/** Switched off: the grey frame, with its marks and Try as they were. */
export const TrackerOff: Story = {
  args: { title: "linear", kind: "tracker", standing: "off", enabled: false, tracker: { tried: { state: "passed", words: "Tried 14 Oct on armada" }, valid: true }, onTry: fn() },
  play: async ({ canvasElement }) => {
    const row = rowOf(canvasElement, "linear");
    await expect(row).toHaveAttribute("data-standing", "off");
    await expect(within(row).getByRole("switch", { name: "linear" })).not.toBeChecked();
  },
};

/** A list holding a tracker keeps the columns in a row that is not one: no marks, no Try. */
export const WithTrackerColumnsOnly: Story = {
  args: { withTracker: true, standing: "used" },
  play: async ({ canvasElement }) => {
    const row = rowOf(canvasElement, "Dusk");
    await expect(row).toHaveAttribute("data-tracking", "true");
    await expect(within(row).queryByRole("button", { name: "Try Dusk" })).toBeNull();
    await expect(within(row).queryByRole("img", { name: "Valid" })).toBeNull();
  },
};
