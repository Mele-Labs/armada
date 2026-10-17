import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { ClashMark, ClashPaths, ClashesProvider, type ClashHolderRef, type Clashing } from "./ClashMark";

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

function holding(open: Clashing["open"], other: ClashHolderRef, holder = "j1"): Clashing {
  return { open, of: (kind, id) => (kind === "session" && id === holder ? [{ with: other, paths: PATHS }] : []) };
}

const opened = fn();

/** The mark is named for the other holder and the paths, and a press opens that holder. */
export const AgainstAnotherAgent: Story = {
  args: { kind: "session", id: "j1" },
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
  args: { kind: "session", id: "j1" },
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
  args: { kind: "session", id: "j2" },
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

const MANY = Array.from({ length: 37 }, (_, i) => `crates/store/src/migrations/${String(i + 1).padStart(4, "0")}_clock.rs`);

/** The paths of a clash, as a Session's Bays panel lists them. A long list shows three files and one chip for the rest; the chip opens them all and then folds them back. */
export const WithManyFiles: Story = {
  render: () => <ClashPaths clash={{ with: MAIN, paths: MANY }} />,
  play: async ({ canvas, userEvent }) => {
    const list = canvas.getByRole("list", { name: "Conflicting files with main" });
    await expect(canvas.getAllByRole("listitem")).toHaveLength(4);
    await expect(list).toHaveTextContent("0003_clock.rs");
    await expect(list).not.toHaveTextContent("0004_clock.rs");
    const more = canvas.getByRole("button", { name: "34 more" });
    await expect(more).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(more);
    await expect(list).toHaveTextContent("0037_clock.rs");
    await expect(canvas.queryByText("34 more")).toBeNull();
    const fewer = canvas.getByRole("button", { name: "Fewer" });
    await expect(fewer).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(fewer);
    await expect(canvas.getAllByRole("listitem")).toHaveLength(4);
  },
};

/** Four files are drawn whole, with no chip. */
export const WithFourFiles: Story = {
  render: () => <ClashPaths clash={{ with: MAIN, paths: MANY.slice(0, 4) }} />,
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("listitem")).toHaveLength(4);
    await expect(canvas.getByRole("list", { name: /^Conflicting files/ })).toHaveTextContent("0004_clock.rs");
    await expect(canvas.queryByRole("button", { name: /more$|^Fewer$/ })).toBeNull();
  },
};

const pressed = fn();

/** Where the host can read a clash's diff, each path is a press that opens it. */
export const PathsThatOpen: Story = {
  render: () => <ClashPaths clash={{ with: SESSION, paths: PATHS }} onPath={pressed} />,
  play: async ({ canvas, userEvent }) => {
    pressed.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Open crates/fleet/src/backoff.rs" }));
    await expect(pressed).toHaveBeenCalledWith("crates/fleet/src/backoff.rs");
  },
};
