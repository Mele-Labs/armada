import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionComposer } from "./SessionComposer";

/**
 * A Session's message box, one box over one slim row: the permission mode,
 * Dispatch's model and effort pair, attach and draw as glyphs with tooltips,
 * and what is waiting to go in a strip that scrolls sideways. `/` opens the
 * skills and commands, `@` the other Sessions.
 */
const meta: Meta<typeof SessionComposer> = {
  title: "Compositions/Session composer",
  component: SessionComposer,
  args: {
    working: false,
    mode: "auto",
    onMode: fn(),
    model: null,
    effort: null,
    models: ["haiku", "sonnet", "opus"],
    efforts: ["low", "medium", "high"],
    onTune: fn(),
    commands: [
      { name: "review", says: "Review the pull request" },
      { name: "simplify", says: "Simplify the changed code" },
    ],
    taggable: [
      { kind: "session", id: "s2", title: "Release notes script" },
      { kind: "session", id: "s3", title: "Store migration spike" },
      { kind: "job", id: "j55", title: "55 Cap the retry backoff" },
      { kind: "pull_request", id: "1843", title: "#1843 Pin the store clock" },
      { kind: "branch", id: "fix/flaky-store", title: "fix/flaky-store" },
    ],
    tags: [],
    onTags: fn(),
    drawn: [],
    onDraw: fn(),
    onRemoveDrawn: fn(),
    onSend: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 10)", padding: "var(--space-8)", background: "var(--bg-raised)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionComposer>;

/** Nothing typed and nothing attached: Send is off. A Session runs in auto. */
export const Blank: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await expect(canvas.getByRole("combobox", { name: "Permission mode" })).toHaveValue("auto");
  },
};

/** `/` at the start opens the commands, and choosing one writes it into the message. */
export const Slash: Story = {
  play: async ({ canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "/sim");
    await userEvent.click(await canvas.findByRole("option", { name: /\/simplify/ }));
    await expect(field).toHaveValue("/simplify ");
  },
};

/** `@` opens Jobs, pull requests, branches and the other Sessions, grouped, and choosing one tags it. */
export const At: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "why did @retry");
    await expect(await canvas.findByRole("group", { name: "Jobs" })).toBeInTheDocument();
    await userEvent.click(await canvas.findByRole("option", { name: "55 Cap the retry backoff" }));
    await expect(args.onTags).toHaveBeenCalledWith([{ kind: "job", id: "j55", title: "55 Cap the retry backoff" }]);
  },
};

/** A tag that waits is a chip, and Send takes it. */
export const Tagged: Story = {
  args: { tags: [{ kind: "job", id: "j55", title: "55 Cap the retry backoff" }] },
  play: async ({ canvas, args }) => {
    await expect(within(canvas.getByRole("group", { name: "Attached" })).getByText("55 Cap the retry backoff")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ tags: [expect.objectContaining({ id: "j55" })] }));
  },
};

/** Below the breakpoint each select is a glyph and a value, and nothing spills out of the box. */
export const Narrow: Story = {
  args: { compact: true },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 7)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas }) => {
    const box = canvas.getByRole("textbox", { name: "Message" }).closest("form")!;
    for (const one of canvas.getAllByRole("combobox")) {
      await expect(one.getBoundingClientRect().right).toBeLessThanOrEqual(box.getBoundingClientRect().right);
    }
  },
};

/**
 * A pasted picture waits as a chip. **The paste is an event carrying a
 * clipboard of its own**, never the machine's, so this story and a person's
 * own copy cannot meet.
 */
export const PastedPicture: Story = {
  play: async ({ canvas }) => {
    const clipboard = new DataTransfer();
    clipboard.items.add(new File(["png"], "Screenshot.png", { type: "image/png" }));
    canvas.getByRole("textbox", { name: "Message" }).dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
    await expect(await within(await canvas.findByRole("group", { name: "Attached" })).findByText("Screenshot.png")).toBeInTheDocument();
  },
};

/** A sketch drawn for the message waits as a chip, and the glyph opens the pad. */
export const DrawnSketch: Story = {
  args: { drawn: [{ id: "k1", title: "pin the clock" }] },
  play: async ({ canvas, args }) => {
    await expect(within(canvas.getByRole("group", { name: "Attached" })).getByText("pin the clock")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Draw sketch" }));
    await expect(args.onDraw).toHaveBeenCalled();
  },
};

/** A session in a terminal holds its permission mode: it is shown and cannot be set. */
export const ModeLocked: Story = {
  args: { modeLocked: true, mode: "plan" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("combobox", { name: "Permission mode" })).toBeDisabled();
    await expect(canvas.getByRole("combobox", { name: "Model" })).toBeEnabled();
  },
};

/** Until a terminal has reported its mode there is none to show. */
export const ModeUnknown: Story = {
  args: { modeLocked: true, modeHidden: true },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("combobox", { name: "Permission mode" })).toBeNull();
  },
};

/** Model and effort are Dispatch's pair, each with Auto, and the mode is a pick of four. */
export const Tuned: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Model" }), "opus");
    await expect(args.onTune).toHaveBeenCalledWith({ model: "opus", effort: null });
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Permission mode" }), "plan");
    await expect(args.onMode).toHaveBeenCalledWith("plan");
  },
};
