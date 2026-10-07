import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { SessionFrame } from "./SessionFrame";

/** A Session open: a band in the state's hue over its body. The state is the frame's hue and one mark. */
const meta: Meta<typeof SessionFrame> = {
  title: "Compositions/Session frame",
  component: SessionFrame,
  args: { id: "s7", children: <div style={{ height: "calc(var(--space-12) * 4)" }} /> },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 5)", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionFrame>;

/** No slot, no branch, no title yet. */
export const Blank: Story = {
  args: { state: "blank", said: "Blank: no slot, no branch" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Blank: no slot, no branch" })).toBeInTheDocument();
  },
};

/** A turn is running; the mark breathes. */
export const Working: Story = { args: { state: "working", said: "Working", title: "Flaky store test" } };

/** Idle and the person's move. */
export const Waiting: Story = { args: { state: "waiting", said: "Waiting on you", title: "Flaky store test" } };

/** A pull request's Checks are red. */
export const Failing: Story = { args: { state: "failing", said: "Checks failed on #1843", title: "Flaky store test" } };

/** The title is edited where it stands: a press opens it, Enter saves it and Esc keeps the old one. */
export const Renamable: Story = {
  args: { state: "waiting", said: "Waiting on you", title: "Flaky store test", onRename: fn() },
  play: async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Flaky store test, rename" }));
    await userEvent.keyboard("{Escape}");
    await expect(args.onRename).not.toHaveBeenCalled();
    await expect(canvas.getByRole("button", { name: "Flaky store test, rename" })).toBeInTheDocument();

    await userEvent.click(canvas.getByRole("button", { name: "Flaky store test, rename" }));
    const field = canvas.getByRole("textbox", { name: "Session name" });
    await userEvent.clear(field);
    await userEvent.type(field, "Store test, flaky on CI{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("Store test, flaky on CI");
  },
};

/** No title yet: the header offers Rename as an icon, since there is no name to press. */
export const Untitled: Story = {
  args: { state: "waiting", said: "Waiting on you", onRename: fn() },
  play: async ({ canvas, userEvent, args }) => {
    await expect(canvas.getByRole("button", { name: "Rename" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Rename" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Session name" }), "Fresh name{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("Fresh name");
  },
};
