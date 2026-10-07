import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionComposer } from "./SessionComposer";

/**
 * A Session's message box: Dispatch's model and effort pair, `/` for skills
 * and commands, `@` for the other Sessions, pasted or dropped pictures and
 * files, and a sketch shared from the person's own.
 */
const meta: Meta<typeof SessionComposer> = {
  title: "Compositions/Session composer",
  component: SessionComposer,
  args: {
    working: false,
    model: null,
    effort: null,
    models: ["haiku", "sonnet", "opus"],
    efforts: ["low", "medium", "high"],
    onTune: fn(),
    commands: [
      { name: "review", says: "Review the pull request" },
      { name: "simplify", says: "Simplify the changed code" },
    ],
    sessions: [
      { id: "s2", title: "Release notes script" },
      { id: "s3", title: "Store migration spike" },
    ],
    sketches: [
      {
        id: "k1",
        title: "Store clock",
        drawing: {
          boxes: [
            { id: "a", x: 0, y: 0, body: "wall clock" },
            { id: "b", x: 320, y: 0, body: "store tests" },
          ],
          lines: [{ id: "l", from: "a", to: "b" }],
        },
      },
    ],
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

/** Nothing typed and nothing attached: Send is off. */
export const Blank: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
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

/** `@` opens the other Sessions, and choosing one tags it with a chip. */
export const At: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "ask @Rel");
    await userEvent.click(await canvas.findByRole("option", { name: "Release notes script" }));
    await expect(within(canvas.getByRole("group", { name: "Attached" })).getByText("Release notes script")).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ mentions: ["s2"] }));
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

/** A sketch is shared from the person's own. */
export const SharedSketch: Story = {
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Attach sketch" }));
    await userEvent.click(await canvas.findByRole("option", { name: /Store clock/ }));
    await expect(within(canvas.getByRole("group", { name: "Attached" })).getByText("Store clock")).toBeInTheDocument();
  },
};

/** Model and effort are Dispatch's pair, each with Auto. */
export const Tuned: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Model" }), "opus");
    await expect(args.onTune).toHaveBeenCalledWith({ model: "opus", effort: null });
  },
};
