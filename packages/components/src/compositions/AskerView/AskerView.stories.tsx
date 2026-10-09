import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { AskerView, type NowAsker } from "./AskerView";

/** What is asking, drawn where the canvas would be: its live output, what it changed, and for a Judge the product beside the checks. */
const meta: Meta<typeof AskerView> = {
  title: "Compositions/Asker view",
  component: AskerView,
};
export default meta;

type Story = StoryObj<typeof AskerView>;

const DRONE: NowAsker = {
  name: "Implement Drone",
  of: "drone",
  step: "Implement",
  state: "waiting",
  actions: ["Read crates/store/src/clock.rs", "Edit crates/store/tests/fixtures.rs"],
  tail: ["test writer::stamps_each_batch ... FAILED", "stopped: two clocks reach the fixture"],
  changed: [
    { path: "crates/store/src/clock.rs", change: "added", diff: ["+pub trait Clock {", "+}"] },
    { path: "crates/store/tests/fixtures.rs", change: "changed", asking: true, diff: ["-    store.set_clock(wall());", "+    store.set_clock(pinned());"] },
  ],
};

/** The file it asks about is marked, and a press on a file opens it. */
export const Drone: Story = {
  args: { asker: DRONE, onOpenFile: fn() },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("region", { name: "Implement Drone" })).toBeInTheDocument();
    await expect(canvas.getByLabelText("Output")).toHaveTextContent("two clocks reach the fixture");
    await expect(canvas.getByRole("img", { name: "Asking about this" })).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: /Open Changed, crates\/store\/tests\/fixtures\.rs/ }));
    await expect(args.onOpenFile).toHaveBeenCalledWith("crates/store/tests/fixtures.rs");
  },
};

/** A Judge's own read, then the work product beside the checks it is reading. */
export const Judge: Story = {
  args: {
    asker: {
      name: "Judge on Implement",
      of: "judge",
      step: "Implement",
      state: "running",
      actions: ["Read the diff against the criteria"],
      tail: ["Criterion 3: reading the retry cap"],
      product: { title: "Cap the retry backoff", lines: ["crates/store/src/backoff.rs   added"] },
      checks: [
        { name: "typecheck", state: "passed" },
        { name: "lint", state: "failed", tail: ["backoff.rs:41 clippy::needless_return"] },
      ],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Work product" })).toBeInTheDocument();
    await expect(canvas.getByRole("region", { name: "Checks it is reading" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Failed" })).toBeInTheDocument();
  },
};
