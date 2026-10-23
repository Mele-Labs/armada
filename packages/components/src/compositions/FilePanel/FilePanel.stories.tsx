import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor, within } from "storybook/test";

import { forgetAllKept } from "../../keep";
import { typeIntoEditor } from "./CodeEditor";
import { FilePanel } from "./FilePanel";
// `index.css` imports this after rules, which a browser drops, so the stylesheet that bounds the editor is named here too.
import "./FilePanel.css";

/**
 * Any file in the side panel: a head with its name, its path and two acts that leave Bridge for it, and the body the file fills.
 * Text is edited in place and a picture is only shown.
 *
 * **Every state below is one the file panel's api can produce.** `readPanelFile` answers text or bytes, `savePanelFile`
 * answers saved, moved with what is on disk, or failed, and a read it refuses leaves the head and one line saying why.
 * The panel is a floating sheet, so each story draws it over the canvas.
 */
const meta: Meta<typeof FilePanel> = {
  title: "Compositions/File panel",
  component: FilePanel,
  args: {
    name: "settings.json",
    path: "/Users/user/Library/Application Support/Armada/settings.json",
    type: "text/json",
    dirty: false,
    onClose: fn(),
    onChange: fn(),
    onSave: fn(),
    onOpenInEditor: fn(),
    onShowInFinder: fn(),
  },
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof FilePanel>;

const SETTINGS = `{
  "limits": {
    "dronesAtOnce": 2,
    "checksAtOnce": 4
  },
  "timeouts": {
    "checkSeconds": 900
  },
  "density": "comfortable"
}
`;

const EDITED = SETTINGS.replace('"comfortable"', '"comfortable",\n  "sounds": false');

/** A file as it was read: the language is the name's extension, and the head carries no mark. */
export const Saved: Story = { args: { text: SETTINGS } };

/**
 * **Save appears only while there is an edit, and carries ⌘S.** A press and the chord do the same thing. A save with
 * nothing changed is not offered, which is the half only a run can hold.
 */
export const Unsaved: Story = {
  args: { text: EDITED, dirty: true },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /^Save/ }));
    await expect(args.onSave).toHaveBeenCalledTimes(1);
    await userEvent.keyboard("{Meta>}s{/Meta}");
    await expect(args.onSave).toHaveBeenCalledTimes(2);
  },
};

/**
 * A save that found the file changed on disk wrote nothing. **Both are drawn, and neither is touched until a press says
 * which stands**; ⌘S does not save over a moved file.
 */
export const Moved: Story = {
  args: { text: EDITED, dirty: true, moved: { onDisk: SETTINGS.replace("comfortable", "compact") }, onKeepMine: fn(), onTakeDisk: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Changed on disk")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /^Save/ })).toBeNull();
    await userEvent.keyboard("{Meta>}s{/Meta}");
    await expect(args.onSave).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Keep mine" }));
    await expect(args.onKeepMine).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Take disk" }));
    await expect(args.onTakeDisk).toHaveBeenCalledTimes(1);
  },
};

/** A picture is shown and not edited, so there is no editor and no unsaved mark. */
export const Picture: Story = {
  args: {
    name: "clock-blocks.png",
    path: "/Users/user/Development/armada/docs/notes/clock-blocks.png",
    type: "image/png",
    imageSrc: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAHgAAABQCAIAAABd+SbeAAAAwUlEQVR42u3QMRGAMBAAwfhJlZKhR0Bk0SACCyiIC9zg4Yvni505A7etjy3cfsxw1/2GW88Z7q/fBho0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRp0UWhYOb+gQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQVeFhpXzCxo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRp0VWhYOb+gQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQReF/gComBke7NyJcQAAAABJRU5ErkJggg==",
  },
};

/** A read the panel refused: one line, facts only, and the head still names the file. */
export const Refused: Story = {
  args: { name: "recording.mov", path: "/Users/user/Movies/recording.mov", type: "text/plain", refused: "Not a text file" },
};

/** **A markdown file offers Preview and no other file does**: the rendered view has nothing to render of JSON or a picture. */
export const MarkdownOffersPreview: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/armada/docs/notes.md", type: "text/markdown", text: "# Notes\n\nA line.\n" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Preview" })).toHaveAttribute("aria-pressed", "false");
  },
};

export const JsonOffersNoPreview: Story = {
  args: { text: SETTINGS },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Preview" })).toBeNull();
  },
};

/**
 * **Preview opens the rendered file in a floating panel beside it, and follows the edit.** Pressing it again closes it. Typed
 * text is in the preview as it is typed, with nothing to save first.
 */
export const PreviewBesideTheFile: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/armada/docs/notes.md", type: "text/markdown", text: "# Notes\n\nA line.\n" },
  render: (args) => {
    const [text, setText] = useState(args.text ?? "");
    return <FilePanel {...args} text={text} dirty={text !== args.text} onChange={setText} />;
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    const preview = canvas.getByRole("dialog", { name: "Preview" });
    await expect(within(preview).getByText("Notes")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Preview" })).toHaveAttribute("aria-pressed", "true");
    typeIntoEditor(canvas.getByRole("group", { name: "Editor notes.md" }), "@3  Extra");
    await waitFor(() => expect(within(preview).getByText("A line. Extra")).toBeVisible());
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    await expect(canvas.queryByRole("dialog", { name: "Preview" })).toBeNull();
  },
};

/**
 * **The switch in the preview's head makes every markdown file open with its preview, and Esc then closes both panels.** The
 * choice is kept, so the next file opens the same way.
 */
export const PreviewByDefault: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/armada/docs/notes.md", type: "text/markdown", text: "# Notes\n\nA line.\n" },
  play: async ({ args, canvas, userEvent }) => {
    forgetAllKept();
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    await userEvent.click(canvas.getByRole("switch", { name: "Open previews by default" }));
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(args.onClose).toHaveBeenCalled());
    forgetAllKept();
  },
};

const LONG = Array.from({ length: 60 }, (_, at) => `## Section ${at + 1}\n\nA paragraph for section ${at + 1}, long enough to stand on its own line in the file.\n`).join("\n");

/** The editor's scroller, which lives in the editor's shadow root. */
const scrollerOf = (editor: HTMLElement): HTMLElement => editor.shadowRoot!.querySelector<HTMLElement>(".cm-scroller")!;

/**
 * **The preview and the file are scroll-locked**: moving either moves the other to the same place in the text. Neither answers
 * the other's move with a move of its own, so a scroll comes to rest where it was put.
 */
export const PreviewScrollsWithTheFile: Story = {
  args: { name: "long.md", path: "/Users/user/Development/armada/docs/long.md", type: "text/markdown", text: LONG },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    const preview = canvas.getByRole("region", { name: "Rendered markdown" });
    const editor = scrollerOf(canvas.getByRole("group", { name: "Editor long.md" }));
    // The editor moves, the preview follows.
    editor.scrollTop = editor.scrollHeight / 2;
    await waitFor(() => expect(preview.scrollTop).toBeGreaterThan(preview.scrollHeight / 4));
    // And it stays: neither side has been moved on by the other's answer.
    const settled = [editor.scrollTop, preview.scrollTop];
    await new Promise((done) => setTimeout(done, 400));
    await expect(Math.abs(editor.scrollTop - settled[0]!)).toBeLessThan(2);
    await expect(Math.abs(preview.scrollTop - settled[1]!)).toBeLessThan(2);
    // The preview moves, the editor follows.
    preview.scrollTop = 0;
    await waitFor(() => expect(editor.scrollTop).toBeLessThan(2));
  },
};

/** The colour a token is drawn in, found by its text inside the editor's shadow root. */
const colourOf = (editor: HTMLElement, token: string): string | undefined => {
  const spans = [...(editor.shadowRoot?.querySelectorAll(".cm-line span") ?? [])];
  const found = spans.find((one) => one.textContent === token);
  return found === undefined ? undefined : getComputedStyle(found).color;
};
const plainColour = (editor: HTMLElement): string => getComputedStyle(editor.shadowRoot?.querySelector(".cm-content") as Element).color;

/**
 * **Every common language is drawn in colour, not only the few the editor was first given.** Go, Ruby and SQL are loaded when
 * the file opens, and a fenced block in markdown is drawn in its own language.
 */
export const HighlightsGo: Story = {
  args: { name: "main.go", path: "/Users/user/Development/app/main.go", type: "text/plain", text: "package main\n\nfunc main() {\n}\n" },
  play: async ({ canvas }) => {
    const editor = canvas.getByRole("group", { name: "Editor main.go" });
    await waitFor(() => expect(colourOf(editor, "func")).toBeDefined());
    await expect(colourOf(editor, "func")).not.toBe(plainColour(editor));
  },
};

export const HighlightsRuby: Story = {
  args: { name: "app.rb", path: "/Users/user/Development/app/app.rb", type: "text/plain", text: "def hello\n  puts 1\nend\n" },
  play: async ({ canvas }) => {
    const editor = canvas.getByRole("group", { name: "Editor app.rb" });
    await waitFor(() => expect(colourOf(editor, "def")).toBeDefined());
    await expect(colourOf(editor, "def")).not.toBe(plainColour(editor));
  },
};

export const HighlightsSql: Story = {
  args: { name: "q.sql", path: "/Users/user/Development/app/q.sql", type: "text/plain", text: "SELECT 1 FROM loans;\n" },
  play: async ({ canvas }) => {
    const editor = canvas.getByRole("group", { name: "Editor q.sql" });
    await waitFor(() => expect(colourOf(editor, "SELECT")).toBeDefined());
    await expect(colourOf(editor, "SELECT")).not.toBe(plainColour(editor));
  },
};

export const HighlightsAFenceInMarkdown: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/app/notes.md", type: "text/markdown", text: "# Notes\n\n```go\nfunc main() {}\n```\n" },
  play: async ({ canvas }) => {
    const editor = canvas.getByRole("group", { name: "Editor notes.md" });
    await waitFor(() => expect(colourOf(editor, "func")).toBeDefined());
    await expect(colourOf(editor, "func")).not.toBe(plainColour(editor));
  },
};

/** **Markdown's own marks are coloured too**: a code span, and the `-` that starts a list item, where both were plain text. */
export const HighlightsMarkdownMarks: Story = {
  args: { name: "marks.md", path: "/Users/user/Development/app/marks.md", type: "text/markdown", text: "A `span` here.\n\n- item\n" },
  play: async ({ canvas }) => {
    const editor = canvas.getByRole("group", { name: "Editor marks.md" });
    await waitFor(() => expect(colourOf(editor, "`span`")).toBeDefined());
    await expect(colourOf(editor, "`span`")).not.toBe(plainColour(editor));
    await expect(colourOf(editor, "-")).not.toBe(plainColour(editor));
  },
};
