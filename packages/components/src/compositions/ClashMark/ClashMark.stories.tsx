import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { ClashMark, ClashNotice, ClashesProvider, type ClashHolderRef, type Clashing } from "./ClashMark";

/**
 * Two agents whose changes in one repository would conflict, drawn on both rows of the pair: one `swords`
 * mark whose tooltip names the other holder and the paths. A press opens that holder. Fleet decides what
 * clashes; a pair that touches a file without a conflict is not in the list, so nothing is drawn.
 */
const meta: Meta<typeof ClashMark> = {
  title: "Compositions/Clash mark",
  component: ClashMark,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ClashMark>;

const SESSION: ClashHolderRef = { kind: "session", id: "s1", label: "Retry tuning" };
const MAIN: ClashHolderRef = { kind: "main", id: "main", label: "main" };
const PATHS = ["crates/fleet/src/retry.rs", "crates/fleet/src/backoff.rs"];

function holding(open: Clashing["open"], other: ClashHolderRef): Clashing {
  return { open, of: (kind, id) => (kind === "job" && id === "j1" ? [{ with: other, paths: PATHS }] : []) };
}

const opened = fn();

/** The mark is named for the other holder and the paths, and a press opens that holder. */
export const AgainstAnotherAgent: Story = {
  args: { kind: "job", id: "j1" },
  decorators: [
    (Story) => (
      <ClashesProvider value={holding(opened, SESSION)}>
        <Story />
      </ClashesProvider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    opened.mockClear();
    const mark = canvas.getByRole("button", { name: "Conflicts with Retry tuning · crates/fleet/src/retry.rs, crates/fleet/src/backoff.rs" });
    await userEvent.click(mark);
    await expect(opened).toHaveBeenCalledTimes(1);
    await expect(opened).toHaveBeenCalledWith(SESSION);
  },
};

/** Main is not something to open, so the mark is a picture and nothing answers a press. */
export const AgainstMain: Story = {
  args: { kind: "job", id: "j1" },
  decorators: [
    (Story) => (
      <ClashesProvider value={holding(opened, MAIN)}>
        <Story />
      </ClashesProvider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    opened.mockClear();
    const mark = canvas.getByRole("img", { name: "Conflicts with main · crates/fleet/src/retry.rs, crates/fleet/src/backoff.rs" });
    await expect(canvas.queryByRole("button")).toBeNull();
    await userEvent.click(mark);
    await expect(opened).not.toHaveBeenCalled();
  },
};

/** A holder in no clash draws nothing, and so does a window that was given none. */
export const InNoClash: Story = {
  args: { kind: "job", id: "j2" },
  decorators: [
    (Story) => (
      <ClashesProvider value={holding(opened, SESSION)}>
        <Story />
      </ClashesProvider>
    ),
  ],
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};

/** In an open Job or Session the same clash is a line: the other holder, linked, and the files. */
export const InAnOpenJob: Story = {
  render: () => (
    <ClashesProvider value={holding(opened, SESSION)}>
      <ClashNotice kind="job" id="j1" />
    </ClashesProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    opened.mockClear();
    await expect(canvas.getByRole("list", { name: "Conflicting files" })).toHaveTextContent("retry.rs");
    await userEvent.click(canvas.getByRole("button", { name: "Open Retry tuning" }));
    await expect(opened).toHaveBeenCalledTimes(1);
    await expect(opened).toHaveBeenCalledWith(SESSION);
  },
};
