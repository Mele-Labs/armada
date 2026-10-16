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

function holding(open: Clashing["open"], other: ClashHolderRef, job = "j1"): Clashing {
  return { open, of: (kind, id) => (kind === "job" && id === job ? [{ with: other, paths: PATHS }] : []) };
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

/** In an open Job or Session the clashes are one bar: folded, the other holders on a line; opened, a row for each with its files. */
export const InAnOpenJob: Story = {
  render: () => (
    <ClashesProvider value={holding(opened, SESSION, "j-open")}>
      <ClashNotice kind="job" id="j-open" />
    </ClashesProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    opened.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Write clash" }));
    await expect(canvas.getByRole("list", { name: /^Conflicting files/ })).toHaveTextContent("retry.rs");
    await userEvent.click(canvas.getByRole("button", { name: "Open Retry tuning" }));
    await expect(opened).toHaveBeenCalledTimes(1);
    await expect(opened).toHaveBeenCalledWith(SESSION);
  },
};

// Each notice has a Job of its own: a bar he opened stays open for its owner, so stories that shared one would start as the last left it.
const MANY = Array.from({ length: 37 }, (_, i) => `crates/store/src/migrations/${String(i + 1).padStart(4, "0")}_clock.rs`);

/** A long list shows three files and one chip for the rest; the chip opens them all and then folds them back. */
export const WithManyFiles: Story = {
  render: () => (
    <ClashesProvider value={{ open: opened, of: (kind, id) => (kind === "job" && id === "j-many" ? [{ with: MAIN, paths: MANY }] : []) }}>
      <ClashNotice kind="job" id="j-many" />
    </ClashesProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    const header = canvas.getByRole("button", { name: "Write clash" });
    await expect(header).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(header);
    const list = canvas.getByRole("list", { name: /^Conflicting files/ });
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
    await expect(list).toHaveTextContent("0003_clock.rs");
    await expect(list).not.toHaveTextContent("0004_clock.rs");
    await expect(canvas.getByText("main")).toBeVisible();
    const more = canvas.getByRole("button", { name: "34 more" });
    await expect(more).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(more);
    await expect(list).toHaveTextContent("0037_clock.rs");
    await expect(canvas.queryByText("34 more")).toBeNull();
    const fewer = canvas.getByRole("button", { name: "Fewer" });
    await expect(fewer).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(fewer);
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
    // A bar he opened stays open for that Job, so the story folds it back.
    await userEvent.click(header);
    await expect(header).toHaveAttribute("aria-expanded", "false");
  },
};

/** The notice starts folded to its line. The line opens the list and folds it again; the holder's own button opens the holder and leaves the panel as it is. */
export const Collapsed: Story = {
  render: () => (
    <ClashesProvider value={holding(opened, SESSION, "j-collapsed")}>
      <ClashNotice kind="job" id="j-collapsed" />
    </ClashesProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    opened.mockClear();
    const header = canvas.getByRole("button", { name: "Write clash" });
    await expect(header).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(header);
    await expect(header).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(canvas.getByRole("button", { name: "Open Retry tuning" }));
    await expect(opened).toHaveBeenCalledTimes(1);
    await expect(header).toHaveAttribute("aria-expanded", "true");
    await userEvent.click(header);
    await expect(header).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.getByRole("list", { name: /^Conflicting files/, hidden: true }).closest("[inert]")).not.toBeNull();
    await userEvent.click(header);
    await expect(canvas.getByRole("list", { name: /^Conflicting files/ })).toHaveTextContent("retry.rs");
    await userEvent.click(header);
  },
};

/** Two clashes are one bar, folded to a line of two chips; opened, a row for each. */
export const WithTwoClashes: Story = {
  render: () => (
    <ClashesProvider value={{ open: opened, of: (kind, id) => (kind === "job" && id === "j-two" ? [{ with: SESSION, paths: PATHS }, { with: MAIN, paths: ["crates/store/src/open.rs"] }] : []) }}>
      <ClashNotice kind="job" id="j-two" />
    </ClashesProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getAllByRole("region", { name: "Write clash" })).toHaveLength(1);
    const header = canvas.getByRole("button", { name: "Write clash" });
    await expect(header).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(header);
    await expect(canvas.getByRole("list", { name: "Conflicting files with Retry tuning" })).toHaveTextContent("retry.rs");
    await expect(canvas.getByRole("list", { name: "Conflicting files with main" })).toHaveTextContent("open.rs");
    await userEvent.click(header);
  },
};

/** Four files are drawn whole, with no chip. */
export const WithFourFiles: Story = {
  render: () => (
    <ClashesProvider value={{ open: opened, of: (kind, id) => (kind === "job" && id === "j-four" ? [{ with: MAIN, paths: MANY.slice(0, 4) }] : []) }}>
      <ClashNotice kind="job" id="j-four" />
    </ClashesProvider>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
    await expect(canvas.getByRole("list", { name: /^Conflicting files/ })).toHaveTextContent("0004_clock.rs");
    await expect(canvas.queryByRole("button", { name: /more$|^Fewer$/ })).toBeNull();
  },
};
