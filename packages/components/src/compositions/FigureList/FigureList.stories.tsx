import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { FigureList } from "./FigureList";

const meta: Meta<typeof FigureList> = {
  title: "Compositions/FigureList",
  component: FigureList,
};
export default meta;

type Story = StoryObj<typeof FigureList>;

/**
 * **Every value ends on the list's right edge, and none of them touches its
 * label.** Alignment is geometry, so this measures rather than reading roles —
 * a value drawn hard left reads identically through `getByRole`. The text is
 * measured with a range, because the `dd` box runs to the edge whichever way
 * its text is aligned.
 */
async function endsOnTheRightEdge(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  const labels = canvas.getAllByRole("term");
  const values = canvas.getAllByRole("definition");
  const list = values[0]?.closest("dl");
  await expect(list).toBeTruthy();
  const edge = list!.getBoundingClientRect().right;
  for (const [index, value] of values.entries()) {
    const text = document.createRange();
    text.selectNodeContents(value);
    const drawn = text.getBoundingClientRect();
    await expect(Math.abs(drawn.right - edge)).toBeLessThanOrEqual(1);
    await expect(drawn.left).toBeGreaterThan(labels[index]!.getBoundingClientRect().right);
  }
}

/** Pulse's figures, on the column *Where things are* draws. */
export const Wide: Story = {
  args: {
    figures: [
      { label: "Processes", value: "None", wrong: true },
      { label: "Worktree", value: "gone", wrong: true },
      { label: "Size on disk", value: "1.2 GiB" },
    ],
  },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-run-column)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await endsOnTheRightEdge(canvasElement);
  },
};

/** The Fleet panel's rows, the column as wide as `protocol` and no wider. */
export const Fit: Story = {
  args: {
    column: "fit",
    figures: [
      { label: "pid", value: "4242" },
      { label: "port", value: "7878" },
      { label: "protocol", value: "14.5" },
      { label: "up", value: "171h 00m" },
    ],
  },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--sidebar-min)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await endsOnTheRightEdge(canvasElement);
  },
};

/**
 * Pulse's band: the label over its figure, the figures packed from the leading
 * edge, and one rule where what the Job is running stops and what it is taking
 * begins.
 *
 * **The figure is the only thing here a person lands on**, so it is the only
 * thing drawn large. `play` measures that rather than reading it: a value set
 * at the label's size reads identically through `getByRole`.
 */
export const Strip: Story = {
  args: {
    column: "strip",
    figures: [
      { label: "Drones running", value: "3" },
      { label: "Checks running", value: "1" },
      { label: "Judges running", value: "0" },
      { label: "Spend", value: "~$9.20", detail: "of $30.00", apart: true },
      { label: "Turns", value: "1204", detail: "of 3000" },
      { label: "Processes", value: "5" },
    ],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    for (const [index, value] of canvas.getAllByRole("definition").entries()) {
      const label = canvas.getAllByRole("term")[index]!;
      const figure = Number.parseFloat(getComputedStyle(value.firstElementChild!).fontSize);
      const said = Number.parseFloat(getComputedStyle(label).fontSize);
      await expect(figure).toBeGreaterThan(said);
      // Under it, never beside it: the label's box ends before the figure's starts.
      await expect(value.getBoundingClientRect().top).toBeGreaterThanOrEqual(
        label.getBoundingClientRect().bottom,
      );
    }
  },
};

/**
 * **The longest figure either caller draws, at the narrowest width there is.**
 * `up` runs to `171h 55m` and a worktree to `1.2 GiB`, and the left column's
 * Fleet panel is 160px — so this is where a right-aligned value would meet its
 * label if the label column were not a track of its own.
 */
export const FitAtColumnMinimum: Story = {
  args: {
    column: "fit",
    figures: [
      { label: "protocol", value: "14.6" },
      { label: "up", value: "171h 55m" },
      { label: "size", value: "1.2 GiB" },
    ],
  },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--sidebar-min)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await endsOnTheRightEdge(canvasElement);
    for (const value of within(canvasElement).getAllByRole("definition")) {
      await expect(value.scrollWidth).toBeLessThanOrEqual(value.clientWidth);
    }
  },
};
