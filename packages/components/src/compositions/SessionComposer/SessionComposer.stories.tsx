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

/** A turn is running and Send still works: the message goes as the agent's next input. */
export const SendsWhileWorking: Story = {
  args: { working: true },
  play: async ({ canvas, args }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "also check the tests");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "also check the tests", files: [], tags: [] });
  },
};

/** `/` at the start opens the commands, and choosing one writes it into the message. */
export const Slash: Story = {
  play: async ({ canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "/sim");
    await userEvent.click(await canvas.findByRole("option", { name: /\/simplify/ }));
    await expect(field).toHaveTextContent("/simplify");
  },
};

/** Enter sends and clears the box; Shift+Enter breaks the line and sends nothing. */
export const EnterSends: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "first{Shift>}{Enter}{/Shift}second");
    await expect(field).toHaveTextContent("first second");
    await expect(args.onSend).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledTimes(1);
    await expect(args.onSend).toHaveBeenCalledWith({ text: "first\nsecond", files: [], tags: [] });
    await expect(field).toHaveTextContent("");
  },
};

/**
 * `@` opens Jobs, pull requests, branches and the other Sessions, grouped. Choosing one writes it into the line
 * at the caret, as a unit: it is not in the row of what waits, Backspace takes it whole, and a send
 * carries it as `@title` in the text and in `tags`.
 */
export const At: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "why did @retry");
    await expect(await canvas.findByRole("group", { name: "Jobs" })).toBeInTheDocument();
    await userEvent.click(await canvas.findByRole("option", { name: "55 Cap the retry backoff" }));
    const chip = field.querySelector("[data-tag-kind]");
    await expect(chip).toHaveTextContent("55 Cap the retry backoff");
    await expect(within(canvas.getByRole("group", { name: "Attached" })).queryByText("55 Cap the retry backoff")).toBeNull();
    await userEvent.keyboard("stall");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "why did @55 Cap the retry backoff stall", files: [], tags: [{ kind: "job", id: "j55", title: "55 Cap the retry backoff" }] });
  },
};

/**
 * A tag in the line is one unit the caret cannot enter, which is what lets Backspace take it whole
 * (the Backspace itself is pressed in `sessions-inline-tags.test.tsx`, with real keys).
 */
export const TagIsOneUnit: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "see @retry");
    await userEvent.click(await canvas.findByRole("option", { name: "55 Cap the retry backoff" }));
    await expect(field.querySelector("[data-tag-kind]")).toHaveAttribute("contenteditable", "false");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "see @55 Cap the retry backoff", files: [], tags: [{ kind: "job", id: "j55", title: "55 Cap the retry backoff" }] });
  },
};

/** A tag that arrives from outside the box is written in at the end of the line, and the host is told it was taken. */
export const Tagged: Story = {
  args: { tags: [{ kind: "job", id: "j55", title: "55 Cap the retry backoff" }] },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("textbox", { name: "Message" }).querySelector("[data-tag-kind]")).toHaveTextContent("55 Cap the retry backoff");
    await expect(args.onTags).toHaveBeenCalledWith([]);
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ text: "@55 Cap the retry backoff", tags: [expect.objectContaining({ id: "j55" })] }));
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

/** In a narrow box the menus shrink and Send stays on their row. */
export const NarrowBar: Story = {
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 10.5)" }}>
        <Story />
      </div>
    ),
  ],
  args: { models: ["haiku", "sonnet", "opus"], efforts: ["low", "medium", "high"], model: "sonnet", effort: "medium" },
  play: async ({ canvas }) => {
    const send = canvas.getByRole("button", { name: "Send" }).getBoundingClientRect();
    for (const name of ["Permission mode", "Model", "Effort"]) {
      const menu = canvas.getByRole("combobox", { name }).getBoundingClientRect();
      await expect(Math.abs(send.top + send.height / 2 - (menu.top + menu.height / 2))).toBeLessThan(send.height);
    }
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

const MANY = Array.from({ length: 126 }, (_, index) => ({ name: index % 9 === 0 ? `re-${index}` : index % 7 === 0 ? `pre-${index}` : `cmd-${index}`, says: `What command ${index} does, said at some length so the line has to be cut short` }));

/** A terminal Session offers 126 commands: `/re` keeps the ones starting with it, then the ones holding it, one row each in a column that scrolls. */
export const SlashFiltered: Story = {
  args: { commands: MANY },
  play: async ({ canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "/re");
    const rows = within(await canvas.findByRole("listbox", { name: "Skills and commands" })).getAllByRole("option");
    await expect(rows.length).toBeLessThan(MANY.length);
    for (const row of rows) await expect(row.textContent).toContain("re");
    await expect(rows[0]!.textContent).toContain("/re-");
    const shown = rows.filter((row) => row.getBoundingClientRect().height > 0);
    for (let at = 1; at < shown.length; at += 1) {
      await expect(shown[at]!.getBoundingClientRect().top).toBeGreaterThan(shown[at - 1]!.getBoundingClientRect().bottom - 1);
    }
    await userEvent.keyboard("{ArrowDown}");
    await expect(rows[1]).toHaveAttribute("aria-selected", "true");
    const picked = rows[1]!.querySelector(".armada-session-composer__name")!.textContent;
    await userEvent.keyboard("{Enter}");
    await expect(field).toHaveTextContent(picked!);
  },
};

/** The message box grows with what is typed, stops at half the height of the area it sits in, and scrolls past that. */
export const GrowsToHalf: Story = {
  render: (args) => (
    <div style={{ height: "calc(var(--space-12) * 8)", display: "flex", flexDirection: "column", justifyContent: "flex-end" }}>
      <SessionComposer {...args} />
    </div>
  ),
  play: async ({ canvas, canvasElement }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    const panel = canvasElement.querySelector("form")!.parentElement!;
    const rest = field.getBoundingClientRect().height;
    await userEvent.type(field, "one{Shift>}{Enter}{/Shift}two{Shift>}{Enter}{/Shift}three{Shift>}{Enter}{/Shift}four");
    const grown = field.getBoundingClientRect().height;
    await expect(grown).toBeGreaterThan(rest);
    await expect(grown).toBeLessThan(panel.clientHeight / 2);
    await userEvent.type(field, "{Shift>}{Enter}{/Shift}m".repeat(14));
    await expect(field.getBoundingClientRect().height).toBeLessThanOrEqual(panel.clientHeight / 2 + 1);
    await expect(field.scrollHeight).toBeGreaterThan(field.clientHeight);
    await userEvent.clear(field);
    await expect(field.getBoundingClientRect().height).toBeCloseTo(rest, 0);
  },
};
