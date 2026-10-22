import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SessionComposer } from "./SessionComposer";

/**
 * A Session's message box, one box over one slim row: the permission mode,
 * Dispatch's model and effort pair, attach and draw as glyphs with tooltips,
 * and what is waiting to go in a strip that scrolls sideways. `/` opens the
 * skills and commands, `@` the other Sessions and the files.
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
      { kind: "session", id: "s4", title: "Cap the retry backoff" },
    ],
    searchFiles: async (query) => ["packages/components/src/compositions/SessionComposer/SessionComposer.tsx", "docs/practices/bridge.md", "README.md"].filter((path) => path.toLowerCase().includes(query.toLowerCase())),
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

/** A command typed whole sends on the first Enter: the list it opened has nothing left to complete. */
export const SlashWholeSends: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "/review");
    await canvas.findByRole("option", { name: /\/review/ });
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "/review", files: [], tags: [] });
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
 * `@` opens the other Sessions, grouped. Choosing one writes it into the line
 * at the caret, as a unit: it is not in the row of what waits, Backspace takes it whole, and a send
 * carries it as `@title` in the text and in `tags`.
 */
export const At: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "why did @retry");
    await expect(await canvas.findByRole("group", { name: "Jobs" })).toBeInTheDocument();
    await userEvent.click(await canvas.findByRole("option", { name: "Cap the retry backoff" }));
    const chip = field.querySelector("[data-tag-kind]");
    await expect(chip).toHaveTextContent("Cap the retry backoff");
    await expect(within(canvas.getByRole("group", { name: "Attached" })).queryByText("Cap the retry backoff")).toBeNull();
    await userEvent.keyboard("stall");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "why did @Cap the retry backoff stall", files: [], tags: [{ kind: "session", id: "s4", title: "Cap the retry backoff" }] });
  },
};

/**
 * A letter after `@` also asks for files, which show under the Sessions. Choosing one writes
 * `@path` into the line as plain text, with no chip.
 */
export const AtFiles: Story = {
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "read @bridge");
    await userEvent.click(await canvas.findByRole("option", { name: /bridge\.md/ }));
    await expect(field.querySelector("[data-tag-kind]")).toBeNull();
    await expect(field).toHaveTextContent("read @docs/practices/bridge.md");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "read @docs/practices/bridge.md", files: [], tags: [] });
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
    await userEvent.click(await canvas.findByRole("option", { name: "Cap the retry backoff" }));
    await expect(field.querySelector("[data-tag-kind]")).toHaveAttribute("contenteditable", "false");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "see @Cap the retry backoff", files: [], tags: [{ kind: "session", id: "s4", title: "Cap the retry backoff" }] });
  },
};

/** A tag that arrives from outside the box is written in at the end of the line, and the host is told it was taken. */
export const Tagged: Story = {
  args: { tags: [{ kind: "session", id: "s4", title: "Cap the retry backoff" }] },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByRole("textbox", { name: "Message" }).querySelector("[data-tag-kind]")).toHaveTextContent("Cap the retry backoff");
    await expect(args.onTags).toHaveBeenCalledWith([]);
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ text: "@Cap the retry backoff", tags: [expect.objectContaining({ id: "s4" })] }));
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

/** No model chosen: the box follows Fleet's default, and says which. */
export const FollowsTheDefault: Story = {
  args: { defaultModel: "opus" },
};

/** Once a turn has resolved the model, the selected option reads its version. */
export const ResolvedModel: Story = {
  args: { model: "opus", resolvedModel: "claude-opus-5-5" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("combobox", { name: "Model" })).toHaveDisplayValue("Opus 5.5");
  },
};

/** The default option reads it too, and still says it is the default. */
export const ResolvedDefault: Story = {
  args: { defaultModel: "opus", resolvedModel: "claude-opus-5-5" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("combobox", { name: "Model" })).toHaveDisplayValue("Opus 5.5 · default");
  },
};

/** A pick of another family keeps its own name: the resolved model is the last turn's, not the next. */
export const PickedAfterResolved: Story = {
  args: { model: "sonnet", resolvedModel: "claude-opus-5-5" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("combobox", { name: "Model" })).toHaveDisplayValue("sonnet");
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

const NUMBERED = {
  open: [
    { tag: { kind: "issue" as const, id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" } },
    { tag: { kind: "pull_request" as const, id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" } },
  ],
  find: async (number: number) =>
    number === 61
      ? { tag: { kind: "issue" as const, id: "61", title: "#61 Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61" }, standing: "Closed" }
      : undefined,
};

/**
 * `#` at the start of a word offers the repository's open issues and pull requests by number, prefix-matched (a key, in `HashTakesAKey`). A
 * chosen one is written in as a unit and sent with its kind and the forge's address.
 */
export const Hash: Story = {
  args: { numbered: NUMBERED },
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "is #22");
    const list = await canvas.findByRole("listbox", { name: "Tag an issue or pull request" });
    await expect(within(list).getAllByRole("option")).toHaveLength(2);
    // The highest number is offered first.
    await userEvent.keyboard("{Enter}");
    await expect(field.querySelector("[data-tag-kind]")).toHaveAttribute("data-tag-url", "https://forge.invalid/pull/2219");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({
      text: "is @#2219 Pin the Bridge store clock",
      files: [],
      tags: [{ kind: "pull_request", id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" }],
    });
  },
};

/** A number that is not among the open ones is looked up once it rests, offered with where it stands, and still tags. */
export const HashLooksANumberUp: Story = {
  args: { numbered: NUMBERED },
  play: async ({ canvas, args }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "#61");
    const closed = await canvas.findByRole("option", { name: /Doctor reports a stale pid/ });
    await expect(closed).toHaveTextContent("Closed");
    await userEvent.click(closed);
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ tags: [expect.objectContaining({ kind: "issue", id: "61", url: "https://forge.invalid/issues/61" })] }));
  },
};

/** A tracker that names its issues by key, as Jira does: the id is the key and the title leads with it. */
const KEYED = {
  open: [
    { tag: { kind: "issue" as const, id: "OPS-7", title: "OPS-7 Pager rota skips the night shift", url: "https://acme.atlassian.net/browse/OPS-7" } },
    { tag: { kind: "issue" as const, id: "ABC-123", title: "ABC-123 Checkout total rounds twice", url: "https://acme.atlassian.net/browse/ABC-123" } },
    { tag: { kind: "issue" as const, id: "OPS-12", title: "OPS-12 Rotate the staging database credentials", url: "https://acme.atlassian.net/browse/OPS-12" } },
    { tag: { kind: "pull_request" as const, id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" } },
  ],
  find: async () => undefined,
};

/**
 * `#` takes a key as well as digits, matched on its reference whatever the case. Each row is named by the key
 * and carries the title without it; digits still offer the numbered ones only.
 */
export const HashTakesAKey: Story = {
  args: { numbered: KEYED },
  play: async ({ canvas, args }) => {
    const field = canvas.getByRole("textbox", { name: "Message" });
    await userEvent.type(field, "#ops");
    const list = await canvas.findByRole("listbox", { name: "Tag an issue or pull request" });
    await expect(within(list).getAllByRole("option")).toHaveLength(2);
    await expect(within(list).getByRole("option", { name: /OPS-7/ })).toHaveTextContent("Pager rota skips the night shift");
    await userEvent.type(field, "-1");
    await expect(within(list).getAllByRole("option")).toHaveLength(1);
    await userEvent.click(within(list).getByRole("option", { name: /OPS-12/ }));
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ tags: [expect.objectContaining({ kind: "issue", id: "OPS-12", url: "https://acme.atlassian.net/browse/OPS-12" })] }));
  },
};

/** Digits offer the numbered ones and leave the keyed alone, as they did before a key could be typed. */
export const HashDigitsLeaveKeysAlone: Story = {
  args: { numbered: KEYED },
  play: async ({ canvas }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "#22");
    const list = await canvas.findByRole("listbox", { name: "Tag an issue or pull request" });
    await expect(within(list).getAllByRole("option")).toHaveLength(1);
  },
};

/** With nothing behind `#`, it is only a character. */
export const HashWithNothingBehindIt: Story = {
  play: async ({ canvas }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Message" }), "see #2214");
    await expect(canvas.queryByRole("listbox")).toBeNull();
  },
};
